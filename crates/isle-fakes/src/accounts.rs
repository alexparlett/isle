//! `org.freedesktop.Accounts` with its cached users, as the greeter reads them.

use zbus::{Connection, zvariant::OwnedObjectPath};

const PATH: &str = "/org/freedesktop/Accounts";

#[derive(Clone, Debug)]
pub struct User {
    pub uid: u64,
    pub name: String,
    pub real_name: String,
    pub icon_file: String,
    pub system_account: bool,
    pub locked: bool,
}

impl User {
    pub fn new(uid: u64, name: &str, real_name: &str) -> Self {
        Self {
            uid,
            name: name.into(),
            real_name: real_name.into(),
            icon_file: String::new(),
            system_account: false,
            locked: false,
        }
    }

    fn path(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(format!("{PATH}/User{}", self.uid)).unwrap()
    }
}

struct Manager {
    users: Vec<OwnedObjectPath>,
}

#[zbus::interface(name = "org.freedesktop.Accounts")]
impl Manager {
    fn list_cached_users(&self) -> Vec<OwnedObjectPath> {
        self.users.clone()
    }
}

struct UserIface(User);

#[zbus::interface(name = "org.freedesktop.Accounts.User")]
impl UserIface {
    #[zbus(property)]
    fn user_name(&self) -> String {
        self.0.name.clone()
    }
    #[zbus(property)]
    fn real_name(&self) -> String {
        self.0.real_name.clone()
    }
    #[zbus(property)]
    fn icon_file(&self) -> String {
        self.0.icon_file.clone()
    }
    #[zbus(property)]
    fn system_account(&self) -> bool {
        self.0.system_account
    }
    #[zbus(property)]
    fn locked(&self) -> bool {
        self.0.locked
    }
}

pub struct Accounts {
    _conn: Connection,
}

impl Accounts {
    pub async fn start(address: &str, users: Vec<User>) -> zbus::Result<Self> {
        let mut builder = zbus::connection::Builder::address(address)?.serve_at(
            PATH,
            Manager {
                users: users.iter().map(User::path).collect(),
            },
        )?;
        for user in users {
            builder = builder.serve_at(user.path(), UserIface(user))?;
        }
        Ok(Self {
            _conn: builder.name("org.freedesktop.Accounts")?.build().await?,
        })
    }
}
