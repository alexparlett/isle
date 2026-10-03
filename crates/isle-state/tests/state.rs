use std::{fs, path::PathBuf};

use isle_state::{Loaded, StateDir, domains::Crashes};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("isle-state-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_domain_round_trips_and_starts_empty_when_missing() {
    let dir = Dir::new("round");
    let state = StateDir::new(&dir.0);
    assert_eq!(
        state.load::<Crashes>(),
        (Crashes::default(), Loaded::Missing)
    );
    let crashes = Crashes {
        quick_deaths: vec![1, 2, 3],
    };
    state.save(&crashes).unwrap();
    assert_eq!(state.load::<Crashes>(), (crashes, Loaded::Read));
    assert!(
        fs::read_to_string(state.path::<Crashes>())
            .unwrap()
            .contains("\"version\": 1")
    );
}

#[test]
fn a_damaged_file_is_moved_aside_and_its_domain_starts_empty() {
    let dir = Dir::new("damaged");
    let state = StateDir::new(&dir.0);
    fs::create_dir_all(&dir.0).unwrap();
    fs::write(
        state.path::<Crashes>(),
        "{\"version\": 1, \"data\": {\"quick_deaths\": [1,",
    )
    .unwrap();
    let (crashes, loaded) = state.load::<Crashes>();
    assert_eq!(crashes, Crashes::default());
    let Loaded::Damaged(bad) = loaded else {
        panic!("{loaded:?}")
    };
    assert!(bad.to_string_lossy().ends_with("crashes.json.bad"));
    assert!(bad.exists() && !state.path::<Crashes>().exists());
}

#[test]
fn a_file_from_a_newer_isle_is_moved_aside_not_misread() {
    let dir = Dir::new("newer");
    let state = StateDir::new(&dir.0);
    fs::create_dir_all(&dir.0).unwrap();
    fs::write(
        state.path::<Crashes>(),
        "{\"version\": 7, \"data\": {\"quick_deaths\": [1]}}",
    )
    .unwrap();
    let (crashes, loaded) = state.load::<Crashes>();
    assert_eq!(crashes, Crashes::default());
    assert!(matches!(loaded, Loaded::Newer { version: 7, .. }));
}
