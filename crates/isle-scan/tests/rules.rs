//! Every shipped rule on a home that has what it looks for, and on one that has the same but
//! for the condition that rules it out.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use isle_scan::{Context, Find, Rules, Scan};

struct Home(PathBuf);

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn home(name: &str) -> Home {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("isle-rules-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    Home(d)
}

fn put(root: &Path, rel: &str, days_old: u64) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(&[7; 8192]).unwrap();
    if days_old > 0 {
        let then = SystemTime::now() - Duration::from_secs(days_old * 24 * 3600);
        f.set_times(fs::FileTimes::new().set_accessed(then).set_modified(then))
            .unwrap();
    }
}

const STEAM: &str = ".local/share/Steam";

/// A rule, what makes a home have it, where its find is and under what name, and what a home
/// without it has instead.
struct Case {
    rule: &'static str,
    has: &'static [&'static str],
    at: &'static str,
    name: &'static str,
    lacks: &'static [&'static str],
}

const OLD: &str = "@old";

const CASES: &[Case] = &[
    Case {
        rule: "npm",
        has: &[".npm/_cacache/x"],
        at: ".npm/_cacache",
        name: "npm",
        lacks: &[".npm/other/x"],
    },
    Case {
        rule: "pnpm",
        has: &[".local/share/pnpm/store/x"],
        at: ".local/share/pnpm/store",
        name: "pnpm",
        lacks: &[".local/share/pnpm/x"],
    },
    Case {
        rule: "bun",
        has: &[".bun/install/cache/x"],
        at: ".bun/install/cache",
        name: "Bun",
        lacks: &[".bun/bin/x"],
    },
    Case {
        rule: "cargo",
        has: &[".cargo/registry/cache/x"],
        at: ".cargo/registry/cache",
        name: "Cargo",
        lacks: &[".cargo/bin/x"],
    },
    Case {
        rule: "gradle-cache",
        has: &[".gradle/caches/x"],
        at: ".gradle/caches",
        name: "Gradle",
        lacks: &[".gradle/wrapper/x"],
    },
    Case {
        rule: "maven",
        has: &[".m2/repository/x"],
        at: ".m2/repository",
        name: "Maven",
        lacks: &[".m2/settings/x"],
    },
    Case {
        rule: "nuget",
        has: &[".nuget/packages/x"],
        at: ".nuget/packages",
        name: "NuGet",
        lacks: &[".nuget/other/x"],
    },
    Case {
        rule: "yarn",
        has: &[".yarn/berry/cache/x"],
        at: ".yarn/berry/cache",
        name: "Yarn",
        lacks: &[".yarn/berry/other/x"],
    },
    Case {
        rule: "electron",
        has: &[".config/Slack/Cache/x"],
        at: ".config/Slack/Cache",
        name: "Slack",
        lacks: &[".config/Slack/Storage/x"],
    },
    Case {
        rule: "crash-reports",
        has: &[".config/Slack/Crashpad/completed/x"],
        at: ".config/Slack/Crashpad/completed",
        name: "Slack crash reports",
        lacks: &[".config/Slack/Crashpad/settings/x"],
    },
    Case {
        rule: "steam-web",
        has: &[".local/share/Steam/config/htmlcache/x"],
        at: ".local/share/Steam/config/htmlcache",
        name: "Steam web pages",
        lacks: &[".local/share/Steam/config/other/x"],
    },
    Case {
        rule: "flatpak-cache",
        has: &[".var/app/org.x.App/cache/x"],
        at: ".var/app/org.x.App/cache",
        name: "org.x.App",
        lacks: &[".var/app/org.x.App/data/x"],
    },
    Case {
        rule: "cache",
        has: &[".cache/spotify/x"],
        at: ".cache/spotify",
        name: "Spotify",
        lacks: &[".cache/cliphist/x"],
    },
    Case {
        rule: "node-modules",
        has: &["web/node_modules/x", "web/package.json"],
        at: "web/node_modules",
        name: "web",
        lacks: &["web/node_modules/x"],
    },
    Case {
        rule: "rust-target",
        has: &["app/target/x", "app/Cargo.toml"],
        at: "app/target",
        name: "app",
        lacks: &["app/target/x"],
    },
    Case {
        rule: "python-env",
        has: &["py/.venv/x", "py/pyproject.toml"],
        at: "py/.venv",
        name: "py",
        lacks: &["py/.venv/x"],
    },
    Case {
        rule: "cmake",
        has: &["c/build/x", "c/CMakeLists.txt"],
        at: "c/build",
        name: "c",
        lacks: &["c/build/x"],
    },
    Case {
        rule: "meson",
        has: &["m/builddir/x", "m/meson.build"],
        at: "m/builddir",
        name: "m",
        lacks: &["m/builddir/x"],
    },
    Case {
        rule: "gradle-build",
        has: &["g/build/x", "g/build.gradle"],
        at: "g/build",
        name: "g",
        lacks: &["g/build/x"],
    },
    Case {
        rule: "web-build",
        has: &["w/.next/x", "w/package.json"],
        at: "w/.next",
        name: "w",
        lacks: &["w/.next/x"],
    },
    Case {
        rule: "zig",
        has: &["z/zig-out/x", "z/build.zig"],
        at: "z/zig-out",
        name: "z",
        lacks: &["z/zig-out/x"],
    },
    Case {
        rule: "dart",
        has: &["d/.dart_tool/x", "d/pubspec.yaml"],
        at: "d/.dart_tool",
        name: "d",
        lacks: &["d/.dart_tool/x"],
    },
    Case {
        rule: "elixir",
        has: &["e/_build/x", "e/mix.exs"],
        at: "e/_build",
        name: "e",
        lacks: &["e/_build/x"],
    },
    Case {
        rule: "haskell",
        has: &["h/.stack-work/x", "h/stack.yaml"],
        at: "h/.stack-work",
        name: "h",
        lacks: &["h/.stack-work/x"],
    },
    Case {
        rule: "steam-prefix",
        has: &[".local/share/Steam/steamapps/compatdata/123/x"],
        at: ".local/share/Steam/steamapps/compatdata/123",
        name: "Proton prefix · app 123",
        lacks: &[
            ".local/share/Steam/steamapps/compatdata/123/x",
            ".local/share/Steam/steamapps/appmanifest_123.acf",
        ],
    },
    Case {
        rule: "steam-shaders",
        has: &[".local/share/Steam/steamapps/shadercache/123/x"],
        at: ".local/share/Steam/steamapps/shadercache/123",
        name: "Shader cache · app 123",
        lacks: &[
            ".local/share/Steam/steamapps/shadercache/123/x",
            ".local/share/Steam/steamapps/appmanifest_123.acf",
        ],
    },
    Case {
        rule: "steam-workshop",
        has: &[".local/share/Steam/steamapps/workshop/content/123/x"],
        at: ".local/share/Steam/steamapps/workshop/content/123",
        name: "Workshop items · app 123",
        lacks: &[
            ".local/share/Steam/steamapps/workshop/content/123/x",
            ".local/share/Steam/steamapps/appmanifest_123.acf",
        ],
    },
    Case {
        rule: "steam-download",
        has: &[".local/share/Steam/steamapps/downloading/123/x"],
        at: ".local/share/Steam/steamapps/downloading/123",
        name: "Unfinished download · app 123",
        lacks: &[
            ".local/share/Steam/steamapps/downloading/123/x",
            ".local/share/Steam/steamapps/appmanifest_123.acf",
        ],
    },
    Case {
        rule: "steam-runners",
        has: &[".local/share/Steam/compatibilitytools.d/GE-Proton9/x"],
        at: ".local/share/Steam/compatibilitytools.d/GE-Proton9",
        name: "GE-Proton9",
        lacks: &[".local/share/Steam/other/GE-Proton9/x"],
    },
    Case {
        rule: "lutris-runners",
        has: &[".local/share/lutris/runners/wine/lutris-7/x"],
        at: ".local/share/lutris/runners/wine/lutris-7",
        name: "lutris-7",
        lacks: &[".local/share/lutris/games/x"],
    },
    Case {
        rule: "heroic-runners",
        has: &[".config/heroic/tools/proton/GE/x"],
        at: ".config/heroic/tools/proton/GE",
        name: "GE",
        lacks: &[".config/heroic/store/x"],
    },
    Case {
        rule: "bottles-runners",
        has: &[".local/share/bottles/runners/soda/x"],
        at: ".local/share/bottles/runners/soda",
        name: "soda",
        lacks: &[".local/share/bottles/bottles/x"],
    },
    Case {
        rule: "shaders",
        has: &[".cache/mesa_shader_cache/x"],
        at: ".cache/mesa_shader_cache",
        name: "Mesa shaders",
        lacks: &[".cache/mesa/x"],
    },
    Case {
        rule: "huggingface",
        has: &[".cache/huggingface/hub/models--org--name/x"],
        at: ".cache/huggingface/hub/models--org--name",
        name: "org/name",
        lacks: &[".cache/huggingface/token/x"],
    },
    Case {
        rule: "ollama",
        has: &[".ollama/models/x"],
        at: ".ollama/models",
        name: "Ollama models",
        lacks: &[".ollama/logs/x"],
    },
    Case {
        rule: "lm-studio",
        has: &[".lmstudio/models/pub/model/x"],
        at: ".lmstudio/models/pub/model",
        name: "pub/model",
        lacks: &[".lmstudio/models/x"],
    },
    Case {
        rule: "torch",
        has: &[".cache/torch/x"],
        at: ".cache/torch",
        name: "PyTorch hub",
        lacks: &[".cache/torchvision-not/x"],
    },
    Case {
        rule: "whisper",
        has: &[".cache/whisper/x"],
        at: ".cache/whisper",
        name: "Whisper",
        lacks: &[".cache/whisperx-not/x"],
    },
    Case {
        rule: "worktrees",
        has: &["repo/.claude/worktrees/wt1/x"],
        at: "repo/.claude/worktrees/wt1",
        name: "repo · wt1",
        lacks: &["repo/.claude/settings/x"],
    },
    Case {
        rule: "old-installers",
        has: &["Downloads/setup.iso@old"],
        at: "Downloads/setup.iso",
        name: "setup.iso",
        lacks: &["Downloads/setup.iso"],
    },
    Case {
        rule: "old-downloads",
        has: &["Downloads/notes.txt@old"],
        at: "Downloads/notes.txt",
        name: "notes.txt",
        lacks: &["Downloads/notes.txt"],
    },
    Case {
        rule: "libvirt",
        has: &[".local/share/libvirt/images/vm.qcow2"],
        at: ".local/share/libvirt/images/vm.qcow2",
        name: "vm.qcow2",
        lacks: &[".local/share/libvirt/qemu/vm.xml"],
    },
    Case {
        rule: "gnome-boxes",
        has: &[".local/share/gnome-boxes/images/box.img"],
        at: ".local/share/gnome-boxes/images/box.img",
        name: "box.img",
        lacks: &[".local/share/gnome-boxes/other/box.img"],
    },
    Case {
        rule: "virtualbox",
        has: &["VirtualBox VMs/Win/x"],
        at: "VirtualBox VMs/Win",
        name: "Win",
        lacks: &["VirtualBox/Win/x"],
    },
];

fn finds(root: &Path, files: &[&str]) -> Vec<Find> {
    for f in files {
        match f.strip_suffix(OLD) {
            Some(rel) => put(root, rel, 400),
            None => put(root, f, 0),
        }
    }
    let (tree, _) = Scan::start(vec![root.to_path_buf()]).finish();
    let ctx = Context {
        home: root.to_path_buf(),
        libraries: vec![root.join(STEAM)],
        now: Context::here(Vec::new()).now,
    };
    Rules::shipped().find(&tree, &ctx)
}

#[test]
fn every_rule_finds_what_it_looks_for_and_no_more() {
    let shipped: Vec<String> = Rules::shipped()
        .rules
        .iter()
        .map(|r| r.id.clone())
        .collect();
    for id in &shipped {
        assert!(
            CASES.iter().any(|c| c.rule == id),
            "rule “{id}” has no case"
        );
    }
    for case in CASES {
        let with = home(&format!("{}-with", case.rule));
        let found = finds(&with.0, case.has);
        let find = found
            .iter()
            .find(|f| f.rule == case.rule)
            .unwrap_or_else(|| {
                panic!(
                    "{}: nothing found in {:?}",
                    case.rule,
                    found.iter().map(|f| (&f.rule, &f.path)).collect::<Vec<_>>()
                )
            });
        assert_eq!(find.path, with.0.join(case.at), "{}", case.rule);
        assert_eq!(find.name, case.name, "{}", case.rule);
        assert!(find.bytes > 0);

        let without = home(&format!("{}-without", case.rule));
        let found = finds(&without.0, case.lacks);
        assert!(
            !found.iter().any(|f| f.rule == case.rule),
            "{}: found without it: {found:?}",
            case.rule
        );
    }
}

#[test]
fn a_named_rule_beats_the_general_one_and_an_outer_find_holds_what_is_inside() {
    let h = home("order");
    let found = finds(
        &h.0,
        &[
            ".cache/spotify/x",
            ".cache/pip/http/x",
            "app/target/x",
            "app/Cargo.toml",
            "app/target/debug/build/x",
        ],
    );
    let paths: Vec<_> = found
        .iter()
        .map(|f| {
            (
                f.rule.as_str(),
                f.path.strip_prefix(&h.0).unwrap().to_path_buf(),
            )
        })
        .collect();
    assert!(paths.contains(&("cache", ".cache/pip".into())), "{paths:?}");
    let inside = |outer: &str| {
        paths
            .iter()
            .any(|(_, p)| p.starts_with(outer) && p != Path::new(outer))
    };
    assert!(
        !inside(".cache/pip") && !inside("app/target"),
        "inside an outer find: {paths:?}"
    );
    let safe: Vec<_> = found
        .iter()
        .filter(|f| f.safe)
        .map(|f| f.rule.as_str())
        .collect();
    assert!(
        !safe.contains(&"rust-target"),
        "a build folder is worth a look, never safe"
    );
}
