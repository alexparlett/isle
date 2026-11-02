//! The greeter mode: the compositor run by greetd as its greeter, the lock and nothing else
//! (`p14b-greeter.md`).

use std::{
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};

use isle_apps::sessions::Session;
use isle_auth::greetd::{Greetd, Step};
use isle_state::StateDir;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{info, warn};
use zeroize::{Zeroize, Zeroizing};

use crate::{state::State, worker::ToLoop};

static ON: OnceLock<bool> = OnceLock::new();

/// This many starts within `FALLBACK_WINDOW` seconds, none of them drawing a face, and the
/// greeter gives way to tuigreet.
const FALLBACK_STARTS: usize = 3;
const FALLBACK_WINDOW: u64 = 60;

/// A greeter started this soon after handing greetd a session finds that it failed.
const FAILED_WITHIN: u64 = 10;

/// tuigreet as P14a runs it (`p14a-session.md` choice 2).
const TUIGREET: &[&str] = &[
    "--time",
    "--remember",
    "--remember-session",
    "--sessions",
    "/usr/share/wayland-sessions",
];

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn memory_dir() -> StateDir {
    StateDir::new(StateDir::default_dir())
}

/// Counts this start; after `FALLBACK_STARTS` faceless ones in a row, runs tuigreet in the
/// greeter's place, so a greeter that cannot draw never keeps anyone out.
pub fn count_start() {
    let dir = memory_dir();
    let mut memory: isle_state::domains::Greeter = dir.load().0;
    let now = unix_now();
    memory
        .faceless_starts
        .retain(|t| now.saturating_sub(*t) < FALLBACK_WINDOW);
    if memory.faceless_starts.len() >= FALLBACK_STARTS {
        memory.faceless_starts.clear();
        let _ = dir.save(&memory);
        fall_back();
    }
    memory.faceless_starts.push(now);
    if let Err(err) = dir.save(&memory) {
        warn!("the greeter's start was not counted: {err}");
    }
}

fn fall_back() -> ! {
    use std::os::unix::process::CommandExt;
    #[cfg(feature = "test-hooks")]
    let program = std::env::var("ISLE_GREETER_FALLBACK").unwrap_or_else(|_| "tuigreet".into());
    #[cfg(not(feature = "test-hooks"))]
    let program = "tuigreet".to_owned();
    tracing::error!(
        "the greeter started {FALLBACK_STARTS} times without drawing its face; tuigreet stands in"
    );
    let err = std::process::Command::new(&program).args(TUIGREET).exec();
    tracing::error!("{program} did not start: {err}");
    std::process::exit(1);
}

/// Whether this process is the greeter; fixed at start.
pub fn on() -> bool {
    ON.get().copied().unwrap_or(false)
}

/// Sets the mode and, before any thread starts, the greeter's own directories where greetd
/// left none: its caches in `/var/cache/isle-greeter`, its state in `/var/lib/isle-greeter`
/// (both the package's, owned by `greeter`).
pub fn start(greeter: bool) {
    let _ = ON.set(greeter);
    if !greeter {
        return;
    }
    for (key, dir) in [
        ("XDG_CACHE_HOME", "/var/cache/isle-greeter"),
        ("XDG_STATE_HOME", "/var/lib/isle-greeter"),
    ] {
        if std::env::var_os(key).is_none() {
            // SAFETY: called from `run` before any thread starts.
            unsafe { std::env::set_var(key, dir) };
        }
    }
}

/// A user the greeter lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub login: String,
    pub name: String,
    pub picture: Option<PathBuf>,
}

/// What the loop asks of the conversation on the worker.
enum Talk {
    Create(String),
    Answer(Option<Zeroizing<String>>),
    Start(Vec<String>, Vec<String>),
    Cancel,
}

/// What the conversation hears back.
enum Heard {
    Step(Step),
    Started(Result<(), String>),
    Lost(String),
}

#[derive(Default)]
pub struct Greeter {
    pub users: Vec<User>,
    pub sessions: Vec<Session>,
    /// The user being signed in; with one user, always them.
    pub chosen: Option<usize>,
    /// The list's keyboard selection.
    pub selected: usize,
    pub session: usize,
    pub menu_open: bool,
    /// PAM's question: its text and whether it is secret.
    pub prompt: Option<(String, bool)>,
    /// The last of PAM's info lines.
    pub said: Option<String>,
    /// A session that did not start, by name.
    pub failed: Option<String>,
    memory: isle_state::domains::Greeter,
    talk: Option<UnboundedSender<(u64, Talk)>>,
    /// Counts each user chosen, so an answer about an earlier one is dropped.
    generation: u64,
    /// The face has been drawn since this start.
    drew: bool,
    /// In a session: the look's write waiting for changes to settle.
    pub look_timer: Option<smithay::reexports::calloop::RegistrationToken>,
    /// In a session: there is no directory to publish the look to, and that was said.
    pub look_unpublished: bool,
}

/// `/etc/login.defs`' range of user ids, and passwd's users in it with a login shell.
fn passwd_users(passwd: &str, login_defs: &str) -> Vec<User> {
    let defs = |key: &str, fallback: u32| {
        login_defs
            .lines()
            .filter_map(|l| l.trim().strip_prefix(key))
            .find_map(|rest| rest.trim().parse().ok())
            .unwrap_or(fallback)
    };
    let (min, max) = (defs("UID_MIN", 1000), defs("UID_MAX", 60000));
    passwd
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(':').collect();
            let [login, _, uid, _, gecos, _, shell] = fields[..] else {
                return None;
            };
            let uid: u32 = uid.parse().ok()?;
            let no_login = shell.ends_with("/nologin") || shell.ends_with("/false");
            ((min..=max).contains(&uid) && !no_login).then(|| User {
                login: login.to_owned(),
                name: Some(gecos.split(',').next().unwrap_or("").trim())
                    .filter(|n| !n.is_empty())
                    .unwrap_or(login)
                    .to_owned(),
                picture: picture_of(login),
            })
        })
        .collect()
}

fn picture_of(login: &str) -> Option<PathBuf> {
    let path = PathBuf::from("/var/lib/AccountsService/icons").join(login);
    path.is_file().then_some(path)
}

/// AccountsService's users that may sign in, by name.
async fn accounts_users() -> zbus::Result<Vec<User>> {
    let bus = zbus::Connection::system().await?;
    let manager = zbus::Proxy::new(
        &bus,
        "org.freedesktop.Accounts",
        "/org/freedesktop/Accounts",
        "org.freedesktop.Accounts",
    )
    .await?;
    let paths: Vec<zbus::zvariant::OwnedObjectPath> = manager.call("ListCachedUsers", &()).await?;
    let mut users = Vec::new();
    for path in paths {
        let user = zbus::Proxy::new(
            &bus,
            "org.freedesktop.Accounts",
            path,
            "org.freedesktop.Accounts.User",
        )
        .await?;
        if user.get_property::<bool>("SystemAccount").await?
            || user.get_property::<bool>("Locked").await?
        {
            continue;
        }
        let login: String = user.get_property("UserName").await?;
        let real: String = user.get_property("RealName").await?;
        let icon: String = user.get_property("IconFile").await?;
        users.push(User {
            name: if real.trim().is_empty() {
                login.clone()
            } else {
                real.trim().to_owned()
            },
            picture: Some(PathBuf::from(icon))
                .filter(|p| p.is_file())
                .or_else(|| picture_of(&login)),
            login,
        });
    }
    users.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(users)
}

/// The users from AccountsService, else passwd's when it does not answer within a second.
async fn load_users() -> Vec<User> {
    match tokio::time::timeout(Duration::from_secs(1), accounts_users()).await {
        Ok(Ok(users)) if !users.is_empty() => users,
        outcome => {
            if let Ok(Err(err)) = outcome {
                warn!("AccountsService did not list the users: {err}");
            }
            passwd_users(
                &std::fs::read_to_string("/etc/passwd").unwrap_or_default(),
                &std::fs::read_to_string("/etc/login.defs").unwrap_or_default(),
            )
        }
    }
}

/// Lists the users again whenever AccountsService comes onto the system bus.
async fn follow_accounts(to_loop: ToLoop) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use futures_util::StreamExt;
    let bus = zbus::Connection::system().await?;
    let dbus = zbus::fdo::DBusProxy::new(&bus).await?;
    let mut owners = dbus
        .receive_name_owner_changed_with_args(&[(0, "org.freedesktop.Accounts")])
        .await?;
    while let Some(signal) = owners.next().await {
        if signal.args()?.new_owner().is_some() {
            let users = load_users().await;
            to_loop.post(move |state| state.greeter_users(users));
        }
    }
    Ok(())
}

/// The conversation with greetd, one request at a time, each answer posted to the loop
/// with the generation it was asked in.
async fn converse(mut talks: UnboundedReceiver<(u64, Talk)>, to_loop: ToLoop) {
    let mut greetd = match Greetd::from_env().await {
        Ok(greetd) => greetd,
        Err(err) => {
            let why = format!("greetd is not reachable: {err}");
            to_loop.post(move |state| state.greeter_heard(0, Heard::Lost(why)));
            return;
        }
    };
    while let Some((generation, talk)) = talks.recv().await {
        let heard = match talk {
            Talk::Create(user) => greetd.create(&user).await.map(Heard::Step),
            Talk::Answer(answer) => greetd
                .answer(answer.as_ref().map(|a| a.as_str()))
                .await
                .map(Heard::Step),
            Talk::Start(cmd, env) => greetd.start(&cmd, &env).await.map(Heard::Started),
            Talk::Cancel => {
                let _ = greetd.cancel().await;
                continue;
            }
        };
        let heard = heard.unwrap_or_else(|err| Heard::Lost(format!("greetd: {err}")));
        let lost = matches!(heard, Heard::Lost(_));
        to_loop.post(move |state| state.greeter_heard(generation, heard));
        if lost {
            return;
        }
    }
}

/// PAM's usual password question reads as the lock's placeholder.
fn placeholder(prompt: &str) -> Option<String> {
    let trimmed = prompt.trim().trim_end_matches(':').trim();
    (!trimmed.eq_ignore_ascii_case("password") && !trimmed.is_empty()).then(|| trimmed.to_owned())
}

impl State {
    /// The greeter's users, sessions and conversation, at its start.
    pub fn greeter_start(&mut self) {
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone())
        else {
            return;
        };
        let greeter = &mut self.isle.greeter;
        greeter.memory = memory_dir().load().0;
        if let Some((at, name)) = greeter.memory.started.take() {
            if unix_now().saturating_sub(at) <= FAILED_WITHIN {
                warn!(session = name, "the session ended as it started");
                greeter.failed = Some(name);
            }
            let _ = memory_dir().save(&greeter.memory);
        }
        let last = greeter.memory.last_user.clone();
        self.greeter_take_look(last.as_deref());
        let greeter = &mut self.isle.greeter;
        greeter.sessions = isle_apps::sessions::sessions(&isle_apps::sessions::dirs());
        if greeter.sessions.is_empty() {
            warn!("no session entries in wayland-sessions");
        }
        let (send, talks) = tokio::sync::mpsc::unbounded_channel();
        greeter.talk = Some(send);
        runtime.spawn(converse(talks, to_loop.clone()));
        let loop_u = to_loop.clone();
        runtime.spawn(async move {
            let users = load_users().await;
            loop_u.post(move |state| state.greeter_users(users));
        });
        runtime.spawn(crate::worker::supervise("accounts-watch", move || {
            follow_accounts(to_loop.clone())
        }));
    }

    /// The users listed, or listed again: one is chosen when there is only one.
    pub fn greeter_users(&mut self, users: Vec<User>) {
        let greeter = &mut self.isle.greeter;
        if greeter.users == users {
            return;
        }
        info!(users = users.len(), "the greeter's users");
        let last = greeter.memory.last_user.clone();
        greeter.selected = users
            .iter()
            .position(|u| Some(&u.login) == last.as_ref())
            .unwrap_or(0);
        greeter.users = users;
        greeter.chosen = None;
        let shown = greeter.users.get(greeter.selected).map(|u| u.login.clone());
        self.greeter_take_look(shown.as_deref());
        let greeter = &mut self.isle.greeter;
        greeter.prompt = None;
        greeter.said = None;
        greeter.generation += 1;
        if self.isle.greeter.users.len() == 1 {
            self.greeter_choose(0);
        } else {
            self.greeter_talk(Talk::Cancel);
            self.refresh_lock_face();
        }
    }

    /// `index`'s user chosen: their last session, and greetd asked to begin.
    pub fn greeter_choose(&mut self, index: usize) {
        let greeter = &mut self.isle.greeter;
        let Some(user) = greeter.users.get(index) else {
            return;
        };
        let login = user.login.clone();
        self.greeter_take_look(Some(&login));
        let greeter = &mut self.isle.greeter;
        let last = greeter.memory.sessions.get(&login).cloned();
        greeter.session = greeter
            .sessions
            .iter()
            .position(|s| Some(&s.id) == last.as_ref())
            .or_else(|| greeter.sessions.iter().position(|s| s.id == "isle"))
            .unwrap_or(0);
        greeter.chosen = Some(index);
        greeter.selected = index;
        greeter.prompt = None;
        greeter.said = None;
        greeter.generation += 1;
        self.clear_attempts();
        self.greeter_talk(Talk::Cancel);
        self.greeter_talk(Talk::Create(login));
        self.refresh_lock_face();
    }

    /// Escape with nothing typed: back to the list, when there is one.
    pub fn greeter_back(&mut self) -> bool {
        let greeter = &mut self.isle.greeter;
        if greeter.users.len() < 2 || greeter.chosen.is_none() {
            return false;
        }
        greeter.chosen = None;
        greeter.prompt = None;
        greeter.said = None;
        greeter.generation += 1;
        self.clear_attempts();
        self.greeter_talk(Talk::Cancel);
        self.refresh_lock_face();
        true
    }

    /// The list's keys while no user is chosen: Left, Right and Return.
    pub fn greeter_list_key(&mut self, sym: smithay::input::keyboard::Keysym) -> bool {
        use smithay::input::keyboard::keysyms as k;
        let greeter = &mut self.isle.greeter;
        if greeter.chosen.is_some() || greeter.users.len() < 2 {
            return false;
        }
        let last = greeter.users.len() - 1;
        match sym.raw() {
            k::KEY_Left | k::KEY_Right => {
                greeter.selected = if sym.raw() == k::KEY_Left {
                    greeter.selected.saturating_sub(1)
                } else {
                    (greeter.selected + 1).min(last)
                };
                let shown = greeter.users[greeter.selected].login.clone();
                self.greeter_take_look(Some(&shown));
            }
            k::KEY_Return | k::KEY_KP_Enter | k::KEY_space => {
                let selected = greeter.selected;
                self.greeter_choose(selected);
                return true;
            }
            _ => return true,
        }
        self.refresh_lock_face();
        true
    }

    pub fn greeter_session_menu(&mut self, open: bool) {
        self.isle.greeter.menu_open = open;
        self.refresh_lock_face();
    }

    pub fn greeter_choose_session(&mut self, index: usize) {
        let greeter = &mut self.isle.greeter;
        if index < greeter.sessions.len() {
            greeter.session = index;
            greeter.failed = None;
        }
        greeter.menu_open = false;
        self.refresh_lock_face();
    }

    /// Return in the field: the answer to PAM's question goes to greetd.
    pub fn greeter_submit(&mut self, answer: Zeroizing<String>) -> bool {
        let greeter = &self.isle.greeter;
        if greeter.chosen.is_none() || greeter.prompt.is_none() {
            return false;
        }
        self.greeter_talk(Talk::Answer(Some(answer)));
        true
    }

    fn greeter_talk(&mut self, talk: Talk) {
        let greeter = &self.isle.greeter;
        if let Some(send) = &greeter.talk
            && send.send((greeter.generation, talk)).is_err()
        {
            self.isle.greeter.talk = None;
        }
    }

    fn clear_attempts(&mut self) {
        let lock = &mut self.isle.lock;
        lock.field.zeroize();
        lock.pending = false;
        lock.status = crate::lock::Status::None;
        lock.failures = 0;
        lock.locked_out_until = None;
    }

    fn greeter_heard(&mut self, generation: u64, heard: Heard) {
        if let Heard::Lost(why) = heard {
            warn!("{why}");
            self.isle.greeter.talk = None;
            self.isle.lock.pending = false;
            self.isle.lock.status = crate::lock::Status::Error;
            self.refresh_lock_face();
            return;
        }
        if generation != self.isle.greeter.generation {
            return;
        }
        let Some(login) = self
            .isle
            .greeter
            .chosen
            .and_then(|i| self.isle.greeter.users.get(i))
            .map(|u| u.login.clone())
        else {
            return;
        };
        match heard {
            Heard::Step(Step::Ask {
                prompt,
                secret,
                messages,
            }) => {
                let greeter = &mut self.isle.greeter;
                greeter.prompt = Some((prompt, secret));
                if let Some(last) = messages.last() {
                    greeter.said = Some(last.clone());
                }
                let lock = &mut self.isle.lock;
                lock.pending = false;
                lock.field.zeroize();
            }
            Heard::Step(Step::Ready { .. }) => {
                let Some(session) = self.isle.greeter.sessions.get(self.isle.greeter.session)
                else {
                    self.isle.lock.pending = false;
                    self.isle.lock.status = crate::lock::Status::Error;
                    warn!("no session to start");
                    self.refresh_lock_face();
                    return;
                };
                info!(session = session.id, "signed in; starting the session");
                let (cmd, env) = (session.exec.clone(), session.env());
                self.greeter_talk(Talk::Start(cmd, env));
                return;
            }
            Heard::Step(step @ Step::Refused { .. }) => {
                let lock = &mut self.isle.lock;
                lock.pending = false;
                lock.field.zeroize();
                match step.lockout() {
                    Some(minutes) => {
                        info!(minutes, "the account is locked out");
                        lock.status = crate::lock::Status::LockedOut;
                        lock.locked_out_until =
                            Some(Instant::now() + Duration::from_secs(minutes as u64 * 60));
                    }
                    None => {
                        lock.failures += 1;
                        lock.status = crate::lock::Status::Wrong;
                        info!(failures = lock.failures, "a wrong password");
                        self.lock_wrong();
                    }
                }
                self.isle.greeter.prompt = None;
                self.greeter_talk(Talk::Create(login));
            }
            Heard::Step(Step::Broken(why)) => {
                warn!("greetd could not go on: {why}");
                self.isle.lock.pending = false;
                self.isle.lock.field.zeroize();
                self.isle.lock.status = crate::lock::Status::Error;
                self.isle.greeter.prompt = None;
                self.greeter_talk(Talk::Create(login));
            }
            Heard::Started(Ok(())) => {
                let greeter = &mut self.isle.greeter;
                let session = &greeter.sessions[greeter.session];
                greeter.memory.started = Some((unix_now(), session.name.clone()));
                greeter
                    .memory
                    .sessions
                    .insert(login.clone(), session.id.clone());
                greeter.memory.last_user = Some(login);
                if let Err(err) = memory_dir().save(&greeter.memory) {
                    warn!("the greeter's memory was not kept: {err}");
                }
                info!("greetd starts the session once the greeter exits");
                // The face goes, the backdrop stays for its last frame, and the greeter ends.
                self.hide_lock_face();
                self.isle.queue_redraw_all();
                let wait = Duration::from_millis(
                    self.isle.shell.theme().tokens.motion_exit_fade_ms as u64,
                );
                let _ = self.isle.loop_handle.insert_source(
                    smithay::reexports::calloop::timer::Timer::from_duration(wait),
                    |_, _, state| {
                        state.isle.loop_signal.stop();
                        smithay::reexports::calloop::timer::TimeoutAction::Drop
                    },
                );
                return;
            }
            Heard::Started(Err(why)) => {
                warn!("greetd did not start the session: {why}");
                let greeter = &mut self.isle.greeter;
                greeter.failed = greeter
                    .sessions
                    .get(greeter.session)
                    .map(|s| s.name.clone());
                self.isle.lock.pending = false;
                self.greeter_talk(Talk::Create(login));
            }
            Heard::Lost(_) => {}
        }
        self.refresh_lock_face();
    }

    /// A frame drawn on the face's screen: this start counts as one that worked.
    pub fn greeter_drew(&mut self, output: &smithay::output::Output) {
        if self.isle.greeter.drew || self.isle.lock.face.output.as_ref() != Some(&output.name()) {
            return;
        }
        self.isle.greeter.drew = true;
        let dir = memory_dir();
        let mut memory: isle_state::domains::Greeter = dir.load().0;
        memory.faceless_starts.clear();
        if let Err(err) = dir.save(&memory) {
            warn!("the greeter's start was not cleared: {err}");
        }
        self.isle.greeter.memory.faceless_starts.clear();
    }

    /// The face's greeter part.
    pub fn greeter_face(&self) -> Option<isle_shell::lock::Greeter> {
        if !on() {
            return None;
        }
        let greeter = &self.isle.greeter;
        let clear = greeter
            .prompt
            .as_ref()
            .filter(|(_, secret)| !secret)
            .map(|_| self.isle.lock.field.as_str().to_owned());
        Some(isle_shell::lock::Greeter {
            users: greeter
                .users
                .iter()
                .map(|u| isle_shell::lock::GreeterUser {
                    name: u.name.clone(),
                    picture: u.picture.clone(),
                })
                .collect(),
            chosen: greeter.chosen.is_some(),
            selected: greeter.selected,
            sessions: greeter.sessions.iter().map(|s| s.name.clone()).collect(),
            session: greeter.session,
            menu_open: greeter.menu_open,
            placeholder: greeter.prompt.as_ref().and_then(|(p, _)| placeholder(p)),
            clear,
        })
    }

    /// The chosen user, for the face's name and picture.
    pub fn greeter_user(&self) -> Option<&User> {
        let greeter = &self.isle.greeter;
        greeter.chosen.and_then(|i| greeter.users.get(i))
    }

    /// The greeter's status line, when it has its own say.
    pub fn greeter_line(&self) -> Option<isle_shell::lock::Line> {
        let greeter = &self.isle.greeter;
        if let Some(name) = &greeter.failed {
            return Some(isle_shell::lock::Line::Stopped(format!(
                "Couldn't start {name}. Choose another session and try again."
            )));
        }
        greeter.said.clone().map(isle_shell::lock::Line::Said)
    }

    /// Restart or Shut Down confirmed at the greeter: straight to logind.
    pub fn greeter_power(&mut self, restart: bool) {
        let Some(runtime) = self.isle.worker.clone() else {
            return;
        };
        runtime.spawn(async move {
            let method = if restart { "Reboot" } else { "PowerOff" };
            let result: zbus::Result<()> = async {
                let bus = zbus::Connection::system().await?;
                bus.call_method(
                    Some("org.freedesktop.login1"),
                    "/org/freedesktop/login1",
                    Some("org.freedesktop.login1.Manager"),
                    method,
                    &(false,),
                )
                .await?;
                Ok(())
            }
            .await;
            if let Err(err) = result {
                warn!("logind refused {method}: {err}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwd_gives_people_with_a_shell_in_login_defs_range() {
        let passwd = "root:x:0:0::/root:/usr/bin/bash\n\
            greeter:x:951:951:greetd greeter user:/:/bin/bash\n\
            alex:x:1000:1000:Alex Example,,,:/home/alex:/usr/bin/fish\n\
            sam:x:1001:1001::/home/sam:/bin/bash\n\
            svc:x:1002:1002:Service:/:/usr/bin/nologin\n\
            nobody:x:65534:65534:Nobody:/:/usr/bin/nologin\n";
        let users = passwd_users(passwd, "UID_MIN 1000\nUID_MAX 60000\n");
        let names: Vec<_> = users
            .iter()
            .map(|u| (u.login.as_str(), u.name.as_str()))
            .collect();
        assert_eq!(names, [("alex", "Alex Example"), ("sam", "sam")]);
    }

    #[test]
    fn pams_password_question_is_the_lock_s_placeholder() {
        assert_eq!(placeholder("Password: "), None);
        assert_eq!(placeholder("password:"), None);
        assert_eq!(placeholder("New password: "), Some("New password".into()));
    }
}
