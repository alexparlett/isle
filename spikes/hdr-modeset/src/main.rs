//! P22's spike: an HDR10 modeset on the NVIDIA card, and the render targets HDR needs.

use std::{
    collections::HashMap,
    ffi::{CStr, c_char},
    fs::{self, File, OpenOptions},
    io::Write as _,
    os::fd::{AsFd, BorrowedFd},
    path::PathBuf,
    process::ExitCode,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use smithay::{
    backend::{
        allocator::{
            Allocator, Fourcc, Modifier,
            dmabuf::AsDmabuf,
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        },
        drm::DrmDeviceFd,
        egl::{EGLContext, EGLDisplay},
        renderer::{
            Bind, Color32F, ExportMem, Frame, Offscreen, Renderer,
            gles::{GlesRenderer, GlesTexture, ffi},
        },
    },
    reexports::drm::{
        self, ClientCapability, Device as _,
        buffer::{Buffer as _, DrmFourcc, DrmModifier, Handle as BufferHandle, PlanarBuffer},
        control::{
            AtomicCommitFlags, Device as ControlDevice, FbCmd2Flags, Mode, ResourceHandle,
            atomic::AtomicModeReq, connector, crtc, dumbbuffer::DumbBuffer, framebuffer, plane,
            property,
        },
    },
    utils::{DeviceFd, Rectangle, Size, Transform},
};

static STOP: AtomicBool = AtomicBool::new(false);
static LOG: Mutex<Option<File>> = Mutex::new(None);
static START: Mutex<Option<Instant>> = Mutex::new(None);

macro_rules! say {
    ($($arg:tt)*) => {{
        let line = format!($($arg)*);
        let t = START.lock().unwrap().map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
        let line = format!("[{t:7.2}] {line}");
        println!("{line}");
        if let Some(f) = LOG.lock().unwrap().as_mut() {
            let _ = writeln!(f, "{line}");
            let _ = f.sync_data();
        }
    }};
}

extern "C" fn on_signal(_: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}

struct Card(File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl drm::Device for Card {}
impl ControlDevice for Card {}

/// The first `/dev/dri/{prefix}N` whose driver is nvidia.
fn nvidia_node(prefix: &str) -> Option<PathBuf> {
    let mut names: Vec<String> = fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(prefix) && !n.contains('-'))
        .collect();
    names.sort();
    names
        .into_iter()
        .find(|n| {
            fs::read_link(format!("/sys/class/drm/{n}/device/driver"))
                .map(|d| d.to_string_lossy().contains("nvidia"))
                .unwrap_or(false)
        })
        .map(|n| PathBuf::from("/dev/dri").join(n))
}

struct Props(HashMap<String, (property::Handle, u64, property::Info)>);

impl Props {
    fn of<T: ResourceHandle>(card: &Card, handle: T) -> Result<Props, String> {
        let set = card.get_properties(handle).map_err(|e| format!("properties: {e}"))?;
        let mut map = HashMap::new();
        let (handles, values) = set.as_props_and_values();
        for (&h, &v) in handles.iter().zip(values) {
            let info = card.get_property(h).map_err(|e| format!("property: {e}"))?;
            map.insert(info.name().to_string_lossy().into_owned(), (h, v, info));
        }
        Ok(Props(map))
    }

    fn handle(&self, name: &str) -> Result<property::Handle, String> {
        self.0.get(name).map(|p| p.0).ok_or(format!("no `{name}` property"))
    }

    fn value(&self, name: &str) -> Option<u64> {
        self.0.get(name).map(|p| p.1)
    }

    fn entry(&self, name: &str, entry: &str) -> Result<u64, String> {
        let info = &self.0.get(name).ok_or(format!("no `{name}` property"))?.2;
        match info.value_type() {
            property::ValueType::Enum(values) => values
                .values()
                .1
                .iter()
                .find(|e| e.name().to_string_lossy() == entry)
                .map(|e| e.value())
                .ok_or(format!("`{name}` has no `{entry}`")),
            _ => Err(format!("`{name}` is not an enum")),
        }
    }

    fn entries(&self, name: &str) -> Vec<String> {
        match self.0.get(name).map(|p| p.2.value_type()) {
            Some(property::ValueType::Enum(values)) => values
                .values()
                .1
                .iter()
                .map(|e| format!("{}={}", e.value(), e.name().to_string_lossy()))
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// A dumb buffer added with an explicit format, which `add_framebuffer`'s depth and bpp cannot name.
struct Dumb {
    buffer: DumbBuffer,
    format: DrmFourcc,
}

impl PlanarBuffer for Dumb {
    fn size(&self) -> (u32, u32) {
        self.buffer.size()
    }
    fn format(&self) -> DrmFourcc {
        self.format
    }
    fn modifier(&self) -> Option<DrmModifier> {
        None
    }
    fn pitches(&self) -> [u32; 4] {
        [self.buffer.pitch(), 0, 0, 0]
    }
    fn handles(&self) -> [Option<BufferHandle>; 4] {
        [Some(self.buffer.handle()), None, None, None]
    }
    fn offsets(&self) -> [u32; 4] {
        [0; 4]
    }
}

/// The kernel's `struct hdr_output_metadata` with its type-1 infoframe.
#[repr(C)]
#[derive(Clone, Copy)]
struct HdrOutputMetadata {
    metadata_type: u32,
    eotf: u8,
    descriptor: u8,
    primaries: [[u16; 2]; 3],
    white: [u16; 2],
    max_mastering: u16,
    min_mastering: u16,
    max_cll: u16,
    max_fall: u16,
}

const _: () = assert!(size_of::<HdrOutputMetadata>() == 32);

/// What the EDID says about HDR: luminances in cd/m², chromaticities as CIE xy.
#[derive(Debug)]
struct EdidHdr {
    pq: bool,
    max: f64,
    fall: f64,
    min: f64,
    primaries: [[f64; 2]; 3],
    white: [f64; 2],
}

fn edid_hdr(edid: &[u8]) -> Option<EdidHdr> {
    if edid.len() < 128 {
        return None;
    }
    // Base block bytes 25–34: two low bits each in 25 and 26, eight high bits in 27–34.
    let lo = |byte: u8, shift: u8| ((byte >> shift) & 0b11) as u16;
    let xy = |hi: u8, low: u16| ((hi as u16) << 2 | low) as f64 / 1024.0;
    let (a, b) = (edid[25], edid[26]);
    let primaries = [
        [xy(edid[27], lo(a, 6)), xy(edid[28], lo(a, 4))],
        [xy(edid[29], lo(a, 2)), xy(edid[30], lo(a, 0))],
        [xy(edid[31], lo(b, 6)), xy(edid[32], lo(b, 4))],
    ];
    let white = [xy(edid[33], lo(b, 2)), xy(edid[34], lo(b, 0))];
    for block in edid[128..].chunks_exact(128) {
        if block[0] != 0x02 {
            continue;
        }
        let end = (block[2] as usize).min(127);
        let mut i = 4;
        while i < end {
            let tag = block[i] >> 5;
            let len = (block[i] & 0x1f) as usize;
            // Extended tag 6: HDR static metadata.
            if tag == 7 && len >= 3 && block[i + 1] == 6 {
                let p = &block[i + 2..i + 1 + len];
                let max_code = p.get(2).copied();
                let max = max_code.map(|c| 50.0 * 2f64.powf(c as f64 / 32.0)).unwrap_or(0.0);
                let fall = p
                    .get(3)
                    .map(|&c| 50.0 * 2f64.powf(c as f64 / 32.0))
                    .unwrap_or(max);
                let min = p
                    .get(4)
                    .map(|&c| max * (c as f64 / 255.0).powi(2) / 100.0)
                    .unwrap_or(0.0);
                return Some(EdidHdr { pq: p[0] & 0b100 != 0, max, fall, min, primaries, white });
            }
            i += len + 1;
        }
    }
    None
}

fn metadata(hdr: &EdidHdr) -> HdrOutputMetadata {
    let unit = |v: f64| (v * 50000.0).round() as u16;
    HdrOutputMetadata {
        metadata_type: 0,
        eotf: 2,
        descriptor: 0,
        primaries: hdr.primaries.map(|[x, y]| [unit(x), unit(y)]),
        white: [unit(hdr.white[0]), unit(hdr.white[1])],
        max_mastering: hdr.max.round() as u16,
        min_mastering: (hdr.min * 10000.0).round() as u16,
        max_cll: hdr.max.round() as u16,
        max_fall: hdr.fall.round() as u16,
    }
}

fn pq(nits: f64) -> f64 {
    let y = (nits / 10000.0).clamp(0.0, 1.0);
    let (m1, m2) = (2610.0 / 16384.0, 2523.0 / 4096.0 * 128.0);
    let (c1, c2, c3) = (3424.0 / 4096.0, 2413.0 / 4096.0 * 32.0, 2392.0 / 4096.0 * 32.0);
    let p = y.powf(m1);
    ((c1 + c2 * p) / (1.0 + c3 * p)).powf(m2)
}

fn srgb(v: f64) -> f64 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.0031308 { 12.92 * v } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

const BT709_TO_BT2020: [[f64; 3]; 3] = [
    [0.6274, 0.3293, 0.0433],
    [0.0691, 0.9195, 0.0114],
    [0.0164, 0.0880, 0.8956],
];
const BT2020_TO_BT709: [[f64; 3]; 3] = [
    [1.6605, -0.5876, -0.0728],
    [-0.1246, 1.1329, -0.0083],
    [-0.0182, -0.1006, 1.1187],
];

fn mul(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
}

/// Grey bars in cd/m²; the top two thirds of the pattern.
const BARS: [f64; 12] = [0.0, 0.5, 2.0, 10.0, 50.0, 100.0, 203.0, 250.0, 302.0, 400.0, 1000.0, 10000.0];
const SDR_WHITE: f64 = 203.0;

#[derive(Clone, Copy, PartialEq)]
enum Signal {
    Sdr,
    Pq,
}

#[derive(Clone, Copy, PartialEq)]
enum Depth {
    Ten,
    Eight,
}

impl Depth {
    fn format(self) -> DrmFourcc {
        match self {
            Depth::Ten => DrmFourcc::Xbgr2101010,
            Depth::Eight => DrmFourcc::Xrgb8888,
        }
    }

    fn pack(self, [r, g, b]: [f64; 3]) -> u32 {
        match self {
            Depth::Ten => {
                let q = |v: f64| (v.clamp(0.0, 1.0) * 1023.0).round() as u32;
                q(r) | q(g) << 10 | q(b) << 20
            }
            Depth::Eight => {
                let q = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
                q(b) | q(g) << 8 | q(r) << 16
            }
        }
    }
}

/// Linear light in cd/m² for pixel (x, y), in BT.709 primaries for SDR and BT.2020 for PQ.
fn scene(x: u32, y: u32, w: u32, h: u32, signal: Signal) -> [f64; 3] {
    if y < h * 2 / 3 {
        let n = BARS[(x as usize * BARS.len()) / w as usize];
        return [n, n, n];
    }
    let patch = (x * 6 / w) as usize;
    let mut c = [0.0; 3];
    c[patch % 3] = SDR_WHITE;
    // Patches 0–2 are BT.709's primaries, 3–5 BT.2020's.
    match (patch < 3, signal) {
        (true, Signal::Pq) => mul(&BT709_TO_BT2020, c),
        (false, Signal::Sdr) => mul(&BT2020_TO_BT709, c),
        _ => c,
    }
}

fn paint(card: &Card, dumb: &mut Dumb, depth: Depth, signal: Signal) -> Result<(), String> {
    let (w, h) = dumb.buffer.size();
    let pitch = dumb.buffer.pitch() as usize;
    let mut map = card
        .map_dumb_buffer(&mut dumb.buffer)
        .map_err(|e| format!("map: {e}"))?;
    for y in 0..h {
        let row = &mut map.as_mut()[y as usize * pitch..];
        for x in 0..w {
            let c = scene(x, y, w, h, signal);
            let e = match signal {
                Signal::Pq => c.map(pq),
                Signal::Sdr => c.map(|v| srgb(v / SDR_WHITE)),
            };
            let px = depth.pack(e).to_le_bytes();
            row[x as usize * 4..x as usize * 4 + 4].copy_from_slice(&px);
        }
    }
    Ok(())
}

struct Output {
    conn: connector::Handle,
    crtc: crtc::Handle,
    primary: plane::Handle,
    others: Vec<plane::Handle>,
    mode: Mode,
    conn_props: Props,
    crtc_props: Props,
    plane_props: HashMap<plane::Handle, Props>,
}

fn find_output(card: &Card, want: Option<(u16, u16, u32)>) -> Result<Output, String> {
    let res = card.resource_handles().map_err(|e| format!("resources: {e}"))?;
    let (conn, info) = res
        .connectors()
        .iter()
        .filter_map(|&c| card.get_connector(c, false).ok().map(|i| (c, i)))
        .find(|(_, i)| i.state() == connector::State::Connected)
        .ok_or("no connected connector")?;
    let crtc = info
        .current_encoder()
        .and_then(|e| card.get_encoder(e).ok())
        .and_then(|e| e.crtc())
        .or_else(|| {
            info.encoders()
                .iter()
                .filter_map(|&e| card.get_encoder(e).ok())
                .flat_map(|e| res.filter_crtcs(e.possible_crtcs()))
                .next()
        })
        .ok_or("no CRTC for the connector")?;
    let modes = info.modes();
    let mode = match want {
        Some((w, h, r)) => modes.iter().find(|m| m.size() == (w, h) && m.vrefresh() == r),
        None => modes.iter().max_by_key(|m| {
            let (w, h) = m.size();
            (w as u32 * h as u32, m.vrefresh())
        }),
    }
    .copied()
    .ok_or("no such mode")?;
    let mut primary = None;
    let mut others = Vec::new();
    let mut plane_props = HashMap::new();
    for p in card.plane_handles().map_err(|e| format!("planes: {e}"))? {
        let pinfo = card.get_plane(p).map_err(|e| format!("plane: {e}"))?;
        if !res.filter_crtcs(pinfo.possible_crtcs()).contains(&crtc) {
            continue;
        }
        let props = Props::of(card, p)?;
        if props.value("type") == Some(1) && primary.is_none() {
            primary = Some(p);
        } else {
            others.push(p);
        }
        plane_props.insert(p, props);
    }
    Ok(Output {
        conn,
        crtc,
        primary: primary.ok_or("no primary plane")?,
        others,
        mode,
        conn_props: Props::of(card, conn)?,
        crtc_props: Props::of(card, crtc)?,
        plane_props,
    })
}

fn blob_id(v: property::Value<'_>) -> u64 {
    match v {
        property::Value::Blob(id) => id,
        _ => 0,
    }
}

struct Commit<'a> {
    fb: framebuffer::Handle,
    colorspace: u64,
    metadata: u64,
    active: bool,
    label: &'a str,
}

fn commit(card: &Card, out: &Output, mode_blob: u64, c: &Commit, test: bool) -> Result<Duration, String> {
    let cp = &out.conn_props;
    let rp = &out.crtc_props;
    let pp = &out.plane_props[&out.primary];
    let (w, h) = out.mode.size();
    let (w, h) = (w as u64, h as u64);
    let crtc_id: u32 = out.crtc.into();
    let fb_id: u32 = c.fb.into();
    let mut req = AtomicModeReq::new();
    req.add_raw_property(out.conn.into(), cp.handle("CRTC_ID")?, crtc_id as u64);
    req.add_raw_property(out.conn.into(), cp.handle("Colorspace")?, c.colorspace);
    req.add_raw_property(out.conn.into(), cp.handle("HDR_OUTPUT_METADATA")?, c.metadata);
    req.add_raw_property(out.crtc.into(), rp.handle("MODE_ID")?, mode_blob);
    req.add_raw_property(out.crtc.into(), rp.handle("ACTIVE")?, c.active as u64);
    for (name, value) in [
        ("FB_ID", fb_id as u64),
        ("CRTC_ID", crtc_id as u64),
        ("SRC_X", 0),
        ("SRC_Y", 0),
        ("SRC_W", w << 16),
        ("SRC_H", h << 16),
        ("CRTC_X", 0),
        ("CRTC_Y", 0),
        ("CRTC_W", w),
        ("CRTC_H", h),
    ] {
        req.add_raw_property(out.primary.into(), pp.handle(name)?, value);
    }
    // Overlays and the cursor left by the session that held the card.
    for p in &out.others {
        let props = &out.plane_props[p];
        req.add_raw_property((*p).into(), props.handle("FB_ID")?, 0);
        req.add_raw_property((*p).into(), props.handle("CRTC_ID")?, 0);
    }
    let mut flags = AtomicCommitFlags::ALLOW_MODESET;
    if test {
        flags |= AtomicCommitFlags::TEST_ONLY;
    }
    let started = Instant::now();
    card.atomic_commit(flags, req)
        .map_err(|e| format!("{}{}: {e}", c.label, if test { " (test)" } else { "" }))?;
    Ok(started.elapsed())
}

/// Waits `secs`, false if interrupted.
fn hold(secs: u64) -> bool {
    let until = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < until {
        if STOP.load(Ordering::SeqCst) {
            return false;
        }
        thread::sleep(Duration::from_millis(100));
    }
    true
}

struct Options {
    cycles: u32,
    hold: u64,
    dpms: bool,
    depth: Depth,
    mode: Option<(u16, u16, u32)>,
    card: Option<PathBuf>,
}

fn parse_mode(s: &str) -> Option<(u16, u16, u32)> {
    let (size, rate) = s.split_once('@')?;
    let (w, h) = size.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?, rate.parse().ok()?))
}

fn run(opts: Options) -> Result<(), String> {
    let path = opts.card.clone().or_else(|| nvidia_node("card")).ok_or("no NVIDIA card")?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let card = Card(file);
    card.set_client_capability(ClientCapability::UniversalPlanes, true)
        .map_err(|e| format!("universal planes: {e}"))?;
    card.set_client_capability(ClientCapability::Atomic, true)
        .map_err(|e| format!("atomic: {e}"))?;
    if let Err(e) = card.acquire_master_lock() {
        say!("master: {e} (continuing; the first commit says whether the card is free)");
    }

    let out = find_output(&card, opts.mode)?;
    let (w, h) = out.mode.size();
    say!(
        "{}: connector {:?}, CRTC {:?}, primary {:?}, {} other planes, mode {}x{}@{}",
        path.display(),
        out.conn,
        out.crtc,
        out.primary,
        out.others.len(),
        w,
        h,
        out.mode.vrefresh()
    );
    say!("Colorspace: {:?}", out.conn_props.entries("Colorspace"));
    say!(
        "now: Colorspace={:?} HDR_OUTPUT_METADATA={:?} max bpc={:?}",
        out.conn_props.value("Colorspace"),
        out.conn_props.value("HDR_OUTPUT_METADATA"),
        out.conn_props.value("max bpc")
    );

    let edid_blob = out.conn_props.value("EDID").ok_or("no EDID")?;
    let edid = card.get_property_blob(edid_blob).map_err(|e| format!("EDID: {e}"))?;
    let hdr = edid_hdr(&edid).ok_or("the EDID has no HDR static metadata")?;
    say!("EDID: {hdr:?}");
    if !hdr.pq {
        return Err("the EDID does not list ST2084".into());
    }
    let meta = metadata(&hdr);
    say!(
        "metadata: primaries {:?} white {:?} max {} min {} (x0.0001) maxCLL {} maxFALL {}",
        meta.primaries,
        meta.white,
        meta.max_mastering,
        meta.min_mastering,
        meta.max_cll,
        meta.max_fall
    );

    let default = out.conn_props.entry("Colorspace", "Default")?;
    let bt2020 = out.conn_props.entry("Colorspace", "BT2020_RGB")?;

    let format = opts.depth.format();
    let mut fbs = Vec::new();
    for signal in [Signal::Sdr, Signal::Pq] {
        let buffer = card
            .create_dumb_buffer((w as u32, h as u32), format, 32)
            .map_err(|e| format!("dumb buffer: {e}"))?;
        let mut dumb = Dumb { buffer, format };
        paint(&card, &mut dumb, opts.depth, signal)?;
        let fb = card
            .add_planar_framebuffer(&dumb, FbCmd2Flags::empty())
            .map_err(|e| format!("framebuffer {format:?}: {e}"))?;
        fbs.push((dumb, fb));
    }
    let (sdr_fb, hdr_fb) = (fbs[0].1, fbs[1].1);
    let mode_blob = blob_id(card.create_property_blob(&out.mode).map_err(|e| format!("mode blob: {e}"))?);
    let meta_blob = blob_id(card.create_property_blob(&meta).map_err(|e| format!("metadata blob: {e}"))?);

    let sdr = Commit { fb: sdr_fb, colorspace: default, metadata: 0, active: true, label: "SDR" };
    let hdr_on = Commit { fb: hdr_fb, colorspace: bt2020, metadata: meta_blob, active: true, label: "HDR" };
    let dark = Commit { active: false, label: "HDR, screen off", ..hdr_on };

    let result = (|| -> Result<(), String> {
        commit(&card, &out, mode_blob, &hdr_on, true)?;
        say!("HDR passes TEST_ONLY at {format:?}");

        say!("SDR baseline: note the OSD");
        let t = commit(&card, &out, mode_blob, &sdr, false)?;
        say!("SDR committed in {t:?}");
        if !hold(5) {
            return Ok(());
        }
        for i in 1..=opts.cycles {
            say!("HDR on, cycle {i}: the OSD should say HDR; the bars from 302 up should look alike");
            let t = commit(&card, &out, mode_blob, &hdr_on, false)?;
            say!("HDR committed in {t:?}");
            if !hold(opts.hold) {
                return Ok(());
            }
            let t = commit(&card, &out, mode_blob, &sdr, false)?;
            say!("SDR, cycle {i}, committed in {t:?}");
            if !hold(5) {
                return Ok(());
            }
        }
        if opts.dpms {
            let t = commit(&card, &out, mode_blob, &hdr_on, false)?;
            say!("HDR for the screen-off test, committed in {t:?}");
            if !hold(5) {
                return Ok(());
            }
            let t = commit(&card, &out, mode_blob, &dark, false)?;
            say!("screen off in {t:?}");
            if !hold(5) {
                return Ok(());
            }
            let t = commit(&card, &out, mode_blob, &hdr_on, false)?;
            say!("screen on in HDR in {t:?}: is it lit, and in HDR?");
            if !hold(opts.hold) {
                return Ok(());
            }
        }
        Ok(())
    })();

    if let Err(e) = &result {
        say!("failed: {e}");
    }
    if STOP.load(Ordering::SeqCst) {
        say!("interrupted");
    }
    match commit(&card, &out, mode_blob, &sdr, false) {
        Ok(t) => say!("left in SDR ({t:?})"),
        Err(e) => say!("could not return to SDR: {e}"),
    }
    hold(2);
    let _ = card.destroy_property_blob(meta_blob);
    let _ = card.destroy_property_blob(mode_blob);
    for (dumb, fb) in fbs {
        let _ = card.destroy_framebuffer(fb);
        let _ = card.destroy_dumb_buffer(dumb.buffer);
    }
    result
}

fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = ((bits >> 10) & 0x1f) as i32;
    let frac = (bits & 0x3ff) as f32;
    sign * match exp {
        0 => frac / 1024.0 * 2f32.powi(-14),
        31 => f32::INFINITY,
        _ => (1.0 + frac / 1024.0) * 2f32.powi(exp - 15),
    }
}

fn gl() -> Result<(), String> {
    let path = nvidia_node("renderD").ok_or("no NVIDIA render node")?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let fd = DrmDeviceFd::new(DeviceFd::from(std::os::fd::OwnedFd::from(file)));
    let gbm = GbmDevice::new(fd).map_err(|e| format!("gbm: {e}"))?;
    let display = unsafe { EGLDisplay::new(gbm.clone()) }.map_err(|e| format!("EGL display: {e}"))?;
    let context = EGLContext::new(&display).map_err(|e| format!("EGL context: {e}"))?;
    let mut renderer = unsafe { GlesRenderer::new(context) }.map_err(|e| format!("GLES: {e}"))?;
    say!("{}", path.display());

    let extensions = renderer
        .with_context(|gl| unsafe {
            CStr::from_ptr(gl.GetString(ffi::EXTENSIONS) as *const c_char)
                .to_string_lossy()
                .into_owned()
        })
        .map_err(|e| format!("extensions: {e}"))?;
    for name in ["GL_EXT_color_buffer_half_float", "GL_EXT_color_buffer_float", "GL_EXT_texture_norm16"] {
        say!("{name}: {}", extensions.split(' ').any(|e| e == name));
    }

    match fp16(&mut renderer) {
        Ok(values) => say!("FP16 texture target: clear and solid read back as {values:?}"),
        Err(e) => say!("FP16 texture target: {e}"),
    }
    for format in [Fourcc::Argb8888, Fourcc::Abgr2101010, Fourcc::Xbgr2101010, Fourcc::Abgr16161616f] {
        match scanout_target(&mut renderer, &gbm, format) {
            Ok(px) => say!("{format:?} GBM scanout buffer as a render target: ok, first pixel {px}"),
            Err(e) => say!("{format:?} GBM scanout buffer as a render target: {e}"),
        }
    }
    Ok(())
}

/// Renders values outside 0..1 into an FP16 texture and reads them back.
fn fp16(renderer: &mut GlesRenderer) -> Result<[[f32; 4]; 2], String> {
    let size = Size::from((64, 64));
    let mut texture: GlesTexture =
        Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr16161616f, size)
            .map_err(|e| format!("create: {e}"))?;
    let mut framebuffer = renderer.bind(&mut texture).map_err(|e| format!("bind: {e}"))?;
    {
        let mut frame = renderer
            .render(&mut framebuffer, (64, 64).into(), Transform::Normal)
            .map_err(|e| format!("render: {e}"))?;
        frame
            .clear(Color32F::new(2.0, 0.5, -0.25, 1.0), &[Rectangle::from_size((32, 64).into())])
            .map_err(|e| format!("clear: {e}"))?;
        let right = Rectangle::new((32, 0).into(), (32, 64).into());
        frame
            .draw_solid(right, &[Rectangle::from_size((32, 64).into())], Color32F::new(4.0, 1.0, -1.0, 1.0))
            .map_err(|e| format!("draw: {e}"))?;
        let _ = frame.finish().map_err(|e| format!("finish: {e}"))?.wait();
    }
    let mapping = renderer
        .copy_framebuffer(&framebuffer, Rectangle::from_size((64, 64).into()), Fourcc::Abgr16161616f)
        .map_err(|e| format!("read: {e}"))?;
    let bytes = renderer.map_texture(&mapping).map_err(|e| format!("map: {e}"))?;
    let pixel = |x: usize| {
        let at = x * 8;
        [0, 1, 2, 3].map(|c| half(u16::from_le_bytes([bytes[at + c * 2], bytes[at + c * 2 + 1]])))
    };
    Ok([pixel(4), pixel(40)])
}

fn scanout_target(
    renderer: &mut GlesRenderer,
    gbm: &GbmDevice<DrmDeviceFd>,
    format: Fourcc,
) -> Result<String, String> {
    let modifiers: Vec<Modifier> = renderer
        .egl_context()
        .dmabuf_render_formats()
        .iter()
        .filter(|f| f.code == format)
        .map(|f| f.modifier)
        .collect();
    say!("{format:?}: render modifiers {modifiers:?}");
    let mut allocator = GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT);
    let buffer = allocator
        .create_buffer(256, 256, format, &modifiers)
        .map_err(|e| format!("allocate: {e}"))?;
    let mut dmabuf = buffer.export().map_err(|e| format!("export: {e}"))?;
    let mut framebuffer = renderer.bind(&mut dmabuf).map_err(|e| format!("bind: {e}"))?;
    {
        let mut frame = renderer
            .render(&mut framebuffer, (256, 256).into(), Transform::Normal)
            .map_err(|e| format!("render: {e}"))?;
        frame
            .clear(Color32F::new(0.5, 0.25, 1.0, 1.0), &[Rectangle::from_size((256, 256).into())])
            .map_err(|e| format!("clear: {e}"))?;
        let _ = frame.finish().map_err(|e| format!("finish: {e}"))?.wait();
    }
    let mapping = renderer
        .copy_framebuffer(&framebuffer, Rectangle::from_size((4, 4).into()), format)
        .map_err(|e| format!("read: {e}"))?;
    let bytes = renderer.map_texture(&mapping).map_err(|e| format!("map: {e}"))?;
    Ok(bytes.iter().take(8).map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "))
}

fn main() -> ExitCode {
    *START.lock().unwrap() = Some(Instant::now());
    if let Ok(f) = OpenOptions::new().create(true).append(true).open("hdr-modeset.log") {
        *LOG.lock().unwrap() = Some(f);
    }
    unsafe {
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut opts = Options { cycles: 3, hold: 10, dpms: false, depth: Depth::Ten, mode: None, card: None };
    let mut command = "run";
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "gl" | "run" => command = if arg == "gl" { "gl" } else { "run" },
            "--cycles" => opts.cycles = it.next().and_then(|v| v.parse().ok()).unwrap_or(3),
            "--hold" => opts.hold = it.next().and_then(|v| v.parse().ok()).unwrap_or(10),
            "--dpms" => opts.dpms = true,
            "--eight" => opts.depth = Depth::Eight,
            "--mode" => opts.mode = it.next().and_then(|v| parse_mode(v)),
            "--card" => opts.card = it.next().map(PathBuf::from),
            _ => {
                eprintln!(
                    "usage: hdr-modeset [run] [--cycles N] [--hold SECS] [--dpms] [--eight] [--mode WxH@HZ] [--card PATH]\n       hdr-modeset gl"
                );
                return ExitCode::from(2);
            }
        }
    }
    say!("hdr-modeset {command} {}", args.join(" "));
    let result = if command == "gl" { gl() } else { run(opts) };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            say!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
