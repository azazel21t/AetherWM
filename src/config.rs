// ============================================================================
// AetherWM - Configuration Parser
// ============================================================================

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Binding {
    pub modifiers: Vec<String>,
    pub key: String,
    pub action: String,
}

#[derive(Clone, Debug)]
pub struct WindowRule {
    pub selector_key: String,
    pub selector_val: String,
    pub property: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub border_width: u32,
    pub border_color: u64,
    pub focused_border_color: u64,
    pub inner_gap: u32,
    pub outer_gap: u32,
    pub master_ratio: f32,
    pub default_layout: String,
    pub autostart: Vec<String>,
    pub bindings: Vec<Binding>,
    pub window_rules: Vec<WindowRule>,
}

fn b(mods: &[&str], key: &str, action: &str) -> Binding {
    Binding {
        modifiers: mods.iter().map(|s| s.to_string()).collect(),
        key: key.to_string(),
        action: action.to_string(),
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            border_width: 1,
            border_color: 0x1f2335,
            focused_border_color: 0x7aa2f7,
            inner_gap: 10,
            outer_gap: 14,
            master_ratio: 0.50,
            default_layout: "master_stack".to_string(),
            autostart: Vec::new(),
            bindings: vec![
                b(&["Mod4"], "q", "killclient"),
                b(&["Mod4"], "r", "reload_config"),
                b(&["Mod4", "Shift"], "space", "togglefloating"),
                b(&["Mod4"], "p", "toggleoverview"),
                b(&["Mod4"], "minus", "zoom_out"),
                b(&["Mod4"], "equal", "zoom_in"),
                b(&["Mod4"], "0", "zoom_reset"),
                b(&["Mod4"], "h", "matrix_left"),
                b(&["Mod4"], "l", "matrix_right"),
                b(&["Mod4"], "k", "matrix_up"),
                b(&["Mod4"], "j", "matrix_down"),
                b(&["Mod4"], "Return", "exec kitty"),
                b(&["Mod4"], "d", "exec rofi -show drun"),
            ],
            window_rules: Vec::new(),
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> io::Result<Config> {
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file);

    let mut config = Config::default();
    config.bindings.clear(); // file bindings replace defaults

    for (idx, line) in reader.lines().enumerate() {
        let line_num = idx + 1;
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // autostart = "picom &"   (must be checked BEFORE generic key = value)
        if trimmed.starts_with("autostart") {
            if let Some((_, cmd)) = trimmed.split_once('=') {
                let cmd = cmd.trim().trim_matches('"').trim_matches('\'');
                if !cmd.is_empty() {
                    config.autostart.push(cmd.to_string());
                }
            }
            continue;
        }

        // bind Mod4 + p -> toggleoverview
        if trimmed.starts_with("bind") {
            if let Some(arrow_idx) = trimmed.find("->") {
                let lhs = trimmed[4..arrow_idx].trim();
                let action = trimmed[arrow_idx + 2..].trim().to_string();
                let parts: Vec<&str> = lhs.split('+').map(|s| s.trim()).collect();
                if let Some((key, mods)) = parts.split_last() {
                    config.bindings.push(Binding {
                        modifiers: mods.iter().map(|s| s.to_string()).collect(),
                        key: key.to_string(),
                        action,
                    });
                    continue;
                }
            }
            warn(line_num, &format!("Malformed keybinding: '{}'", trimmed));
            continue;
        }

        // rule class: "Kitty" -> border_color = 0x7aa2f7
        if trimmed.starts_with("rule") {
            if let Some(arrow_idx) = trimmed.find("->") {
                let selector_part = trimmed[4..arrow_idx].trim();
                let prop_part = trimmed[arrow_idx + 2..].trim();
                if let (Some((sk, sv)), Some((pk, pv))) =
                    (selector_part.split_once(':'), prop_part.split_once('='))
                {
                    config.window_rules.push(WindowRule {
                        selector_key: sk.trim().to_string(),
                        selector_val: sv.trim().trim_matches('"').trim_matches('\'').to_string(),
                        property: pk.trim().to_string(),
                        value: pv.trim().trim_matches('"').trim_matches('\'').to_string(),
                    });
                    continue;
                }
            }
            warn(line_num, &format!("Malformed window rule: '{}'", trimmed));
            continue;
        }

        // key = value
        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim();
            let val = val.trim().trim_matches('"').trim_matches('\'');
            let hex = |s: &str| u64::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok();
            match key {
                "border_width" => { if let Ok(v) = val.parse() { config.border_width = v; } }
                "border_color" => { if let Some(v) = hex(val) { config.border_color = v; } }
                "focused_border_color" => { if let Some(v) = hex(val) { config.focused_border_color = v; } }
                "inner_gap" => { if let Ok(v) = val.parse() { config.inner_gap = v; } }
                "outer_gap" => { if let Ok(v) = val.parse() { config.outer_gap = v; } }
                "master_ratio" => { if let Ok(v) = val.parse() { config.master_ratio = v; } }
                "default_layout" => { config.default_layout = val.to_string(); }
                _ => warn(line_num, &format!("Unknown configuration key: '{}'", key)),
            }
        } else {
            warn(line_num, &format!("Unrecognised line: '{}'", trimmed));
        }
    }

    Ok(config)
}

fn warn(line: usize, msg: &str) {
    eprintln!("AetherWM Config Warning [line {}]: {}", line, msg);
}
