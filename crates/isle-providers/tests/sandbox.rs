//! A provider reaches what it was granted and nothing else: files, hosts, buses, commands,
//! Isle's own places. Needs bwrap and xdg-dbus-proxy.

use std::{
    fs,
    path::{Path, PathBuf},
};

use isle_providers::{Data, Grant, Manifest, run::Runner};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("isle-providers-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn runner(dir: &Path) -> Runner {
    let mut r = Runner::new(dir.join("sockets"), dir.join("states"));
    r.init = PathBuf::from(env!("CARGO_BIN_EXE_isle-provider-init"));
    r
}

/// A provider in `dir/name` running `sh -c script`, answering text.
fn provider(dir: &Path, name: &str, script: &str, permissions: &str) -> (Manifest, PathBuf) {
    let folder = dir.join(name);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("run.sh"), format!("#!/bin/sh\n{script}\n")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(folder.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        folder.join("widget.json"),
        format!(
            r#"{{"id":"{name}","name":"{name}","version":"1.0.0","sizes":["2x1"],"default":"2x1",
            "template":"text","exec":["./run.sh"],"interval":60,"permissions":{{{permissions}}}}}"#
        ),
    )
    .unwrap();
    (Manifest::load(&folder).unwrap(), folder)
}

fn text(outcome: &isle_providers::run::Outcome) -> String {
    match &outcome.answer {
        Ok(Data::Text { text }) => text.clone(),
        other => format!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn files_are_reached_only_as_granted() {
    let dir = Dir::new("files");
    let secret = dir.0.join("secret");
    fs::create_dir_all(&secret).unwrap();
    fs::write(secret.join("note"), "hidden\n").unwrap();
    let target = secret.to_string_lossy().into_owned();
    let script = format!(
        r#"if read x < {target}/note 2>/dev/null; then r=read; else r=none; fi
if (echo x > {target}/new) 2>/dev/null; then w=wrote; else w=refused; fi
echo "{{\"text\":\"$r $w\"}}""#
    );
    let (m, folder) = provider(&dir.0, "files", &script, &format!(r#""change":["{target}"]"#));
    let r = runner(&dir.0);

    let none = r.once(&m, &folder, &[], "{}").await;
    assert_eq!(text(&none), "none refused", "declared but not granted: {none:?}");
    let read = r.once(&m, &folder, &[Grant::Read(target.clone())], "{}").await;
    assert_eq!(text(&read), "none refused", "a grant it did not declare does nothing");
    let change = r.once(&m, &folder, &[Grant::Change(target.clone())], "{}").await;
    assert_eq!(text(&change), "read wrote");
    assert!(secret.join("new").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn commands_are_on_the_path_only_as_granted() {
    let dir = Dir::new("run");
    let script = r#"if command -v git >/dev/null; then r=git; else r=nogit; fi; echo "{\"text\":\"$r\"}""#;
    let (m, folder) = provider(&dir.0, "run", script, r#""run":["git"]"#);
    let r = runner(&dir.0);
    assert_eq!(text(&r.once(&m, &folder, &[], "{}").await), "nogit");
    assert_eq!(text(&r.once(&m, &folder, &[Grant::Run("git".into())], "{}").await), "git");
}

#[tokio::test(flavor = "multi_thread")]
async fn isles_own_settings_stay_hidden_inside_a_granted_home() {
    let dir = Dir::new("guard");
    let home = dir.0.join("home");
    fs::create_dir_all(home.join(".config/isle")).unwrap();
    fs::write(home.join(".config/isle/settings.kdl"), "x").unwrap();
    fs::write(home.join("mine"), "x").unwrap();
    let h = home.to_string_lossy().into_owned();
    let script = format!(
        r#"if [ -e {h}/.config/isle/settings.kdl ]; then s=seen; else s=hidden; fi
if [ -e {h}/mine ]; then m=mine; else m=nothing; fi
echo "{{\"text\":\"$s $m\"}}""#
    );
    let (m, folder) = provider(&dir.0, "guard", &script, r#""change":["~"]"#);
    let mut r = runner(&dir.0);
    r.home = home.clone();
    let out = r.once(&m, &folder, &[Grant::Change("~".into())], "{}").await;
    assert_eq!(text(&out), "hidden mine", "{out:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn hosts_are_reached_through_the_proxy_only_as_granted() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let dir = Dir::new("net");
    // A web server on the host's loopback, which the sandbox cannot see but the proxy can.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf).await;
            let _ = s
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"text\":\"web\"}\n")
                .await;
        }
    });
    let script = format!(
        r#"curl -s --max-time 5 http://localhost:{port}/ || echo '{{"text":"no network"}}'"#
    );
    let (m, folder) = provider(&dir.0, "net", &script, r#""connect":["localhost"],"run":["curl"]"#);
    let mut r = runner(&dir.0);
    r.ports = vec![port];
    let curl = Grant::Run("curl".into());

    let none = r.once(&m, &folder, &[curl.clone()], "{}").await;
    assert_eq!(text(&none), "no network");
    let granted = r.once(&m, &folder, &[curl.clone(), Grant::Connect("localhost".into())], "{}").await;
    assert_eq!(text(&granted), "web", "{granted:?}");

    // Another host is refused by the proxy and named.
    let other = format!(r#"curl -s --max-time 5 http://example.com:{port}/ || echo '{{"text":"refused"}}'"#);
    let (m2, folder2) = provider(&dir.0, "other", &other, r#""connect":["localhost"],"run":["curl"]"#);
    let refused = r.once(&m2, &folder2, &[curl, Grant::Connect("localhost".into())], "{}").await;
    assert_eq!(refused.denied, vec!["example.com".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn buses_are_reached_only_by_the_names_granted() {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        return;
    }
    let dir = Dir::new("bus");
    let script = r#"if dbus-send --session --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.GetId >/dev/null 2>&1; then a=yes; else a=no; fi
if dbus-send --session --print-reply --dest=org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.DBus.Peer.Ping >/dev/null 2>&1; then b=yes; else b=no; fi
echo "{\"text\":\"$a $b\"}""#;
    let (m, folder) = provider(&dir.0, "bus", script, r#""bus":["org.freedesktop.DBus"],"run":["dbus-send"]"#);
    let r = runner(&dir.0);
    let send = Grant::Run("dbus-send".into());
    assert_eq!(text(&r.once(&m, &folder, &[send.clone()], "{}").await), "no no");
    let out = r.once(&m, &folder, &[send, Grant::Bus("org.freedesktop.DBus".into())], "{}").await;
    assert_eq!(text(&out), "yes no", "{out:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn input_failures_limits_and_streams() {
    let dir = Dir::new("misc");
    let r = runner(&dir.0);
    let (echo, f1) = provider(&dir.0, "echo", r#"read line; n=${#line}; echo "{\"text\":\"$n\"}""#, "");
    let out = r.once(&echo, &f1, &[], "{\"size\":\"2x1\"}\n").await;
    assert_eq!(text(&out), "14", "settings and size come on stdin: {out:?}");

    let (fail, f2) = provider(&dir.0, "fail", "echo 'The key is wrong' >&2; exit 3", "");
    assert_eq!(r.once(&fail, &f2, &[], "{}").await.answer.unwrap_err(), "The key is wrong");

    let (slow, f3) = provider(&dir.0, "slow", "while :; do :; done", "");
    let mut quick = r.clone();
    quick.limit = std::time::Duration::from_millis(500);
    assert_eq!(quick.once(&slow, &f3, &[], "{}").await.answer.unwrap_err(), "slow didn't answer. Try again later.");

    let (stream, f4) = provider(&dir.0, "stream", r#"for i in 1 2 3; do echo "{\"text\":\"$i\"}"; done; exit 2"#, "");
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let ended = r.stream(&stream, &f4, &[], "{}", tx).await;
    let mut got = Vec::new();
    while let Ok(Ok(Data::Text { text })) = rx.try_recv() {
        got.push(text);
    }
    assert_eq!(got, ["1", "2", "3"]);
    assert_eq!(ended.answer.unwrap_err(), "It stopped with status 2");
}
