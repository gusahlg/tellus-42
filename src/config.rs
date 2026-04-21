use std::{
    fs,
    path::{Path, PathBuf},
};

use ratatui::style::Color;
use tellus_level::LayerKind;

use crate::app::expand_user_path;

#[derive(Debug, Clone)]
pub struct UiTheme {
    pub sidebar_bg: Color,
    pub panel_border: Color,
    pub panel_text: Color,
    pub muted_text: Color,
    pub accent_text: Color,
    pub success_text: Color,
    pub warning_text: Color,
    pub error_text: Color,
    pub grid_bg: Color,
    pub tile_bg: Color,
    pub cursor_normal: Color,
    pub cursor_insert: Color,
    pub cursor_command: Color,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sidebar_width: u16,
    pub tile_gap_x: u16,
    pub tile_gap_y: u16,
    pub theme: UiTheme,
    pub layer_mappings: [Option<PathBuf>; 3],
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            sidebar_bg: Color::Rgb(22, 24, 28),
            panel_border: Color::Rgb(96, 102, 112),
            panel_text: Color::Rgb(214, 217, 224),
            muted_text: Color::Rgb(142, 149, 160),
            accent_text: Color::Rgb(156, 196, 255),
            success_text: Color::Rgb(150, 204, 167),
            warning_text: Color::Rgb(230, 201, 123),
            error_text: Color::Rgb(232, 111, 111),
            grid_bg: Color::Rgb(220, 220, 220),
            tile_bg: Color::Black,
            cursor_normal: Color::Rgb(61, 110, 173),
            cursor_insert: Color::Rgb(72, 140, 87),
            cursor_command: Color::Rgb(166, 132, 58),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 38,
            tile_gap_x: 1,
            tile_gap_y: 1,
            theme: UiTheme::default(),
            layer_mappings: [None, None, None],
        }
    }
}

pub fn default_config_path() -> PathBuf {
    expand_user_path("~/.tellus-42.conf")
}

pub fn load_from_default_location() -> Result<Option<AppConfig>, String> {
    let path = default_config_path();
    if !path.exists() {
        return Ok(None);
    }
    load_from_file(&path).map(Some)
}

pub fn load_from_file(path: impl AsRef<Path>) -> Result<AppConfig, String> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;

    let mut config = AppConfig::default();
    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("invalid config line {line_number}: expected key=value"));
        };

        apply_entry(&mut config, key.trim(), value.trim(), line_number)?;
    }

    Ok(config)
}

fn apply_entry(
    config: &mut AppConfig,
    key: &str,
    value: &str,
    line_number: usize,
) -> Result<(), String> {
    match key {
        "sidebar_width" => {
            config.sidebar_width = parse_u16(value, key, line_number)?;
        }
        "tile_gap_x" => {
            config.tile_gap_x = parse_u16(value, key, line_number)?;
        }
        "tile_gap_y" => {
            config.tile_gap_y = parse_u16(value, key, line_number)?;
        }
        "ground_images" => {
            config.layer_mappings[layer_index(LayerKind::Ground)] = Some(expand_user_path(value));
        }
        "detail_images" => {
            config.layer_mappings[layer_index(LayerKind::Detail)] = Some(expand_user_path(value));
        }
        "logic_images" => {
            config.layer_mappings[layer_index(LayerKind::Logic)] = Some(expand_user_path(value));
        }
        "sidebar_bg" => config.theme.sidebar_bg = parse_color(value, key, line_number)?,
        "panel_border" => config.theme.panel_border = parse_color(value, key, line_number)?,
        "panel_text" => config.theme.panel_text = parse_color(value, key, line_number)?,
        "muted_text" => config.theme.muted_text = parse_color(value, key, line_number)?,
        "accent_text" => config.theme.accent_text = parse_color(value, key, line_number)?,
        "success_text" => config.theme.success_text = parse_color(value, key, line_number)?,
        "warning_text" => config.theme.warning_text = parse_color(value, key, line_number)?,
        "error_text" => config.theme.error_text = parse_color(value, key, line_number)?,
        "grid_bg" => config.theme.grid_bg = parse_color(value, key, line_number)?,
        "tile_bg" => config.theme.tile_bg = parse_color(value, key, line_number)?,
        "cursor_normal" => config.theme.cursor_normal = parse_color(value, key, line_number)?,
        "cursor_insert" => config.theme.cursor_insert = parse_color(value, key, line_number)?,
        "cursor_command" => config.theme.cursor_command = parse_color(value, key, line_number)?,
        _ => return Err(format!("unknown config key on line {line_number}: {key}")),
    }

    Ok(())
}

fn parse_u16(value: &str, key: &str, line_number: usize) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|_| format!("invalid integer for {key} on line {line_number}: {value}"))
}

fn parse_color(value: &str, key: &str, line_number: usize) -> Result<Color, String> {
    let hex = value.trim();
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return Err(format!(
            "invalid color for {key} on line {line_number}: expected #RRGGBB"
        ));
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| format!("invalid color for {key} on line {line_number}: {value}"))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| format!("invalid color for {key} on line {line_number}: {value}"))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| format!("invalid color for {key} on line {line_number}: {value}"))?;
    Ok(Color::Rgb(r, g, b))
}

fn layer_index(layer: LayerKind) -> usize {
    match layer {
        LayerKind::Ground => 0,
        LayerKind::Detail => 1,
        LayerKind::Logic => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_config() {
        let temp = std::env::temp_dir().join("tellus_42_config_test.conf");
        fs::write(
            &temp,
            "sidebar_width=44\nground_images=~/tiles/ground\naccent_text=#112233\n",
        )
        .unwrap();

        let config = load_from_file(&temp).unwrap();
        assert_eq!(config.sidebar_width, 44);
        assert!(config.layer_mappings[0].as_ref().unwrap().to_string_lossy().contains("tiles/ground"));
        assert_eq!(config.theme.accent_text, Color::Rgb(0x11, 0x22, 0x33));

        let _ = fs::remove_file(temp);
    }
}
