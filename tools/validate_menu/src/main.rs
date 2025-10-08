use serde_json::Value;
use std::fs;

fn main() {
    // When run as workspace member, the current dir is tools/validate_menu; menu.json lives at ../../ussdgeth/src/data/menu.json
    let path = "../../ussdgeth/src/data/menu.json";
    let content = fs::read_to_string(path).expect("Failed to read menu.json");
    let v: Value = serde_json::from_str(&content).expect("Invalid JSON");

    // Basic smoke checks
    if v.get("menus").is_none() {
        // Simplified from !v.get("menus").is_some()
        eprintln!("menu.json missing 'menus' key");
        std::process::exit(2);
    }

    if v.get("services").is_none() {
        // Simplified from !v.get("services").is_some()
        eprintln!("menu.json missing 'services' key");
        std::process::exit(2);
    }

    // Further checks: ensure every menu screen has a screen_type and default_next_screen
    if let Some(menus) = v.get("menus") {
        if let Some(obj) = menus.as_object() {
            for (name, screen) in obj {
                if screen.get("screen_type").is_none() {
                    eprintln!("Screen {} missing screen_type", name);
                    std::process::exit(2);
                }
                if screen.get("default_next_screen").is_none() {
                    eprintln!("Screen {} missing default_next_screen", name);
                    std::process::exit(2);
                }
            }
        }
    }

    println!("menu.json validation passed");
}
