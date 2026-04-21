use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

use image::{DynamicImage, imageops::FilterType};
use ratatui::style::Color;
use tellus_level::{LayerKind, Level};

pub const COMMAND_HEIGHT: u16 = 3;
const MIN_TILE_WIDTH: u16 = 6;
const MIN_TILE_HEIGHT: u16 = 3;
const MAX_ZOOM: u8 = 4;

use crate::config::{AppConfig, UiTheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    Noop,
    Quit,
}

#[derive(Debug, Clone)]
struct EditSnapshot {
    level: Level,
    dirty: bool,
}

#[derive(Debug, Clone)]
pub struct App {
    level: Level,
    path: Option<PathBuf>,
    dirty: bool,
    active_layer: LayerKind,
    cursor_x: u16,
    cursor_y: u16,
    view_x: u16,
    view_y: u16,
    zoom: u8,
    mode: Mode,
    status: String,
    command_buffer: String,
    config: AppConfig,
    undo_stack: Vec<EditSnapshot>,
    redo_stack: Vec<EditSnapshot>,
    layers: [LayerAssets; 3],
}

#[derive(Debug, Clone)]
pub struct LayerAssets {
    pub folder: Option<PathBuf>,
    pub tiles: Vec<TileTexture>,
}

#[derive(Debug, Clone)]
pub struct TileTexture {
    pub id: u16,
    pub name: String,
    image: DynamicImage,
}

impl App {
    pub fn new(level: Level, path: Option<PathBuf>) -> Self {
        Self {
            level,
            path,
            dirty: false,
            active_layer: LayerKind::Ground,
            cursor_x: 0,
            cursor_y: 0,
            view_x: 0,
            view_y: 0,
            zoom: 2,
            mode: Mode::Normal,
            status: "Normal mode".to_string(),
            command_buffer: String::new(),
            config: AppConfig::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            layers: std::array::from_fn(|_| LayerAssets::default()),
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let level = Level::load_from_file(&path).map_err(|err| err.to_string())?;
        Ok(Self::new(level, Some(path)))
    }

    pub fn blank(width: u16, height: u16, path: Option<PathBuf>) -> Result<Self, String> {
        let level = Level::new(width, height).map_err(|err| err.to_string())?;
        Ok(Self::new(level, path))
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn level(&self) -> &Level {
        &self.level
    }

    pub fn active_layer(&self) -> LayerKind {
        self.active_layer
    }

    pub fn zoom(&self) -> u8 {
        self.zoom
    }

    pub fn cursor(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }

    pub fn view_origin(&self) -> (u16, u16) {
        (self.view_x, self.view_y)
    }

    pub fn tile_size(&self) -> (u16, u16) {
        let zoom = u16::from(self.zoom);
        (MIN_TILE_WIDTH * zoom, MIN_TILE_HEIGHT * zoom)
    }

    pub fn sidebar_width(&self) -> u16 {
        self.config.sidebar_width
    }

    pub fn tile_gap(&self) -> (u16, u16) {
        (self.config.tile_gap_x, self.config.tile_gap_y)
    }

    pub fn theme(&self) -> &UiTheme {
        &self.config.theme
    }

    pub fn layer_assets(&self, layer: LayerKind) -> &LayerAssets {
        &self.layers[layer_index(layer)]
    }

    #[cfg(test)]
    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    #[cfg(test)]
    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn apply_config(&mut self, config: AppConfig) -> Result<(), String> {
        self.config = config.clone();
        for (index, folder) in config.layer_mappings.iter().enumerate() {
            let Some(folder) = folder.clone() else {
                continue;
            };
            let layer = match index {
                0 => LayerKind::Ground,
                1 => LayerKind::Detail,
                _ => LayerKind::Logic,
            };
            self.map_layer_folder(layer, folder)?;
        }
        if self.status == "Normal mode" {
            self.status = "Loaded configuration".to_string();
        }
        Ok(())
    }

    pub fn begin_command(&mut self) {
        self.mode = Mode::Command;
        self.command_buffer.clear();
        self.status = "Command mode".to_string();
    }

    pub fn cancel_command(&mut self) {
        self.mode = Mode::Normal;
        self.command_buffer.clear();
        self.status = "Normal mode".to_string();
    }

    pub fn enter_insert_mode(&mut self) {
        self.mode = Mode::Insert;
        self.status = "Insert mode: press 1-9 to paint, Esc to stop".to_string();
    }

    pub fn enter_normal_mode(&mut self) {
        self.mode = Mode::Normal;
        self.status = "Normal mode".to_string();
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub fn command_push(&mut self, ch: char) {
        self.command_buffer.push(ch);
    }

    pub fn command_backspace(&mut self) {
        self.command_buffer.pop();
    }

    pub fn submit_command(&mut self) -> Result<CommandAction, String> {
        let input = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.mode = Mode::Normal;

        if input.is_empty() {
            self.status = "Normal mode".to_string();
            return Ok(CommandAction::Noop);
        }

        let result = self.run_command(&input)?;
        Ok(result)
    }

    pub fn move_cursor(&mut self, dx: i16, dy: i16, viewport_tiles: (u16, u16)) {
        let max_x = self.level.width.saturating_sub(1);
        let max_y = self.level.height.saturating_sub(1);

        self.cursor_x = clamp_u16(self.cursor_x, dx, max_x);
        self.cursor_y = clamp_u16(self.cursor_y, dy, max_y);
        self.ensure_cursor_visible(viewport_tiles);
    }

    pub fn cycle_layer(&mut self, delta: i8) {
        let current = layer_index(self.active_layer) as i8;
        let next = (current + delta).rem_euclid(3) as usize;
        self.active_layer = [LayerKind::Ground, LayerKind::Detail, LayerKind::Logic][next];
        self.status = format!("Active layer: {}", layer_name(self.active_layer));
    }

    pub fn adjust_zoom(&mut self, delta: i8, viewport_tiles: (u16, u16)) {
        let next = (self.zoom as i8 + delta).clamp(1, MAX_ZOOM as i8) as u8;
        self.zoom = next;
        self.ensure_cursor_visible(viewport_tiles);
        self.status = format!("Zoom {}", self.zoom);
    }

    pub fn paint_digit(&mut self, digit: u16) -> Result<(), String> {
        validate_tile_id(digit, "paint")?;
        self.record_undo_state();
        self.level
            .set_tile(self.active_layer, self.cursor_x, self.cursor_y, digit)
            .map_err(|err| err.to_string())?;
        self.dirty = true;
        let mapped = self
            .layer_assets(self.active_layer)
            .tiles
            .iter()
            .any(|tile| tile.id == digit);
        self.status = if mapped {
            format!(
                "Painted {} at ({}, {}) on {}",
                digit,
                self.cursor_x,
                self.cursor_y,
                layer_name(self.active_layer)
            )
        } else {
            format!(
                "Painted {} at ({}, {}) on {} (unmapped, showing numeric fallback)",
                digit,
                self.cursor_x,
                self.cursor_y,
                layer_name(self.active_layer)
            )
        };
        Ok(())
    }

    pub fn fill_active_layer(&mut self, digit: u16) -> Result<(), String> {
        validate_tile_id(digit, "fill")?;
        self.record_undo_state();

        for y in 0..self.level.height {
            for x in 0..self.level.width {
                self.level
                    .set_tile(self.active_layer, x, y, digit)
                    .map_err(|err| err.to_string())?;
            }
        }

        self.dirty = true;
        let mapped = self
            .layer_assets(self.active_layer)
            .tiles
            .iter()
            .any(|tile| tile.id == digit);
        self.status = if mapped {
            format!("Filled {} with {}", layer_name(self.active_layer), digit)
        } else {
            format!(
                "Filled {} with {} (unmapped, showing numeric fallback)",
                layer_name(self.active_layer),
                digit
            )
        };
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), String> {
        let Some(snapshot) = self.undo_stack.pop() else {
            return Err("nothing to undo".to_string());
        };

        self.redo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot);
        self.status = "Undo".to_string();
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        let Some(snapshot) = self.redo_stack.pop() else {
            return Err("nothing to redo".to_string());
        };

        self.undo_stack.push(self.snapshot());
        self.restore_snapshot(snapshot);
        self.status = "Redo".to_string();
        Ok(())
    }

    pub fn visible_tile_id(&self, x: u16, y: u16) -> Option<u16> {
        self.level.tile(self.active_layer, x, y).ok()
    }

    pub fn tile_texture(&self, layer: LayerKind, id: u16) -> Option<&TileTexture> {
        self.layer_assets(layer).tiles.iter().find(|tile| tile.id == id)
    }

    pub fn texture_colors(
        &self,
        texture: Option<&TileTexture>,
        width: u16,
        cell_rows: u16,
    ) -> Vec<Vec<(Color, Color)>> {
        let Some(texture) = texture else {
            return vec![vec![(Color::Reset, Color::Reset); width as usize]; cell_rows as usize];
        };

        let rows = sample_texture(texture, width, cell_rows.saturating_mul(2))
            .chunks(width as usize)
            .map(|row| row.to_vec())
            .collect::<Vec<_>>();

        rows.chunks(2)
            .map(|pair| {
                let top = pair.first().cloned().unwrap_or_default();
                let bottom = pair.get(1).cloned().unwrap_or_else(|| top.clone());
                top.into_iter().zip(bottom).collect::<Vec<_>>()
            })
            .collect::<Vec<Vec<(Color, Color)>>>()
    }

    pub fn ensure_cursor_visible(&mut self, viewport_tiles: (u16, u16)) {
        let (tiles_w, tiles_h) = viewport_tiles;
        let max_view_x = self.level.width.saturating_sub(tiles_w.max(1));
        let max_view_y = self.level.height.saturating_sub(tiles_h.max(1));

        if self.cursor_x < self.view_x {
            self.view_x = self.cursor_x;
        }
        if self.cursor_y < self.view_y {
            self.view_y = self.cursor_y;
        }
        if self.cursor_x >= self.view_x.saturating_add(tiles_w.max(1)) {
            self.view_x = self.cursor_x.saturating_add(1).saturating_sub(tiles_w.max(1));
        }
        if self.cursor_y >= self.view_y.saturating_add(tiles_h.max(1)) {
            self.view_y = self.cursor_y.saturating_add(1).saturating_sub(tiles_h.max(1));
        }

        self.view_x = self.view_x.min(max_view_x);
        self.view_y = self.view_y.min(max_view_y);
    }

    fn snapshot(&self) -> EditSnapshot {
        EditSnapshot {
            level: self.level.clone(),
            dirty: self.dirty,
        }
    }

    fn restore_snapshot(&mut self, snapshot: EditSnapshot) {
        self.level = snapshot.level;
        self.dirty = snapshot.dirty;
    }

    fn record_undo_state(&mut self) {
        self.undo_stack.push(self.snapshot());
        self.redo_stack.clear();
    }

    fn run_command(&mut self, input: &str) -> Result<CommandAction, String> {
        let parts: Vec<_> = input.split_whitespace().collect();
        let Some(command) = parts.first().copied() else {
            self.status = "Normal mode".to_string();
            return Ok(CommandAction::Noop);
        };

        match command {
            "q" | "quit" => Ok(CommandAction::Quit),
            "w" | "write" => {
                let arg = command_arg(input).map(expand_user_path);
                match arg {
                    None => self.save(None)?,
                    Some(path) => self.save(Some(path))?,
                }
                Ok(CommandAction::Noop)
            }
            "open" => {
                let path = command_arg(input)
                    .map(expand_user_path)
                    .ok_or_else(|| "usage: :open <path>".to_string())?;
                *self = Self::from_path(path)?;
                self.status = "Opened level".to_string();
                Ok(CommandAction::Noop)
            }
            "new" => {
                let (width, height, path) = parse_new_command(input)?;
                *self = Self::blank(width, height, path)?;
                self.status = "Created new level".to_string();
                Ok(CommandAction::Noop)
            }
            "map" => {
                let (layer, folder) = parse_map_command(input)?;
                self.map_layer_folder(layer, folder)?;
                Ok(CommandAction::Noop)
            }
            "fill" => {
                let digit = parse_fill_command(input)?;
                self.fill_active_layer(digit)?;
                Ok(CommandAction::Noop)
            }
            "help" => {
                self.status =
                    ":w [path], :q, :open <path>, :new <w> <h> [path], :map <layer> <folder>, :fill <0-9>"
                        .to_string();
                Ok(CommandAction::Noop)
            }
            _ => Err(format!("unknown command: {command}")),
        }
    }

    fn save(&mut self, path: Option<PathBuf>) -> Result<(), String> {
        if let Some(path) = path {
            self.path = Some(path);
        }

        let Some(path) = self.path.as_ref() else {
            return Err("no active output path; use :w <path>".to_string());
        };

        self.level
            .save_to_file(path)
            .map_err(|err| err.to_string())?;
        self.dirty = false;
        self.status = format!("Saved {}", path.display());
        Ok(())
    }

    fn map_layer_folder(&mut self, layer: LayerKind, folder: PathBuf) -> Result<(), String> {
        let mut files = fs::read_dir(&folder)
            .map_err(|err| format!("failed to read {}: {err}", folder.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| is_supported_image(path))
            .collect::<Vec<_>>();

        files.sort();

        let mut tiles = Vec::new();
        let mut skipped = 0usize;

        for path in files {
            match image::open(&path) {
                Ok(image) => {
                    let id = (tiles.len() + 1) as u16;
                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| format!("tile_{id}"));
                    tiles.push(TileTexture { id, name, image });
                    if tiles.len() == 9 {
                        break;
                    }
                }
                Err(_) => {
                    skipped += 1;
                }
            }
        }

        if tiles.is_empty() {
            return Err(format!(
                "no readable images found in {}",
                folder.display()
            ));
        }

        self.layers[layer_index(layer)] = LayerAssets {
            folder: Some(folder.clone()),
            tiles,
        };
        self.status = format!(
            "Mapped {} textures for {} from {}{}",
            self.layer_assets(layer).tiles.len(),
            layer_name(layer),
            folder.display(),
            if skipped > 0 {
                format!(" ({skipped} unreadable files skipped)")
            } else {
                String::new()
            }
        );
        Ok(())
    }
}

impl Default for LayerAssets {
    fn default() -> Self {
        Self {
            folder: None,
            tiles: Vec::new(),
        }
    }
}

fn parse_layer(input: &str) -> Result<LayerKind, String> {
    match input {
        "ground" => Ok(LayerKind::Ground),
        "detail" => Ok(LayerKind::Detail),
        "logic" => Ok(LayerKind::Logic),
        _ => Err(format!("invalid layer: {input}")),
    }
}

fn parse_u16(input: &str, name: &'static str) -> Result<u16, String> {
    input
        .parse::<u16>()
        .map_err(|_| format!("invalid {name}: {input}"))
}

fn parse_fill_command(input: &str) -> Result<u16, String> {
    let value = command_arg(input).ok_or_else(|| "usage: :fill <0-9>".to_string())?;
    let digit = parse_u16(value, "tile id")?;
    validate_tile_id(digit, "fill")?;
    Ok(digit)
}

fn parse_new_command(input: &str) -> Result<(u16, u16, Option<PathBuf>), String> {
    let mut parts = input.split_whitespace();
    let Some("new") = parts.next() else {
        return Err("usage: :new <width> <height> [path]".to_string());
    };

    let width = parts
        .next()
        .ok_or_else(|| "usage: :new <width> <height> [path]".to_string())
        .and_then(|value| parse_u16(value, "width"))?;
    let height = parts
        .next()
        .ok_or_else(|| "usage: :new <width> <height> [path]".to_string())
        .and_then(|value| parse_u16(value, "height"))?;

    let path = parts.next().map(|first| {
        let rest = std::iter::once(first).chain(parts).collect::<Vec<_>>().join(" ");
        expand_user_path(rest)
    });

    Ok((width, height, path))
}

fn command_arg(input: &str) -> Option<&str> {
    input.split_once(char::is_whitespace).map(|(_, rest)| rest.trim()).filter(|rest| !rest.is_empty())
}

fn parse_map_command(input: &str) -> Result<(LayerKind, PathBuf), String> {
    let rest = command_arg(input).ok_or_else(|| "usage: :map <ground|detail|logic> <folder>".to_string())?;
    let (layer, path) = rest
        .split_once(char::is_whitespace)
        .ok_or_else(|| "usage: :map <ground|detail|logic> <folder>".to_string())?;
    let folder = path.trim();
    if folder.is_empty() {
        return Err("usage: :map <ground|detail|logic> <folder>".to_string());
    }
    Ok((parse_layer(layer)?, expand_user_path(folder)))
}

fn layer_index(layer: LayerKind) -> usize {
    match layer {
        LayerKind::Ground => 0,
        LayerKind::Detail => 1,
        LayerKind::Logic => 2,
    }
}

pub fn layer_name(layer: LayerKind) -> &'static str {
    match layer {
        LayerKind::Ground => "ground",
        LayerKind::Detail => "detail",
        LayerKind::Logic => "logic",
    }
}

fn clamp_u16(value: u16, delta: i16, max_value: u16) -> u16 {
    if delta >= 0 {
        value.saturating_add(delta as u16).min(max_value)
    } else {
        value.saturating_sub(delta.unsigned_abs()).min(max_value)
    }
}

fn is_supported_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp")
    )
}

pub fn expand_user_path(raw: impl AsRef<str>) -> PathBuf {
    let raw = raw.as_ref();
    if raw == "~" {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(raw));
    }

    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }

    PathBuf::from(raw)
}

fn validate_tile_id(digit: u16, action: &'static str) -> Result<(), String> {
    if digit <= 9 {
        Ok(())
    } else {
        Err(format!("{action} only supports tile IDs 0-9"))
    }
}

fn sample_texture(texture: &TileTexture, width: u16, height: u16) -> Vec<Color> {
    let resized = texture
        .image
        .resize_exact(width.max(1) as u32, height.max(1) as u32, FilterType::Nearest)
        .to_rgba8();

    resized
        .pixels()
        .map(|pixel| {
            let [r, g, b, a] = pixel.0;
            if a == 0 {
                Color::Reset
            } else {
                Color::Rgb(r, g, b)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn zoom_is_clamped() {
        let mut app = App::blank(8, 8, None).unwrap();
        app.adjust_zoom(20, (8, 8));
        assert_eq!(app.zoom(), 4);
        app.adjust_zoom(-20, (8, 8));
        assert_eq!(app.zoom(), 1);
    }

    #[test]
    fn layer_cycles() {
        let mut app = App::blank(4, 4, None).unwrap();
        app.cycle_layer(1);
        assert_eq!(app.active_layer(), LayerKind::Detail);
        app.cycle_layer(1);
        assert_eq!(app.active_layer(), LayerKind::Logic);
        app.cycle_layer(1);
        assert_eq!(app.active_layer(), LayerKind::Ground);
    }

    #[test]
    fn unmapped_digit_can_still_be_painted() {
        let mut app = App::blank(4, 4, None).unwrap();
        app.paint_digit(7).unwrap();
        assert_eq!(app.visible_tile_id(0, 0), Some(7));
        assert!(app.status().contains("numeric fallback"));
    }

    #[test]
    fn expands_tilde_paths() {
        let expected = env::var("HOME").unwrap();
        assert_eq!(expand_user_path("~/tmp"), PathBuf::from(expected).join("tmp"));
    }

    #[test]
    fn map_command_keeps_path_with_spaces() {
        let (layer, path) = parse_map_command("map ground ~/Pictures/My Tiles").unwrap();
        assert_eq!(layer, LayerKind::Ground);
        assert!(path.to_string_lossy().contains("Pictures/My Tiles"));
    }

    #[test]
    fn mapping_skips_bad_images_and_keeps_valid_ones() {
        let temp = unique_temp_dir("tellus_42_map_test");
        fs::create_dir_all(&temp).unwrap();

        let valid_path = temp.join("01_valid.png");
        let invalid_path = temp.join("02_invalid.png");
        let extra_path = temp.join("03_valid.png");

        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .save(&valid_path)
            .unwrap();
        fs::write(&invalid_path, b"not a real png").unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 255, 0, 255]))
            .save(&extra_path)
            .unwrap();

        let mut app = App::blank(4, 4, None).unwrap();
        app.map_layer_folder(LayerKind::Ground, temp.clone()).unwrap();

        assert_eq!(app.layer_assets(LayerKind::Ground).tiles.len(), 2);
        assert!(app.status().contains("skipped"));

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn fill_active_layer_writes_all_tiles() {
        let mut app = App::blank(3, 2, None).unwrap();
        app.fill_active_layer(4).unwrap();

        for y in 0..app.level().height {
            for x in 0..app.level().width {
                assert_eq!(app.visible_tile_id(x, y), Some(4));
            }
        }
    }

    #[test]
    fn fill_command_parses_digit() {
        assert_eq!(parse_fill_command("fill 9").unwrap(), 9);
        assert!(parse_fill_command("fill 12").is_err());
    }

    #[test]
    fn new_command_keeps_path_with_spaces() {
        let (width, height, path) = parse_new_command("new 10 12 ~/My Levels/test level.tlvl").unwrap();
        assert_eq!((width, height), (10, 12));
        assert!(path.unwrap().to_string_lossy().contains("My Levels/test level.tlvl"));
    }

    #[test]
    fn undo_reverts_last_edit_and_redo_restores_it() {
        let mut app = App::blank(3, 3, None).unwrap();
        app.paint_digit(5).unwrap();
        assert_eq!(app.visible_tile_id(0, 0), Some(5));
        assert_eq!(app.undo_len(), 1);
        assert_eq!(app.redo_len(), 0);

        app.undo().unwrap();
        assert_eq!(app.visible_tile_id(0, 0), Some(0));
        assert_eq!(app.undo_len(), 0);
        assert_eq!(app.redo_len(), 1);

        app.redo().unwrap();
        assert_eq!(app.visible_tile_id(0, 0), Some(5));
        assert_eq!(app.undo_len(), 1);
        assert_eq!(app.redo_len(), 0);
    }

    #[test]
    fn new_edit_clears_redo_history() {
        let mut app = App::blank(3, 3, None).unwrap();
        app.paint_digit(1).unwrap();
        app.undo().unwrap();
        assert_eq!(app.redo_len(), 1);

        app.paint_digit(2).unwrap();
        assert_eq!(app.redo_len(), 0);
        assert_eq!(app.visible_tile_id(0, 0), Some(2));
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{nanos}"))
    }
}
