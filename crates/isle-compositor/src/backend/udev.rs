//! Isle on the hardware: a logind seat through libseat, DRM/KMS outputs, libinput.
//! One GPU renders and its connectors are driven; another GPU's are not yet (P14).

use std::{collections::HashMap, path::Path, time::Duration};

use smithay::{
    backend::{
        allocator::{
            Fourcc,
            dmabuf::Dmabuf,
            format::FormatSet,
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        },
        drm::{
            DrmDevice, DrmDeviceFd, DrmEvent, DrmNode, NodeType,
            compositor::FrameFlags,
            exporter::gbm::GbmFramebufferExporter,
            output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements},
        },
        egl::{EGLContext, EGLDisplay},
        input::InputEvent,
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{ImportDma, ImportMemWl, gles::GlesRenderer},
        session::{Event as SessionEvent, Session, libseat::LibSeatSession},
        udev::{UdevBackend, UdevEvent, all_gpus, primary_gpu},
    },
    input::pointer::CursorImageStatus,
    output::{Mode, Output, PhysicalProperties, Scale},
    reexports::{
        calloop::{
            RegistrationToken,
            timer::{TimeoutAction, Timer},
        },
        drm::{
            Device as _,
            control::{ModeTypeFlags, connector, crtc},
        },
        input::{self as libinput, DeviceCapability, Libinput},
        rustix::fs::OFlags,
        wayland_server::backend::GlobalId,
    },
    utils::{DeviceFd, Transform},
    wayland::dmabuf::DmabufFeedbackBuilder,
};
use smithay_drm_extras::{
    display_info,
    drm_scanner::{DrmScanEvent, DrmScanner},
};
use tracing::{error, info, warn};

use super::{Backend, CLEAR, Rendered};
use crate::{
    cursor::Cursors,
    render::IsleElement,
    state::{Isle, State},
};

const COLOUR_FORMATS: &[Fourcc] = &[Fourcc::Argb8888, Fourcc::Xrgb8888];
/// How often connectors are probed while no output is lit (`compositor.md`, Outputs 5).
const REPROBE: Duration = Duration::from_secs(3);

type Manager = DrmOutputManager<
    GbmAllocator<DrmDeviceFd>,
    GbmFramebufferExporter<DrmDeviceFd>,
    (),
    DrmDeviceFd,
>;
type Surface =
    DrmOutput<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

/// Which device and CRTC an output is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutputId {
    device: DrmNode,
    crtc: crtc::Handle,
}

struct Lit {
    output: Output,
    global: GlobalId,
    surface: Surface,
}

struct Device {
    manager: Manager,
    scanner: DrmScanner,
    lit: HashMap<crtc::Handle, Lit>,
    nvidia: bool,
    _token: RegistrationToken,
}

pub struct Udev {
    pub session: LibSeatSession,
    libinput: Libinput,
    primary: DrmNode,
    render_node: DrmNode,
    renderer: Option<GlesRenderer>,
    devices: HashMap<DrmNode, Device>,
    cursors: Cursors,
    pointers: Vec<libinput::Device>,
    reprobe: Option<RegistrationToken>,
    /// The render GPU, until the session is first active and it can be opened.
    waiting: Option<(DrmNode, std::path::PathBuf)>,
}

impl Udev {
    /// Opens the seat and picks the render GPU: `render-device`, else the boot GPU.
    pub fn new(
        render_device: Option<&str>,
    ) -> Result<
        (
            Self,
            smithay::backend::session::libseat::LibSeatSessionNotifier,
        ),
        Box<dyn std::error::Error>,
    > {
        let (session, notifier) = LibSeatSession::new()?;
        let seat = session.seat();
        let path = match render_device {
            Some(path) => Some(path.into()),
            None => primary_gpu(&seat)?.or_else(|| all_gpus(&seat).ok()?.into_iter().next()),
        }
        .ok_or("no GPU")?;
        let node = DrmNode::from_path(&path)?;
        let primary = node
            .node_with_type(NodeType::Primary)
            .and_then(Result::ok)
            .unwrap_or(node);
        let render_node = node
            .node_with_type(NodeType::Render)
            .and_then(Result::ok)
            .unwrap_or(node);
        info!(?path, %primary, %render_node, "render GPU");
        let libinput = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
            session.clone().into(),
        );
        Ok((
            Self {
                session,
                libinput,
                primary,
                render_node,
                renderer: None,
                devices: HashMap::new(),
                cursors: Cursors::load(),
                pointers: Vec::new(),
                reprobe: None,
                waiting: None,
            },
            notifier,
        ))
    }

    pub fn seat_name(&self) -> String {
        self.session.seat()
    }

    pub fn renderer(&mut self) -> &mut GlesRenderer {
        self.renderer.as_mut().expect("the render GPU is open")
    }

    pub fn renderer_opt(&mut self) -> Option<&mut GlesRenderer> {
        self.renderer.as_mut()
    }

    pub fn import_dmabuf(&mut self, dmabuf: &Dmabuf) -> bool {
        self.renderer
            .as_mut()
            .is_some_and(|r| r.import_dmabuf(dmabuf, None).is_ok())
    }

    pub fn render(&mut self, isle: &mut Isle, output: &Output) -> Result<Rendered, String> {
        let id = *output
            .user_data()
            .get::<OutputId>()
            .ok_or("not a DRM output")?;
        let renderer = self.renderer.as_mut().ok_or("no renderer")?;
        let lit = self
            .devices
            .get_mut(&id.device)
            .and_then(|d| d.lit.get_mut(&id.crtc))
            .ok_or("the output is gone")?;
        let scale = output.current_scale().integer_scale().max(1) as u32;
        let cursor = match &isle.cursor_status {
            CursorImageStatus::Named(icon) => self.cursors.get(*icon, scale),
            _ => None,
        };
        let mut elements = Vec::new();
        isle.render_elements(renderer, output, cursor, &mut |e| elements.push(e));
        let result = lit
            .surface
            .render_frame(renderer, &elements, CLEAR, FrameFlags::DEFAULT)
            .map_err(|e| format!("{e:?}"))?;
        let damaged = !result.is_empty;
        let states = result.states;
        if damaged {
            lit.surface.queue_frame(()).map_err(|e| format!("{e:?}"))?;
        }
        Ok(Rendered { damaged, states })
    }
}

/// Opens the render GPU, lights its connectors and hooks udev, the session and libinput
/// into the loop.
pub fn setup(
    state: &mut State,
    notifier: smithay::backend::session::libseat::LibSeatSessionNotifier,
) -> Result<(), Box<dyn std::error::Error>> {
    let Backend::Udev(udev) = &mut state.backend else {
        unreachable!("setup for the udev backend");
    };
    let seat = udev.session.seat();
    let udev_backend = UdevBackend::new(&seat)?;
    let devices: Vec<(DrmNode, std::path::PathBuf)> = udev_backend
        .device_list()
        .filter_map(|(id, path)| Some((DrmNode::from_dev_id(id).ok()?, path.to_owned())))
        .collect();
    let primary = udev.primary;
    for (node, _) in devices.iter().filter(|(node, _)| *node != primary) {
        info!(%node, "not driving this GPU's outputs; one GPU renders for now");
    }
    udev.waiting = devices.into_iter().find(|(node, _)| *node == primary);
    // A device opened before the session is active cannot become DRM master.
    if udev.session.is_active() {
        state.open_render_gpu()?;
    } else {
        info!("waiting for the session to become active");
    }

    let handle = state.isle.loop_handle.clone();
    handle
        .insert_source(udev_backend, move |event, _, state| {
            let _beat = state.isle.heartbeat.beat("udev");
            match event {
                UdevEvent::Added { device_id, path } => {
                    if let Ok(node) = DrmNode::from_dev_id(device_id)
                        && node == primary
                        && let Err(err) = state.device_added(node, &path)
                    {
                        error!(%node, "could not open the GPU: {err}");
                    }
                }
                UdevEvent::Changed { device_id } => {
                    if let Ok(node) = DrmNode::from_dev_id(device_id) {
                        state.device_changed(node);
                    }
                }
                UdevEvent::Removed { device_id } => {
                    if let Ok(node) = DrmNode::from_dev_id(device_id) {
                        state.device_removed(node);
                    }
                }
            }
        })
        .map_err(|e| e.error)?;

    handle
        .insert_source(notifier, |event, _, state| {
            let _beat = state.isle.heartbeat.beat("session");
            state.session_event(event);
        })
        .map_err(|e| e.error)?;

    let Backend::Udev(udev) = &mut state.backend else {
        unreachable!();
    };
    udev.libinput
        .udev_assign_seat(&seat)
        .map_err(|()| "libinput could not take the seat")?;
    let input = LibinputInputBackend::new(udev.libinput.clone());
    handle
        .insert_source(input, |event, _, state| {
            let _beat = state.isle.heartbeat.beat("libinput");
            state.libinput_event(event);
        })
        .map_err(|e| e.error)?;
    Ok(())
}

impl State {
    /// Opens the render GPU, then serves the formats it can import.
    fn open_render_gpu(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some((node, path)) = self.udev().waiting.take() else {
            return Ok(());
        };
        self.device_added(node, &path)?;
        let udev = self.udev();
        let render_node = udev.render_node;
        let renderer = udev
            .renderer
            .as_mut()
            .ok_or("the render GPU did not open")?;
        let shm = renderer.shm_formats();
        let formats = renderer.dmabuf_formats();
        self.isle.shm_state.update_formats(shm);
        let feedback = DmabufFeedbackBuilder::new(render_node.dev_id(), formats).build()?;
        self.isle
            .dmabuf_state
            .create_global_with_default_feedback::<State>(&self.isle.display_handle, &feedback);
        Ok(())
    }

    fn udev(&mut self) -> &mut Udev {
        match &mut self.backend {
            Backend::Udev(udev) => udev,
            _ => unreachable!("a udev call on another backend"),
        }
    }

    fn device_added(
        &mut self,
        node: DrmNode,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let udev = self.udev();
        let fd = udev.session.open(
            path,
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
        )?;
        let fd = DrmDeviceFd::new(DeviceFd::from(fd));
        let (drm, drm_events) = DrmDevice::new(fd.clone(), true)?;
        let gbm = GbmDevice::new(fd)?;
        if udev.renderer.is_none() {
            let display = unsafe { EGLDisplay::new(gbm.clone())? };
            let context = EGLContext::new(&display)?;
            udev.renderer = Some(unsafe { GlesRenderer::new(context)? });
        }
        let nvidia = drm
            .get_driver()
            .map(|d| d.name().to_string_lossy().to_lowercase().contains("nvidia"))
            .unwrap_or(false);
        info!(%node, nvidia, "GPU opened");

        let token = self
            .isle
            .loop_handle
            .insert_source(drm_events, move |event, _, state| {
                let _beat = state.isle.heartbeat.beat("drm");
                match event {
                    DrmEvent::VBlank(crtc) => state.drm_vblank(node, crtc),
                    DrmEvent::Error(err) => error!(%node, "DRM: {err}"),
                }
            })
            .map_err(|e| e.error)?;

        let udev = self.udev();
        let renderer = udev.renderer.as_mut().unwrap();
        let render_formats: FormatSet = renderer
            .egl_context()
            .dmabuf_render_formats()
            .iter()
            .copied()
            .collect();
        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );
        let exporter = GbmFramebufferExporter::new(gbm.clone(), Some(udev.render_node).into());
        let manager = DrmOutputManager::new(
            drm,
            allocator,
            exporter,
            Some(gbm),
            COLOUR_FORMATS.iter().copied(),
            render_formats,
        );
        udev.devices.insert(
            node,
            Device {
                manager,
                scanner: DrmScanner::new(),
                lit: HashMap::new(),
                nvidia,
                _token: token,
            },
        );
        self.device_changed(node);
        Ok(())
    }

    /// Probes the device's connectors and lights or drops what changed.
    pub fn device_changed(&mut self, node: DrmNode) {
        let Some(device) = self.udev().devices.get_mut(&node) else {
            return;
        };
        let events = match device.scanner.scan_connectors(device.manager.device()) {
            Ok(events) => events,
            Err(err) => {
                warn!(%node, "could not scan connectors: {err}");
                return;
            }
        };
        for event in events {
            match event {
                DrmScanEvent::Connected {
                    connector,
                    crtc: Some(crtc),
                } => self.connector_connected(node, connector, crtc),
                DrmScanEvent::Disconnected {
                    crtc: Some(crtc), ..
                } => self.connector_disconnected(node, crtc),
                _ => {}
            }
        }
        self.arm_reprobe();
    }

    fn device_removed(&mut self, node: DrmNode) {
        let crtcs: Vec<crtc::Handle> = self
            .udev()
            .devices
            .get(&node)
            .map(|d| d.lit.keys().copied().collect())
            .unwrap_or_default();
        for crtc in crtcs {
            self.connector_disconnected(node, crtc);
        }
        self.udev().devices.remove(&node);
        self.arm_reprobe();
    }

    fn connector_connected(
        &mut self,
        node: DrmNode,
        connector: connector::Info,
        crtc: crtc::Handle,
    ) {
        let State { backend, isle } = self;
        let Backend::Udev(udev) = backend else {
            return;
        };
        let Some(device) = udev.devices.get_mut(&node) else {
            return;
        };
        let drm = device.manager.device();
        let name = format!(
            "{}-{}",
            connector.interface().as_str(),
            connector.interface_id()
        );
        let info = display_info::for_connector(drm, connector.handle());
        let make = info
            .as_ref()
            .and_then(|i| i.make())
            .unwrap_or_else(|| "Unknown".into());
        let model = info
            .as_ref()
            .and_then(|i| i.model())
            .unwrap_or_else(|| "Unknown".into());
        let serial = info
            .as_ref()
            .and_then(|i| i.serial())
            .unwrap_or_else(|| "Unknown".into());
        let identity = format!("{make} {model} {serial}");
        let config = isle
            .settings
            .output
            .get(&identity)
            .cloned()
            .unwrap_or_default();
        if !config.enabled {
            info!(name, identity, "output disabled in settings");
            return;
        }

        let preferred = connector
            .modes()
            .iter()
            .position(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
            .unwrap_or(0);
        let Some(&preferred_mode) = connector.modes().get(preferred) else {
            warn!(name, "the connector offers no mode");
            return;
        };
        let drm_mode = config
            .mode
            .as_deref()
            .and_then(|wanted| pick_mode(connector.modes(), wanted))
            .unwrap_or(preferred_mode);

        let (w_mm, h_mm) = connector.size().unwrap_or((0, 0));
        let output = Output::new(
            name.clone(),
            PhysicalProperties {
                size: (w_mm as i32, h_mm as i32).into(),
                subpixel: connector.subpixel().into(),
                make,
                model,
                serial_number: serial,
            },
        );
        let global = output.create_global::<State>(&isle.display_handle);
        output.set_preferred(Mode::from(preferred_mode));
        output.change_current_state(
            Some(Mode::from(drm_mode)),
            Some(transform(config.transform)),
            Some(Scale::Fractional(config.scale)),
            None,
        );
        output
            .user_data()
            .insert_if_missing(|| OutputId { device: node, crtc });

        let mut planes = match drm.planes(&crtc) {
            Ok(planes) => planes,
            Err(err) => {
                warn!(name, "could not read the CRTC's planes: {err}");
                return;
            }
        };
        // Overlay planes break NVIDIA's display engine (cosmic-comp's quirk, P2 choice 9).
        if device.nvidia {
            planes.overlay.clear();
        }
        let renderer = udev.renderer.as_mut().unwrap();
        let surface = match device
            .manager
            .lock()
            .initialize_output::<_, IsleElement>(
                crtc,
                drm_mode,
                &[connector.handle()],
                &output,
                Some(planes),
                renderer,
                &DrmOutputRenderElements::default(),
            ) {
            Ok(surface) => surface,
            Err(err) => {
                warn!(name, "could not light the output: {err}");
                isle.display_handle.remove_global::<State>(global);
                return;
            }
        };
        device.lit.insert(
            crtc,
            Lit {
                output: output.clone(),
                global,
                surface,
            },
        );

        let x = isle
            .space
            .outputs()
            .filter_map(|o| isle.space.output_geometry(o))
            .map(|g| g.loc.x + g.size.w)
            .max()
            .unwrap_or(0);
        let position = match (config.x, config.y) {
            (Some(x), Some(y)) => (x, y),
            _ => (x, 0),
        };
        info!(name, identity, mode = ?Mode::from(drm_mode), ?position, "output lit");
        isle.add_output(&output, position.into());
        isle.queue_redraw(&output);
    }

    fn connector_disconnected(&mut self, node: DrmNode, crtc: crtc::Handle) {
        let State { backend, isle } = self;
        let Backend::Udev(udev) = backend else {
            return;
        };
        let Some(device) = udev.devices.get_mut(&node) else {
            return;
        };
        let Some(lit) = device.lit.remove(&crtc) else {
            return;
        };
        info!(name = lit.output.name(), "output gone");
        isle.remove_output(&lit.output);
        lit.output.leave_all();
        isle.display_handle.remove_global::<State>(lit.global);
        drop(lit.surface);
        let renderer = udev.renderer.as_mut().unwrap();
        let _ = device
            .manager
            .lock()
            .try_to_restore_modifiers::<_, IsleElement>(
                renderer,
                &DrmOutputRenderElements::default(),
            );
    }

    /// While nothing is lit, probe the connectors every few seconds until something is,
    /// since a KVM can hand a screen back without a hotplug event.
    fn arm_reprobe(&mut self) {
        let lit = !self.isle.space.outputs().next().is_none();
        let udev = self.udev();
        if lit {
            if let Some(token) = udev.reprobe.take() {
                self.isle.loop_handle.remove(token);
            }
            return;
        }
        if udev.reprobe.is_some() {
            return;
        }
        warn!("no output is lit; probing every {} s", REPROBE.as_secs());
        let token = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(REPROBE), |_, _, state| {
                let _beat = state.isle.heartbeat.beat("reprobe");
                let nodes: Vec<DrmNode> = state.udev().devices.keys().copied().collect();
                for node in nodes {
                    state.device_changed(node);
                }
                if state.isle.space.outputs().next().is_some() {
                    state.udev().reprobe = None;
                    TimeoutAction::Drop
                } else {
                    TimeoutAction::ToDuration(REPROBE)
                }
            })
            .expect("the reprobe timer");
        self.udev().reprobe = Some(token);
    }

    /// Releases every output and probes again as one step (`isle displays redetect`).
    pub fn redetect(&mut self) {
        if !matches!(self.backend, Backend::Udev(_)) {
            return;
        }
        info!("redetecting outputs");
        let nodes: Vec<DrmNode> = self.udev().devices.keys().copied().collect();
        for node in nodes {
            let crtcs: Vec<crtc::Handle> = self.udev().devices[&node].lit.keys().copied().collect();
            for crtc in crtcs {
                self.connector_disconnected(node, crtc);
            }
            if let Some(device) = self.udev().devices.get_mut(&node) {
                device.scanner = DrmScanner::new();
            }
            self.device_changed(node);
        }
    }

    fn drm_vblank(&mut self, node: DrmNode, crtc: crtc::Handle) {
        let Some(lit) = self
            .udev()
            .devices
            .get_mut(&node)
            .and_then(|d| d.lit.get_mut(&crtc))
        else {
            return;
        };
        if let Err(err) = lit.surface.frame_submitted() {
            warn!("frame not submitted: {err:?}");
        }
        let output = lit.output.clone();
        self.on_vblank(&output);
    }

    fn session_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::PauseSession => {
                info!("session paused");
                let udev = self.udev();
                udev.libinput.suspend();
                for device in udev.devices.values_mut() {
                    device.manager.pause();
                }
            }
            SessionEvent::ActivateSession => {
                info!("session active");
                if self.udev().waiting.is_some() {
                    if let Err(err) = self.open_render_gpu() {
                        error!("could not open the render GPU: {err}");
                        self.isle.loop_signal.stop();
                    }
                    // Input devices could not be opened while the session was inactive.
                    let udev = self.udev();
                    udev.libinput.suspend();
                    if udev.libinput.resume().is_err() {
                        error!("libinput did not resume");
                    }
                    return;
                }
                let udev = self.udev();
                if udev.libinput.resume().is_err() {
                    error!("libinput did not resume");
                }
                for device in udev.devices.values_mut() {
                    if let Err(err) = device.manager.lock().activate(false) {
                        error!("could not take the GPU back: {err}");
                    }
                    for lit in device.lit.values() {
                        lit.surface.reset_buffers();
                    }
                }
                // A KVM or another session may have changed what is connected.
                let nodes: Vec<DrmNode> = self.udev().devices.keys().copied().collect();
                for node in nodes {
                    self.device_changed(node);
                }
                self.isle.queue_redraw_all();
            }
        }
    }

    fn libinput_event(&mut self, event: InputEvent<LibinputInputBackend>) {
        match &event {
            InputEvent::DeviceAdded { device } => {
                info!(
                    name = %device.name(),
                    keyboard = device.has_capability(DeviceCapability::Keyboard),
                    pointer = device.has_capability(DeviceCapability::Pointer),
                    "input device"
                );
                if device.has_capability(DeviceCapability::Keyboard) {
                    use smithay::backend::input::Device as _;
                    self.keyboard_added(&device.id(), &device.name(), None);
                    const KEY_A: u32 = 30;
                    if !device.keyboard_has_key(KEY_A).unwrap_or(true)
                        && let Some(k) =
                            self.isle.keyboards.devices.iter_mut().find(|k| k.id == device.id())
                    {
                        k.typing = false;
                    }
                }
                if device.has_capability(DeviceCapability::Pointer) {
                    let mut device = device.clone();
                    self.apply_pointer_settings(&mut device);
                    self.udev().pointers.push(device);
                }
            }
            InputEvent::DeviceRemoved { device } => {
                use smithay::backend::input::Device as _;
                self.keyboard_removed(&device.id());
                self.udev().pointers.retain(|d| d != device);
            }
            _ => {}
        }
        self.process_input_event(event);
    }

    /// A pointing device's settings, by its name (`pointer "name" { }`).
    pub fn apply_pointer_settings(&self, device: &mut libinput::Device) {
        use isle_settings::schema::{AccelProfile, ScrollMethod};
        let Some(config) = self
            .isle
            .settings
            .pointer
            .get(device.name().as_ref())
            .cloned()
        else {
            let _ = device.config_accel_set_profile(libinput::AccelProfile::Flat);
            return;
        };
        let _ = device.config_accel_set_profile(match config.accel_profile {
            AccelProfile::Flat => libinput::AccelProfile::Flat,
            AccelProfile::Adaptive => libinput::AccelProfile::Adaptive,
        });
        let _ = device.config_accel_set_speed(config.speed);
        let _ = device.config_left_handed_set(config.left_handed);
        if let Some(natural) = config.natural_scroll {
            let _ = device.config_scroll_set_natural_scroll_enabled(natural);
        }
        if let Some(tap) = config.tap {
            let _ = device.config_tap_set_enabled(tap);
        }
        if let Some(method) = config.scroll_method {
            let _ = device.config_scroll_set_method(match method {
                ScrollMethod::TwoFinger => libinput::ScrollMethod::TwoFinger,
                ScrollMethod::Edge => libinput::ScrollMethod::Edge,
                ScrollMethod::OnButtonDown => libinput::ScrollMethod::OnButtonDown,
                ScrollMethod::NoScroll => libinput::ScrollMethod::NoScroll,
            });
        }
    }

    /// Applies pointer settings to every device after they change.
    pub fn reapply_pointer_settings(&mut self) {
        let Backend::Udev(udev) = &mut self.backend else {
            return;
        };
        let mut devices = std::mem::take(&mut udev.pointers);
        for device in &mut devices {
            self.apply_pointer_settings(device);
        }
        self.udev().pointers = devices;
    }

    pub fn switch_vt(&mut self, vt: i32) {
        if let Backend::Udev(udev) = &mut self.backend
            && let Err(err) = udev.session.change_vt(vt)
        {
            warn!(vt, "could not switch VT: {err}");
        }
    }
}

/// The connector mode a `WIDTHxHEIGHT[@HZ]` setting names, nearest in refresh.
fn pick_mode(
    modes: &[smithay::reexports::drm::control::Mode],
    wanted: &str,
) -> Option<smithay::reexports::drm::control::Mode> {
    let (size, hz) = match wanted.split_once('@') {
        Some((size, hz)) => (size, hz.parse::<f64>().ok()),
        None => (wanted, None),
    };
    let (w, h) = size.split_once('x')?;
    let (w, h): (u16, u16) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    modes
        .iter()
        .filter(|m| m.size() == (w, h))
        .min_by_key(|m| match hz {
            Some(hz) => ((Mode::from(**m).refresh as f64 / 1000.0 - hz).abs() * 1000.0) as i64,
            None => -(Mode::from(**m).refresh as i64),
        })
        .copied()
}

fn transform(setting: isle_settings::schema::Transform) -> Transform {
    use isle_settings::schema::Transform as T;
    match setting {
        T::Normal => Transform::Normal,
        T::Rotate90 => Transform::_90,
        T::Rotate180 => Transform::_180,
        T::Rotate270 => Transform::_270,
        T::Flipped => Transform::Flipped,
        T::Flipped90 => Transform::Flipped90,
        T::Flipped180 => Transform::Flipped180,
        T::Flipped270 => Transform::Flipped270,
    }
}
