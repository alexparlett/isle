//! A settings file written from chosen keys reads back as the same values.

use isle_settings::{Section, Store, Value, parse_path, schema::Settings, text_of};

fn set(settings: &mut Settings, key: &str, kdl: &str) {
    settings
        .set(&parse_path(key).unwrap(), Value::parse_kdl(kdl))
        .unwrap();
}

#[test]
fn the_kept_keys_read_back_and_the_rest_stay_default() {
    let mut settings = Settings::default();
    set(&mut settings, "appearance.mode", "\"light\"");
    set(&mut settings, "appearance.accent", "\"#3d5a3a\"");
    set(&mut settings, "keyboard.layouts", "\"gb\" \"us\"");
    set(&mut settings, "output[AOC U34 1].scale", "1.25");
    set(
        &mut settings,
        "keyboard-device[Keychron Q6 Max].profile",
        "\"mac\"",
    );
    set(&mut settings, "windows.edge-tiling", "#false");
    let keep = |key: &str| !key.starts_with("windows.");
    let text = text_of(&settings, keep);

    let file = std::env::temp_dir().join(format!("isle-text-of-{}.kdl", std::process::id()));
    std::fs::write(&file, &text).unwrap();
    let read: Store<Settings> = Store::open(&file);
    let _ = std::fs::remove_file(&file);
    assert!(read.error().is_none(), "{text}");
    let back = read.current();
    assert_eq!(back.appearance, settings.appearance);
    assert_eq!(back.keyboard, settings.keyboard);
    assert_eq!(back.output, settings.output);
    assert_eq!(back.keyboard_device, settings.keyboard_device);
    assert_eq!(back.windows, Settings::default().windows, "{text}");
}
