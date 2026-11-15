use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Binding {
    pub modifiers: Vec<String>,
    pub key: String,
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct WindowRule {
    pub selector_key: String,
    pub selector_val: String,
    pub property: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    // General Window Settings
    pub border_width: u32,
    pub border_color: u64,
    pub focused_border_color: u64,
    pub inner_gap: u32,
    pub outer_gap: u32,

    // Tiling Layout Preferences
    pub master_ratio: f32,
    pub default_layout: String, // e.g., "master_stack" or "monocle"

    // Autostart Applications
    pub autostart: Vec<String>,

    // Collections
    pub bindings: Vec<Binding>,
    pub window_rules: Vec<WindowRule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            border_width: 2,
            border_color: 0x1f2335,
            focused_border_color: 0x7aa2f7,
            inner_gap: 10,
            outer_gap: 14,
            master_ratio: 0.5,
            default_layout: String::from("master_stack"),
            autostart: Vec::new(),
            bindings: Vec::new(),
            window_rules: Vec::new(),
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, io::Error> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut config = Config::default();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Skip empty lines and full-line comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Strip inline comments (e.g., "reload_config # comment" -> "reload_config")
        let clean_line = if let Some(idx) = trimmed.find('#') {
            &trimmed[..idx]
        } else {
            trimmed
        }
        .trim();

        if clean_line.is_empty() {
            continue;
        }

        // Parse global properties (e.g., border_width = 2, master_ratio = 0.55, inner_gap = 12)
        if clean_line.contains('=') && !clean_line.starts_with("bind ") && !clean_line.starts_with("windowrule ") {
            if let Some((key_part, val_part)) = clean_line.split_once('=') {
                let key = key_part.trim();
                let val = val_part.trim();

                match key {
                    "border_width" => {
                        if let Ok(v) = val.parse::<u32>() {
                            config.border_width = v;
                        }
                    }
                    "border_color" => {
                        let clean_val = val.strip_prefix("0x").unwrap_or(val);
                        if let Ok(v) = u64::from_str_radix(clean_val, 16) {
                            config.border_color = v;
                        }
                    }
                    "focused_border_color" => {
                        let clean_val = val.strip_prefix("0x").unwrap_or(val);
                        if let Ok(v) = u64::from_str_radix(clean_val, 16) {
                            config.focused_border_color = v;
                        }
                    }
                    "inner_gap" => {
                        if let Ok(v) = val.parse::<u32>() {
                            config.inner_gap = v;
                        }
                    }
                    "outer_gap" => {
                        if let Ok(v) = val.parse::<u32>() {
                            config.outer_gap = v;
                        }
                    }
                    "master_ratio" => {
                        if let Ok(v) = val.parse::<f32>() {
                            config.master_ratio = v.clamp(0.1, 0.9);
                        }
                    }
                    "default_layout" => {
                        config.default_layout = val.to_string();
                    }
                    "autostart" => {
                        let cleaned = val.trim_matches(|c| c == '[' || c == ']' || c == '"' || c == '\'' || c == ' ');
                        if !cleaned.is_empty() {
                            for cmd in cleaned.split(',') {
                                let cmd_clean = cmd.trim().trim_matches(|c| c == '"' || c == '\'');
                                if !cmd_clean.is_empty() {
                                    config.autostart.push(cmd_clean.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // Parse keybindings: bind Mod4 + r -> reload_config
        else if clean_line.starts_with("bind ") {
            let rest = clean_line.strip_prefix("bind ").unwrap().trim();
            if let Some((chord_part, action_part)) = rest.split_once("->") {
                let action = action_part.trim().to_string();
                let tokens: Vec<&str> = chord_part.split('+').map(|s| s.trim()).collect();
                
                if !tokens.is_empty() {
                    let key = tokens.last().unwrap().to_string();
                    let modifiers: Vec<String> = tokens[..tokens.len() - 1]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    config.bindings.push(Binding {
                        modifiers,
                        key,
                        action,
                    });
                }
            }
        }
        // Parse window rules: windowrule class = kitty, border_width = 4
        else if clean_line.starts_with("windowrule ") {
            let rest = clean_line.strip_prefix("windowrule ").unwrap().trim();
            let parts: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let selector_tokens: Vec<&str> = parts[0].split('=').map(|s| s.trim()).collect();
                let property_tokens: Vec<&str> = parts[1].split('=').map(|s| s.trim()).collect();

                if selector_tokens.len() == 2 && property_tokens.len() == 2 {
                    config.window_rules.push(WindowRule {
                        selector_key: selector_tokens[0].to_string(),
                        selector_val: selector_tokens[1].to_string(),
                        property: property_tokens[0].to_string(),
                        value: property_tokens[1].to_string(),
                    });
                }
            }
        }
    }

    Ok(config)
}
