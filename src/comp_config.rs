use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct CompositorConfig {
    pub enabled: bool,
    pub corner_radius: u32,
    pub blur: bool,
    pub shadow: bool,
    pub shadow_opacity: u32,
    pub opacity: u32,
    pub animations: bool,
    pub animation_duration_ms: u32,
    pub warnings: Vec<String>,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            corner_radius: 0,
            blur: false,
            shadow: false,
            shadow_opacity: 28,
            opacity: 100,
            animations: false,
            animation_duration_ms: 140,
            warnings: Vec::new(),
        }
    }
}

pub fn load<P: AsRef<Path>>(path: P) -> io::Result<CompositorConfig> {
    let file = File::open(path)?;
    let mut config = CompositorConfig::default();
    for (index, line) in io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            warning(&mut config, index + 1, "expected key = value");
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        let parse_bool = || match value.to_ascii_lowercase().as_str() {
            "true" | "yes" | "on" | "1" => Some(true),
            "false" | "no" | "off" | "0" => Some(false),
            _ => None,
        };
        match key {
            "enabled" => set_bool(&mut config, "enabled", parse_bool(), index + 1),
            "blur" => set_bool(&mut config, "blur", parse_bool(), index + 1),
            "shadow" => set_bool(&mut config, "shadow", parse_bool(), index + 1),
            "animations" => set_bool(&mut config, "animations", parse_bool(), index + 1),
            "corner_radius" => set_range(
                &mut config.corner_radius,
                value.parse().ok(),
                0..=48,
                key,
                index + 1,
                &mut config.warnings,
            ),
            "shadow_opacity" => set_range(
                &mut config.shadow_opacity,
                value.parse().ok(),
                0..=70,
                key,
                index + 1,
                &mut config.warnings,
            ),
            "opacity" => set_range(
                &mut config.opacity,
                value.parse().ok(),
                20..=100,
                key,
                index + 1,
                &mut config.warnings,
            ),
            "animation_duration_ms" => set_range(
                &mut config.animation_duration_ms,
                value.parse().ok(),
                0..=1000,
                key,
                index + 1,
                &mut config.warnings,
            ),
            _ => warning(
                &mut config,
                index + 1,
                &format!("unknown compositor setting '{key}'"),
            ),
        }
    }
    Ok(config)
}

fn set_bool(config: &mut CompositorConfig, key: &str, value: Option<bool>, line: usize) {
    let target = match key {
        "enabled" => &mut config.enabled,
        "blur" => &mut config.blur,
        "shadow" => &mut config.shadow,
        _ => &mut config.animations,
    };
    if let Some(value) = value {
        *target = value;
    } else {
        warning(config, line, &format!("invalid boolean for '{key}'"));
    }
}

fn set_range(
    target: &mut u32,
    value: Option<u32>,
    range: std::ops::RangeInclusive<u32>,
    key: &str,
    line: usize,
    warnings: &mut Vec<String>,
) {
    if let Some(value) = value.filter(|v| range.contains(v)) {
        *target = value;
    } else {
        push_warning(
            warnings,
            line,
            &format!(
                "'{key}' must be between {} and {}",
                range.start(),
                range.end()
            ),
        );
    }
}

fn warning(config: &mut CompositorConfig, line: usize, message: &str) {
    push_warning(&mut config.warnings, line, message);
}

fn push_warning(warnings: &mut Vec<String>, line: usize, message: &str) {
    let warning = format!("AetherWM compositor config keyword issue [line {line}]: {message}");
    eprintln!("{warning}");
    warnings.push(warning);
}
