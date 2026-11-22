//! What Get brings and Remove takes, said before the password (`p14e-store.md` choices 9–11).

use std::collections::HashSet;
use std::process::Command;

use isle_catalog::catalogue::{Kind, Offer};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: String,
    /// Download size in bytes, when known.
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// Every package the run touches, the app's own first.
    pub packages: Vec<Package>,
    /// Why it cannot be done: the packages that need it, or the command's own words.
    pub refused: Option<String>,
    /// Optional extras: a package and what it adds.
    pub extras: Vec<(String, String)>,
    pub note: Option<String>,
    /// An AUR package's build script and install script, read before it is built.
    pub scripts: Vec<Script>,
}

/// A script an AUR build runs, with the lines that deserve a closer look.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Script {
    pub name: String,
    pub text: String,
    pub flags: Vec<Flag>,
}

/// A line of a script and why it is worth a look; `line` counts from 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Flag {
    pub line: usize,
    pub why: &'static str,
}

/// The lines of a PKGBUILD or install script that do something a package rarely needs:
/// running fetched or hidden code, reaching outside the package, or acting as root itself.
pub fn review(text: &str) -> Vec<Flag> {
    let word = |l: &str, w: &str| {
        l.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
            .any(|t| t == w)
    };
    let mut out = Vec::new();
    // Inside a list of package names (`depends=(…)` and its kin), which name tools, not run them.
    let mut names = false;
    for (i, raw) in text.lines().enumerate() {
        let l = raw.trim();
        let opens = ["depends=", "makedepends=", "checkdepends=", "optdepends=", "provides=", "conflicts=", "replaces="]
            .iter()
            .any(|k| l.starts_with(k));
        if opens {
            names = !l.contains(')');
            continue;
        }
        if names {
            names = !l.contains(')');
            continue;
        }
        if l.is_empty() || l.starts_with('#') || l.starts_with("url=") {
            continue;
        }
        let fetches = word(l, "curl") || word(l, "wget");
        let why = if fetches
            && l.split('|').skip(1).any(|p| {
                let p = p.trim_start().trim_start_matches("sudo ");
                ["sh", "bash", "zsh", "python", "python3", "perl"]
                    .iter()
                    .any(|s| p == *s || p.starts_with(&format!("{s} ")))
            }) {
            Some("Runs a script fetched from the network")
        } else if fetches || l.contains("git clone") {
            Some("Downloads while it builds, outside the sources it lists and checks")
        } else if word(l, "sudo") || word(l, "pkexec") || l.starts_with("su ") {
            Some("Asks for root by itself")
        } else if l.contains("base64 -d") || l.contains("base64 --decode") || l.contains("xxd -r") {
            Some("Decodes hidden text")
        } else if word(l, "eval") {
            Some("Runs text it puts together as a command")
        } else if l.contains("/dev/tcp") || l.contains("/dev/udp") || word(l, "nc") || word(l, "ncat") || word(l, "socat") {
            Some("Opens a network connection")
        } else if word(l, "chmod")
            && (l.contains("777") || l.contains("+s") || l.split_whitespace().any(|t| t.len() == 4 && t.starts_with(['4', '2', '6']) && t.chars().all(|c| c.is_ascii_digit())))
        {
            Some("Makes files writable by anyone, or run as their owner")
        } else if l.contains("$HOME") || l.contains("~/") || l.contains("/home/") {
            Some("Reaches into home folders")
        } else if word(l, "rm")
            && l.contains("-r")
            && l.split_whitespace().any(|t| t == "/" || t == "/*" || t.starts_with("/usr") || t.starts_with("/etc") || t.starts_with("/boot"))
        {
            Some("Removes files outside the package")
        } else if l.contains("systemctl enable") || l.contains("systemctl start") {
            Some("Starts a service by itself")
        } else if l.contains("http://") && !l.contains("https://") {
            Some("Fetches over unencrypted http")
        } else {
            None
        };
        if let Some(why) = why {
            out.push(Flag { line: i + 1, why });
        }
    }
    out
}

/// The install script a PKGBUILD names, with `$pkgname` filled in.
pub fn install_script(pkgbuild: &str, pkgname: &str) -> Option<String> {
    let line = pkgbuild.lines().find(|l| l.trim_start().starts_with("install="))?;
    let value = line.trim_start()["install=".len()..]
        .trim()
        .trim_matches(['"', '\'']);
    let name = value
        .replace("${pkgname}", pkgname)
        .replace("$pkgname", pkgname);
    (!name.is_empty() && !name.contains('$') && !name.contains('/')).then_some(name)
}

impl Plan {
    pub fn total(&self) -> u64 {
        self.packages.iter().filter_map(|p| p.size).sum()
    }

    /// The packages besides the app's own.
    pub fn others(&self, own: &str) -> usize {
        self.packages.iter().filter(|p| p.name != own).count()
    }
}

/// `pacman --print --print-format '%n %v %s'`: a package per line.
pub fn parse_print(out: &str) -> Vec<Package> {
    out.lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let name = f.next()?.to_string();
            let version = f.next().unwrap_or_default().to_string();
            Some(Package {
                name,
                version,
                size: f.next().and_then(|s| s.parse().ok()),
            })
        })
        .collect()
}

/// The packages a removal would break, from pacman's
/// `:: removing X breaks dependency 'X' required by Y` lines.
pub fn parse_breaks(err: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in err.lines() {
        if let Some((_, by)) = l.split_once(" required by ") {
            let by = by.trim().to_string();
            if !out.contains(&by) {
                out.push(by);
            }
        }
    }
    out
}

/// `pacman -Si`'s Optional Deps: `name: what it adds`, continued on indented lines.
pub fn parse_optdepends(info: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut inside = false;
    for l in info.lines() {
        let entry = if let Some(rest) = l.strip_prefix("Optional Deps") {
            inside = true;
            rest.trim_start().trim_start_matches(':').trim()
        } else if inside && l.starts_with(char::is_whitespace) {
            l.trim()
        } else {
            inside = false;
            continue;
        };
        if entry == "None" || entry.is_empty() {
            continue;
        }
        let (name, why) = entry.split_once(": ").unwrap_or((entry, ""));
        let name = name.split(['>', '<', '=']).next().unwrap_or(name).to_string();
        let why = why.trim_end_matches("[installed]").trim().to_string();
        out.push((name, why));
    }
    out
}

/// `flatpak remote-info`'s download size and runtime.
pub fn parse_remote_info(out: &str) -> (Option<u64>, Option<String>) {
    let mut size = None;
    let mut runtime = None;
    for l in out.lines() {
        let Some((key, value)) = l.split_once(':') else {
            continue;
        };
        match key.trim() {
            "Download" | "Download Size" => size = parse_size(value.trim()),
            "Runtime" => runtime = Some(value.trim().to_string()),
            _ => {}
        }
    }
    (size, runtime)
}

/// `123.4 MB`, `1.2 GB`, `900 kB`, as flatpak writes them.
fn parse_size(s: &str) -> Option<u64> {
    let (n, unit) = s.split_once(char::is_whitespace)?;
    let n: f64 = n.replace('\u{a0}', "").parse().ok()?;
    let scale = match unit.trim() {
        "bytes" | "B" => 1.,
        "kB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        _ => return None,
    };
    Some((n * scale) as u64)
}

pub fn size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.0} MB", b / 1e6)
    } else {
        format!("{:.0} KB", (b / 1e3).max(1.))
    }
}

fn run(program: &str, args: &[&str]) -> Option<(bool, String, String)> {
    let out = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// Of `deps`, those nothing installed provides (`pacman -T`).
fn missing(deps: &[String]) -> Vec<String> {
    if deps.is_empty() {
        return Vec::new();
    }
    let args: Vec<&str> = std::iter::once("-T")
        .chain(deps.iter().map(String::as_str))
        .collect();
    run("pacman", &args)
        .map(|(_, out, _)| out.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// A dependency's name without its version: `obs-studio>=28` is `obs-studio`.
fn bare_name(dep: &str) -> &str {
    dep.split(['>', '<', '=']).next().unwrap_or(dep)
}

/// What installing `lacking` brings: the repositories' packages with what they need and their
/// sizes, then those only the AUR has, which paru builds.
fn resolve(lacking: &[String]) -> Vec<Package> {
    let names: Vec<&str> = lacking.iter().map(|d| bare_name(d)).collect();
    let in_repos = |name: &str| {
        run("pacman", &["-Sp", "--print-format", "%n", name]).is_some_and(|(ok, _, _)| ok)
    };
    let (repo, aur): (Vec<&str>, Vec<&str>) = names.iter().partition(|n| in_repos(n));
    let mut out = Vec::new();
    if !repo.is_empty() {
        let args: Vec<&str> = ["-S", "--print", "--print-format", "%n %v %s"]
            .into_iter()
            .chain(repo.iter().copied())
            .collect();
        if let Some((true, printed, _)) = run("pacman", &args) {
            out.extend(parse_print(&printed));
        }
    }
    out.extend(aur.into_iter().map(|name| Package {
        name: name.to_string(),
        version: "from the AUR".into(),
        size: None,
    }));
    out
}

fn pacman_get(package: &str) -> Plan {
    let mut plan = Plan::default();
    match run(
        "pacman",
        &["-S", "--print", "--print-format", "%n %v %s", package],
    ) {
        Some((true, out, _)) => {
            let mut packages = parse_print(&out);
            if let Some(i) = packages.iter().position(|p| p.name == package) {
                let own = packages.remove(i);
                packages.insert(0, own);
            }
            plan.packages = packages;
        }
        Some((false, _, err)) => plan.refused = Some(err.trim().to_string()),
        None => plan.refused = Some("pacman could not be run.".into()),
    }
    if let Some((true, info, _)) = run("pacman", &["-Si", package]) {
        let extras = parse_optdepends(&info);
        let names: Vec<String> = extras.iter().map(|(n, _)| n.clone()).collect();
        let wanted: HashSet<String> = missing(&names).into_iter().collect();
        plan.extras = extras
            .into_iter()
            .filter(|(n, _)| wanted.contains(n))
            .collect();
    }
    plan
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AurInfo {
    #[serde(default)]
    depends: Vec<String>,
    #[serde(default)]
    make_depends: Vec<String>,
    #[serde(default)]
    opt_depends: Vec<String>,
    version: String,
    #[serde(default)]
    package_base: Option<String>,
}

/// A file of an AUR package's repository, as the AUR shows it.
async fn aur_file(base: &str, file: &str) -> Option<String> {
    let reply = isle_catalog::catalogue::aur_client()
        .get("https://aur.archlinux.org/cgit/aur.git/plain/".to_string() + file)
        .query(&[("h", base)])
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    reply.text().await.ok()
}

#[derive(serde::Deserialize)]
struct AurReply {
    #[serde(default)]
    results: Vec<AurInfo>,
}

async fn aur_get(package: &str) -> Plan {
    let mut plan = Plan {
        note: Some("Built on this computer from a script anyone can publish; neither CachyOS nor Arch checks it.".into()),
        ..Plan::default()
    };
    let reply = async {
        isle_catalog::catalogue::aur_client()
            .get("https://aur.archlinux.org/rpc/v5/info")
            .query(&[("arg[]", package)])
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<AurReply>()
            .await
            .map_err(|e| e.to_string())
    }
    .await;
    let info = match reply {
        Ok(r) => r.results.into_iter().next(),
        Err(e) => {
            plan.refused = Some(format!("The AUR could not be reached: {e}"));
            return plan;
        }
    };
    let Some(info) = info else {
        plan.refused = Some(format!("The AUR has no package named {package}."));
        return plan;
    };
    plan.packages.push(Package {
        name: package.to_string(),
        version: info.version,
        size: None,
    });
    let deps: Vec<String> = info
        .depends
        .iter()
        .chain(&info.make_depends)
        .cloned()
        .collect();
    let found = tokio::task::spawn_blocking(move || resolve(&missing(&deps)))
        .await
        .unwrap_or_default();
    plan.packages.extend(found);
    let base = info.package_base.clone().unwrap_or_else(|| package.to_string());
    if let Some(pkgbuild) = aur_file(&base, "PKGBUILD").await {
        let install = install_script(&pkgbuild, package);
        plan.scripts.push(Script {
            name: "PKGBUILD".into(),
            flags: review(&pkgbuild),
            text: pkgbuild,
        });
        if let Some(name) = install
            && let Some(text) = aur_file(&base, &name).await
        {
            plan.scripts.push(Script {
                flags: review(&text),
                name,
                text,
            });
        }
    }
    plan.extras = info
        .opt_depends
        .iter()
        .map(|o| {
            let (n, why) = o.split_once(": ").unwrap_or((o, ""));
            (n.to_string(), why.to_string())
        })
        .collect();
    plan
}

fn flathub_get(app: &str) -> Plan {
    let mut plan = Plan::default();
    let Some((ok, out, err)) = run("flatpak", &["remote-info", isle_updates::flathub_installation(), "flathub", app]) else {
        plan.refused = Some("Flatpak is not installed. Install the flatpak package from CachyOS first.".into());
        return plan;
    };
    if !ok {
        plan.refused = Some(err.trim().to_string());
        return plan;
    }
    let (size, runtime) = parse_remote_info(&out);
    plan.packages.push(Package {
        name: app.to_string(),
        version: String::new(),
        size,
    });
    if let Some(runtime) = runtime {
        let have = run("flatpak", &["info", &runtime]).is_some_and(|(ok, _, _)| ok);
        if !have {
            let (size, _) = run("flatpak", &["remote-info", isle_updates::flathub_installation(), "flathub", &runtime])
                .map(|(_, out, _)| parse_remote_info(&out))
                .unwrap_or_default();
            // `org.gnome.Platform/x86_64/50`: the runtime and its version.
            let mut parts = runtime.split('/');
            let (name, version) = (parts.next().unwrap_or_default(), parts.nth(1).unwrap_or_default());
            plan.packages.push(Package {
                name: name.to_string(),
                version: version.to_string(),
                size,
            });
        }
    }
    plan
}

/// What getting `offer` brings.
pub async fn get(offer: Offer) -> Plan {
    match offer.kind {
        Kind::Aur => aur_get(&offer.install).await,
        Kind::Cachyos => tokio::task::spawn_blocking(move || pacman_get(&offer.install))
            .await
            .unwrap_or_default(),
        Kind::Flathub => tokio::task::spawn_blocking(move || flathub_get(&offer.install))
            .await
            .unwrap_or_default(),
    }
}

/// What removing a package takes, or who needs it.
pub fn remove(catalogue: &str, id: &str) -> Plan {
    let mut plan = Plan::default();
    if catalogue == "flathub" {
        plan.packages.push(Package {
            name: id.to_string(),
            version: String::new(),
            size: None,
        });
        plan.note = Some("Its documents and settings stay in your home folder.".into());
        return plan;
    }
    match run("pacman", &["-Rs", "--print", "--print-format", "%n %v", id]) {
        Some((true, out, _)) => {
            plan.packages = parse_print(&out);
            plan.note = Some("Settings you changed are kept beside the originals as .pacsave files.".into());
        }
        // pacman prints what a removal would break on stdout, its error on stderr.
        Some((false, out, err)) => {
            let by = parse_breaks(&format!("{out}\n{err}"));
            plan.refused = Some(if by.is_empty() {
                err.trim().to_string()
            } else {
                format!("{} needs it.", by.join(", "))
            });
        }
        None => plan.refused = Some("pacman could not be run.".into()),
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints what review flags in each file of `$ISLE_REVIEW_DIR`, to judge it on real scripts.
    #[test]
    #[ignore]
    fn review_real_scripts() {
        let Some(dir) = std::env::var_os("ISLE_REVIEW_DIR") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let text = std::fs::read_to_string(e.path()).unwrap_or_default();
            let lines: Vec<&str> = text.lines().collect();
            for f in review(&text) {
                println!("{:?}:{} {} | {}", e.file_name(), f.line, f.why, lines[f.line - 1].trim());
            }
        }
    }

    #[test]
    fn a_build_script_s_risky_lines() {
        let pkgbuild = r#"pkgname=demo
url="https://example.org/demo"
source=("http://example.org/demo.tar.gz")
install=$pkgname.install
# curl https://example.org/x | sh is only a comment
build() {
  curl -fsSL https://example.org/setup.sh | bash
  wget https://example.org/extra.bin
  echo aGVsbG8= | base64 -d > hidden
  eval "$cmd"
  chmod 4755 "$pkgdir/usr/bin/demo"
  cp config ~/.config/demo
  sudo make install
}
package() {
  install -Dm755 demo "$pkgdir/usr/bin/demo"
  rm -rf "$srcdir/build"
}
"#;
        let got: Vec<(usize, &str)> = review(pkgbuild).iter().map(|f| (f.line, f.why)).collect();
        assert_eq!(
            got,
            [
                (3, "Fetches over unencrypted http"),
                (7, "Runs a script fetched from the network"),
                (8, "Downloads while it builds, outside the sources it lists and checks"),
                (9, "Decodes hidden text"),
                (10, "Runs text it puts together as a command"),
                (11, "Makes files writable by anyone, or run as their owner"),
                (12, "Reaches into home folders"),
                (13, "Asks for root by itself"),
            ]
        );
        assert_eq!(install_script(pkgbuild, "demo").as_deref(), Some("demo.install"));
        // Names of tools in dependency lists, and the homepage, are not what the build runs.
        let lists = "url=\"http://example.org\"\noptdepends=('sudo: privilege elevation'\n  'wget: downloads'\n  'socat: sandbox')\ndepends=('curl')\n";
        assert!(review(lists).is_empty(), "{:?}", review(lists));
        assert_eq!(install_script("pkgname=x\n", "x"), None);
        let install = "post_install() {\n  systemctl enable --now demo.service\n  rm -rf /etc/demo-old\n}\n";
        let got: Vec<usize> = review(install).iter().map(|f| f.line).collect();
        assert_eq!(got, [2, 3]);
    }

    #[test]
    fn get_s_list() {
        let got = parse_print("krita 5.2.9-3 98423311\nkimageformats5 5.116.0-1 211000\n");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].size, Some(98423311));
        assert_eq!(got[1].version, "5.116.0-1");
    }

    #[test]
    fn a_refused_removal_names_who_needs_it() {
        let err = "checking dependencies...\nerror: failed to prepare transaction (could not satisfy dependencies)\n:: removing libplacebo breaks dependency 'libplacebo' required by ffmpeg\n:: removing libplacebo breaks dependency 'libplacebo.so=338-64' required by mpv\n:: removing libplacebo breaks dependency 'libplacebo' required by ffmpeg\n";
        assert_eq!(parse_breaks(err), ["ffmpeg", "mpv"]);
    }

    #[test]
    fn works_better_with() {
        let info = "Name            : krita\nOptional Deps   : poppler-qt5: PDF filter [installed]\n                  libheif: HEIF filter\n                  krita-plugin-gmic>=3: GMic plugin\nRequired By     : None\n";
        assert_eq!(
            parse_optdepends(info),
            [
                ("poppler-qt5".to_string(), "PDF filter".to_string()),
                ("libheif".to_string(), "HEIF filter".to_string()),
                ("krita-plugin-gmic".to_string(), "GMic plugin".to_string()),
            ]
        );
        assert!(parse_optdepends("Optional Deps   : None\n").is_empty());
    }

    #[test]
    fn flathub_s_facts() {
        let out = "        ID: com.obsproject.Studio\n       Ref: app/com.obsproject.Studio/x86_64/stable\n  Download: 112.6 MB\n Installed: 341.0 MB\n   Runtime: org.kde.Platform/x86_64/6.8\n";
        assert_eq!(
            parse_remote_info(out),
            (Some(112_600_000), Some("org.kde.Platform/x86_64/6.8".into()))
        );
        assert_eq!(
            parse_remote_info(" Download Size: 930.5 kB\n       Runtime: org.gnome.Platform/x86_64/50\n").0,
            Some(930_500)
        );
        assert_eq!(size(532_000_000), "532 MB");
        assert_eq!(size(1_234_000_000), "1.2 GB");
    }
}
