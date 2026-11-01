//! What Keychain remembers in its `isle-state` domain (`p19c-keychain.md` choice 24): the last
//! sidebar row, the list's width and the generator's recipe. No secret is kept.

use isle_secrets::generator::Recipe;
use isle_state::{Domain, StateDir};
use serde::{Deserialize, Serialize};

use crate::model::Row;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Remembered {
    pub row: Row,
    pub list_width: f32,
    pub recipe: Recipe,
}

impl Default for Remembered {
    fn default() -> Self {
        Self {
            row: Row::All,
            list_width: 300.,
            recipe: Recipe::default(),
        }
    }
}

impl Domain for Remembered {
    const NAME: &'static str = "keychain";
    const VERSION: u32 = 1;
}

pub fn load() -> Remembered {
    let (data, loaded) = StateDir::new(StateDir::default_dir()).load::<Remembered>();
    if !matches!(
        loaded,
        isle_state::Loaded::Read | isle_state::Loaded::Missing
    ) {
        tracing::warn!(?loaded, "Keychain's state");
    }
    data
}

pub fn save(data: &Remembered) {
    if let Err(err) = StateDir::new(StateDir::default_dir()).save(data) {
        tracing::warn!("Keychain's state could not be saved: {err}");
    }
}
