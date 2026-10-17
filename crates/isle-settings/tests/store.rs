use std::{fs, path::PathBuf};

use isle_settings::{Reload, SetError, Store, Value, schema::Settings, schema_of};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("isle-settings-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self) -> PathBuf {
        self.0.join("settings.kdl")
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_missing_file_means_every_default_and_writes_nothing() {
    let dir = Dir::new("missing");
    let store: Store<Settings> = Store::open(dir.file());
    assert!(store.error().is_none());
    assert_eq!(store.current(), &Settings::default());
    assert_eq!(store.get("keyboard.repeat-rate"), Ok(Value::Int(25)));
    assert!(!dir.file().exists());
}

#[test]
fn a_set_keeps_the_users_comments_and_reports_its_key() {
    let dir = Dir::new("comments");
    let typed = "// my keyboard\nkeyboard {\n    repeat-rate 30 // quick\n    layouts \"gb\"\n}\n";
    fs::write(dir.file(), typed).unwrap();
    let mut store: Store<Settings> = Store::open(dir.file());
    assert_eq!(store.current().keyboard.repeat_rate, 30);

    let changed = store.set("keyboard.repeat-rate", Value::Int(40)).unwrap();
    assert_eq!(changed, vec!["keyboard.repeat-rate".to_owned()]);
    assert_eq!(
        fs::read_to_string(dir.file()).unwrap(),
        "// my keyboard\nkeyboard {\n    repeat-rate 40 // quick\n    layouts \"gb\"\n}\n"
    );
    assert_eq!(
        store.reload(),
        Reload::Unchanged,
        "Isle's own write is not a hand edit"
    );
}

#[test]
fn a_broken_hand_edit_keeps_the_last_good_settings_and_refuses_sets() {
    let dir = Dir::new("broken");
    fs::write(dir.file(), "keyboard {\n    repeat-rate 30\n}\n").unwrap();
    let mut store: Store<Settings> = Store::open(dir.file());

    let typed = "keyboard {\n    repeat-rate 30\n    layouts \"gb\"\n";
    fs::write(dir.file(), typed).unwrap();
    let Reload::Rejected(err) = store.reload() else {
        panic!("a file without its closing brace was accepted");
    };
    assert_eq!(err.line, 1);
    assert_eq!(store.current().keyboard.repeat_rate, 30);
    assert!(store.current().keyboard.layouts.is_empty(), "the system's layout stands");

    assert!(matches!(
        store.set("keyboard.repeat-rate", Value::Int(50)),
        Err(SetError::Refused(_))
    ));
    assert_eq!(
        fs::read_to_string(dir.file()).unwrap(),
        typed,
        "the file stays as typed"
    );

    fs::write(dir.file(), format!("{typed}}}\n")).unwrap();
    assert_eq!(
        store.reload(),
        Reload::Applied(vec!["keyboard.layouts".to_owned()])
    );
    assert!(store.error().is_none());
    store.set("keyboard.repeat-rate", Value::Int(50)).unwrap();
}

#[test]
fn an_unknown_key_rejects_the_whole_file_with_its_position() {
    let dir = Dir::new("unknown");
    fs::write(
        dir.file(),
        "keyboard {\n    repeat-rate 40\n    clack #true\n}\n",
    )
    .unwrap();
    let store: Store<Settings> = Store::open(dir.file());
    let err = store.error().expect("an unknown key is an error");
    assert_eq!((err.line, err.column), (3, 5));
    assert!(err.message.contains("clack"), "{}", err.message);
    assert_eq!(
        store.current().keyboard.repeat_rate,
        25,
        "nothing of the file applies"
    );
}

#[test]
fn values_are_checked_against_their_range_type_and_choices() {
    let dir = Dir::new("checks");
    let mut store: Store<Settings> = Store::open(dir.file());
    let err = store
        .set("keyboard.repeat-rate", Value::Int(0))
        .unwrap_err();
    assert_eq!(
        err,
        SetError::Invalid("keyboard.repeat-rate: must be from 1 to 100".into())
    );
    assert!(store.set("keyboard.numlock", Value::Int(1)).is_err());
    let err = store
        .set(
            "output[AOC U34G2G4R3 0].vrr",
            Value::Str("sometimes".into()),
        )
        .unwrap_err();
    assert!(err.to_string().contains("off, fullscreen, always"), "{err}");
    assert!(store.set("keyboard.clack", Value::Bool(true)).is_err());
    assert!(!dir.file().exists(), "nothing invalid was written");

    fs::write(dir.file(), "output \"A\" { scale 9.0 }\n").unwrap();
    store.reload();
    assert!(store.error().unwrap().message.contains("from 0.5 to 4"));
}

#[test]
fn keyed_nodes_and_app_sections_are_created_where_absent() {
    let dir = Dir::new("keyed");
    let mut store: Store<Settings> = Store::open(dir.file());
    store
        .set("output[AOC U34G2G4R3 0].scale", Value::Float(1.25))
        .unwrap();
    store
        .set("output[AOC U34G2G4R3 0].vrr", Value::Str("always".into()))
        .unwrap();
    store
        .set(
            "app[io.github.alexparlett.Isle.Example].show-hidden",
            Value::Bool(true),
        )
        .unwrap();
    store
        .set(
            "keyboard.layouts",
            Value::List(vec!["gb".into(), "fr".into()]),
        )
        .unwrap();
    assert_eq!(
        fs::read_to_string(dir.file()).unwrap(),
        "output \"AOC U34G2G4R3 0\" {\n    scale 1.25\n    vrr \"always\"\n}\n\
         app \"io.github.alexparlett.Isle.Example\" {\n    show-hidden #true\n}\n\
         keyboard {\n    layouts \"gb\" \"fr\"\n}\n"
    );
    let again: Store<Settings> = Store::open(dir.file());
    assert_eq!(again.current(), store.current());
    assert!(again.current().example_app.show_hidden);
}

#[test]
fn an_unknown_app_or_a_repeated_node_is_an_error() {
    let dir = Dir::new("repeat");
    fs::write(dir.file(), "app \"com.example.Nope\" { }\n").unwrap();
    assert!(
        Store::<Settings>::open(dir.file())
            .error()
            .unwrap()
            .message
            .contains("com.example.Nope")
    );
    fs::write(dir.file(), "debug { }\ndebug { }\n").unwrap();
    assert!(
        Store::<Settings>::open(dir.file())
            .error()
            .unwrap()
            .message
            .contains("twice")
    );
    fs::write(dir.file(), "output \"A\" { }\noutput \"A\" { }\n").unwrap();
    assert!(
        Store::<Settings>::open(dir.file())
            .error()
            .unwrap()
            .message
            .contains("twice")
    );
}

#[test]
fn a_symlinked_file_is_written_through_its_link() {
    let dir = Dir::new("symlink");
    let target = dir.0.join("dotfiles.kdl");
    fs::write(&target, "debug { watchdog-abort #true }\n").unwrap();
    std::os::unix::fs::symlink(&target, dir.file()).unwrap();
    let mut store: Store<Settings> = Store::open(dir.file());
    store
        .set("debug.watchdog-abort", Value::Bool(false))
        .unwrap();
    assert!(
        fs::symlink_metadata(dir.file())
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "debug { watchdog-abort #false }\n"
    );
}

#[test]
fn the_schema_lists_every_key_with_its_default() {
    let keys = schema_of::<Settings>();
    let rate = keys
        .iter()
        .find(|k| k.key == "keyboard.repeat-rate")
        .unwrap();
    assert_eq!(
        (rate.kind, rate.default.as_str(), rate.constraint.as_str()),
        ("integer", "25", "1 to 100")
    );
    let vrr = keys.iter().find(|k| k.key == "output[*].vrr").unwrap();
    assert_eq!(vrr.default, "\"fullscreen\"");
    assert_eq!(vrr.constraint, "off, fullscreen, always");
    assert!(
        keys.iter()
            .any(|k| k.key == "app[io.github.alexparlett.Isle.Example].sidebar-width")
    );
}

#[test]
fn a_safe_start_ignores_the_file_until_asked_and_writes_nothing_until_then() {
    let dir = Dir::new("safe");
    fs::write(dir.file(), "keyboard { repeat-rate 40; }\n").unwrap();
    let mut store: Store<Settings> = Store::defaults(dir.file());
    assert_eq!(store.current().keyboard.repeat_rate, 25);
    assert!(store.error().is_some());
    assert!(matches!(
        store.set("keyboard.repeat-rate", Value::Int(30)),
        Err(SetError::Refused(_))
    ));
    assert_eq!(
        store.use_file(),
        Reload::Applied(vec!["keyboard.repeat-rate".into()])
    );
    assert!(store.error().is_none());
    assert_eq!(store.current().keyboard.repeat_rate, 40);
}

#[test]
fn a_list_can_be_emptied_and_read_back_empty() {
    let dir = Dir::new("empty");
    let mut store: Store<Settings> = Store::open(dir.file());
    assert_eq!(store.get("windows.games"), Ok(Value::List(vec![])));

    store
        .set("windows.keep-running", Value::List(vec!["kitty".into()]))
        .unwrap();
    let changed = store.set("windows.keep-running", Value::List(vec![])).unwrap();
    assert_eq!(changed, vec!["windows.keep-running".to_owned()]);
    assert_eq!(store.get("windows.keep-running"), Ok(Value::List(vec![])));

    let changed = store.set("shortcuts.hide", Value::List(vec![])).unwrap();
    assert_eq!(changed, vec!["shortcuts.hide".to_owned()]);
    assert!(store.current().shortcuts.hide.is_empty(), "an empty list turns the action off");
    let again: Store<Settings> = Store::open(dir.file());
    assert_eq!(again.current(), store.current());
}

#[test]
fn a_reset_takes_the_node_out_and_the_default_back() {
    let dir = Dir::new("reset");
    let typed = "// mine\nkeyboard {\n    repeat-rate 40 // quick\n    layouts \"gb\"\n}\noutput \"AOC U34 1\" {\n    mode \"2560x1440\"\n    scale 1.25\n}\n";
    fs::write(dir.file(), typed).unwrap();
    let mut store: Store<Settings> = Store::open(dir.file());

    assert_eq!(store.reset("keyboard.repeat-rate").unwrap(), vec!["keyboard.repeat-rate".to_owned()]);
    assert_eq!(store.get("keyboard.repeat-rate"), Ok(Value::Int(25)));
    assert_eq!(store.reset("output[AOC U34 1].mode").unwrap(), vec!["output[AOC U34 1].mode".to_owned()]);
    assert!(store.get("output[AOC U34 1].mode").is_err(), "an optional key is unset again");
    assert_eq!(
        fs::read_to_string(dir.file()).unwrap(),
        "// mine\nkeyboard {\n    layouts \"gb\"\n}\noutput \"AOC U34 1\" {\n    scale 1.25\n}\n"
    );
    assert_eq!(store.reset("keyboard.repeat-rate"), Ok(vec![]), "nothing written is nothing to reset");
    assert_eq!(store.reload(), Reload::Unchanged);

    assert!(matches!(store.reset("keyboard.repeat-speed"), Err(SetError::Invalid(_))));
    store.reset("keyboard").unwrap();
    assert_eq!(store.current().keyboard, Settings::default().keyboard);
}

#[test]
fn a_reset_that_would_clash_is_refused() {
    let dir = Dir::new("clash");
    fs::write(dir.file(), "shortcuts {\n    hide \"Cmd+J\"\n    minimise \"Cmd+H\"\n}\n").unwrap();
    let mut store: Store<Settings> = Store::open(dir.file());
    assert!(store.error().is_none());
    assert!(matches!(store.reset("shortcuts.hide"), Err(SetError::Invalid(_))));
    assert_eq!(store.current().shortcuts.hide, vec!["Cmd+J".to_owned()]);
}

#[test]
fn an_entry_is_removed_whole_and_only_an_entry() {
    let dir = Dir::new("remove");
    let mut store: Store<Settings> = Store::open(dir.file());
    store
        .set("focus-mode[Reading].allow-apps", Value::List(vec!["kitty".into()]))
        .unwrap();
    store.set("notification-app[steam].sound", Value::Bool(false)).unwrap();
    assert!(store.values().contains_key("focus-mode[Reading].allow-apps"));

    let changed = store.remove("focus-mode[Reading]").unwrap();
    assert!(changed.contains(&"focus-mode[Reading].allow-apps".to_owned()));
    assert!(!store.values().keys().any(|k| k.starts_with("focus-mode[Reading]")));
    assert!(store.values().contains_key("notification-app[steam].sound"));

    assert!(matches!(store.remove("keyboard"), Err(SetError::Invalid(_))));
    assert!(matches!(store.remove("focus-mode[Reading].symbol"), Err(SetError::Invalid(_))));
}

#[test]
fn a_reset_is_refused_while_a_hand_edit_is_broken() {
    let dir = Dir::new("reset-broken");
    fs::write(dir.file(), "keyboard {\n    repeat-rate 30\n}\n").unwrap();
    let mut store: Store<Settings> = Store::open(dir.file());
    fs::write(dir.file(), "keyboard {\n    repeat-rate \"fast\"\n}\n").unwrap();
    assert!(matches!(store.reload(), Reload::Rejected(_)));
    assert!(matches!(store.reset("keyboard.repeat-rate"), Err(SetError::Refused(_))));
    assert!(matches!(store.remove("output[A]"), Err(SetError::Refused(_))));
}
