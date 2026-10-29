//! Yours' Tag release…: a provider's folder committed and tagged `v<version>` through
//! gitoxide, as `git add -A`, `git commit` and `git tag` would (v1 D30).

use std::path::Path;

use gix::{object::tree::EntryKind, refs::transaction::PreviousValue};

/// What tagging did.
#[derive(Clone, Debug, PartialEq)]
pub enum Tagged {
    /// Tagged, and the folder's `origin` to push it to, if it has one.
    New { tag: String, remote: Option<String> },
    /// The tag was there already; nothing was committed.
    Already { tag: String },
}

/// Left out besides `.git` and the folder's `.gitignore`: what running or installing a provider
/// leaves in it.
const LEFT_OUT: [&str; 3] = ["__pycache__", ".isle-install.json", ".DS_Store"];

/// A version as a release names it: three numbers, a pre-release after a hyphen allowed.
pub fn is_release_version(version: &str) -> bool {
    let core = version.split_once('-').map_or(version, |(core, _)| core);
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Commits everything in `folder` (a repository made there if it has none) when it differs
/// from the last commit, then tags that commit `v<version>`. The index is left matching the
/// commit, so `git status` there reads clean.
pub fn tag_release(folder: &Path, version: &str) -> Result<Tagged, String> {
    if !is_release_version(version) {
        return Err("Set a version like 1.0.0 first".into());
    }
    let tag = format!("v{version}");
    let err = |e: &dyn std::fmt::Display| e.to_string();
    let repo = if folder.join(".git").exists() {
        gix::open(folder).map_err(|e| err(&e))?
    } else {
        gix::init(folder).map_err(|e| err(&e))?
    };
    if repo
        .try_find_reference(format!("refs/tags/{tag}").as_str())
        .map_err(|e| err(&e))?
        .is_some()
    {
        return Ok(Tagged::Already { tag });
    }
    let ignored = ignore_patterns(folder);
    let mut editor = repo.empty_tree().edit().map_err(|e| err(&e))?;
    add_dir(&repo, &mut editor, folder, "", &ignored)?;
    let tree = editor.write().map_err(|e| err(&e))?.detach();
    let head = repo.head_commit().ok();
    let target = match &head {
        Some(commit) if commit.tree_id().map_err(|e| err(&e))? == tree => commit.id,
        _ => {
            let (name, email) = identity(&repo);
            let who = gix::actor::Signature {
                name: name.into(),
                email: email.into(),
                time: gix::date::Time::now_local_or_utc(),
            };
            let mut time = gix::date::parse::TimeBuf::default();
            let who = who.to_ref(&mut time);
            let parents: Vec<gix::ObjectId> = head.iter().map(|c| c.id).collect();
            let id = repo
                .commit_as(who, who, "HEAD", &tag, tree, parents)
                .map_err(|e| err(&e))?
                .detach();
            let mut index = repo.index_from_tree(&tree).map_err(|e| err(&e))?;
            index.write(Default::default()).map_err(|e| err(&e))?;
            id
        }
    };
    repo.tag_reference(&tag, target, PreviousValue::MustNotExist)
        .map_err(|e| err(&e))?;
    let remote = repo.find_remote("origin").ok().and_then(|r| {
        r.url(gix::remote::Direction::Push)
            .map(|u| u.to_bstring().to_string())
    });
    Ok(Tagged::New { tag, remote })
}

/// The user's git identity, or Isle's as v1 used.
fn identity(repo: &gix::Repository) -> (String, String) {
    let configured = repo
        .committer()
        .and_then(Result::ok)
        .map(|s| (s.name.to_string(), s.email.to_string()));
    configured.unwrap_or_else(|| ("Isle".into(), "isle@localhost".into()))
}

/// The folder's `.gitignore` as plain names, `*.ext` and `name/`, matched at any depth.
fn ignore_patterns(folder: &Path) -> Vec<String> {
    std::fs::read_to_string(folder.join(".gitignore"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().trim_start_matches('/').trim_end_matches('/'))
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('!'))
        .map(str::to_owned)
        .collect()
}

fn left_out(name: &str, ignored: &[String]) -> bool {
    name == ".git"
        || LEFT_OUT.contains(&name)
        || ignored.iter().any(|p| match p.strip_prefix('*') {
            Some(suffix) => name.ends_with(suffix),
            None => p == name,
        })
}

fn add_dir(
    repo: &gix::Repository,
    editor: &mut gix::object::tree::Editor<'_>,
    dir: &Path,
    prefix: &str,
    ignored: &[String],
) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| e.to_string();
    for entry in std::fs::read_dir(dir).map_err(|e| err(&e))? {
        let entry = entry.map_err(|e| err(&e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if left_out(&name, ignored) {
            continue;
        }
        let path = format!("{prefix}{name}");
        let kind = entry.file_type().map_err(|e| err(&e))?;
        if kind.is_dir() {
            add_dir(repo, editor, &entry.path(), &format!("{path}/"), ignored)?;
            continue;
        }
        let (bytes, mode) = if kind.is_symlink() {
            let target = std::fs::read_link(entry.path()).map_err(|e| err(&e))?;
            (
                target.to_string_lossy().into_owned().into_bytes(),
                EntryKind::Link,
            )
        } else {
            use std::os::unix::fs::PermissionsExt;
            let executable =
                entry.metadata().map_err(|e| err(&e))?.permissions().mode() & 0o111 != 0;
            let bytes = std::fs::read(entry.path()).map_err(|e| err(&e))?;
            (
                bytes,
                if executable {
                    EntryKind::BlobExecutable
                } else {
                    EntryKind::Blob
                },
            )
        };
        let id = repo.write_blob(bytes).map_err(|e| err(&e))?.detach();
        editor
            .upsert(path.as_str(), mode, id)
            .map_err(|e| err(&e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    #[test]
    fn a_folder_is_committed_tagged_and_reads_clean() {
        let dir = std::env::temp_dir().join(format!("isle-release-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib/__pycache__")).unwrap();
        std::fs::write(dir.join("widget.json"), "{}").unwrap();
        std::fs::write(dir.join("lib/hn.py"), "print(1)").unwrap();
        std::fs::write(dir.join("lib/__pycache__/hn.pyc"), "x").unwrap();
        std::fs::write(dir.join("notes.log"), "x").unwrap();
        std::fs::write(dir.join(".gitignore"), "*.log\n").unwrap();

        assert_eq!(
            tag_release(&dir, "1.0"),
            Err("Set a version like 1.0.0 first".into())
        );
        assert_eq!(
            tag_release(&dir, "0.1.0"),
            Ok(Tagged::New {
                tag: "v0.1.0".into(),
                remote: None
            })
        );
        assert_eq!(
            tag_release(&dir, "0.1.0"),
            Ok(Tagged::Already {
                tag: "v0.1.0".into()
            })
        );
        let have_git = std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_ok();
        if have_git {
            assert_eq!(
                git(&dir, &["ls-tree", "-r", "--name-only", "v0.1.0"]),
                ".gitignore\nlib/hn.py\nwidget.json"
            );
            assert_eq!(
                git(&dir, &["status", "--porcelain", "--untracked-files=no"]),
                ""
            );
        }

        // Unchanged, a new version tags the same commit; changed, it commits first.
        std::fs::write(dir.join("widget.json"), "{\"v\":2}").unwrap();
        assert!(matches!(tag_release(&dir, "0.2.0"), Ok(Tagged::New { .. })));
        if have_git {
            assert_eq!(git(&dir, &["rev-list", "--count", "v0.2.0"]), "2");
            assert_eq!(git(&dir, &["log", "-1", "--format=%s", "v0.2.0"]), "v0.2.0");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
