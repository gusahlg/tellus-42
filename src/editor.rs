use std::path::{Path, PathBuf};

use tellus_level::{LayerKind, Level};

use crate::commands::Command;

pub struct Editor {
    level: Level,
    path: Option<PathBuf>,
    dirty: bool,
}

impl Editor {
    pub fn new(level: Level, path: Option<PathBuf>) -> Self {
        Self {
            level,
            path,
            dirty: false,
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

    pub fn apply(&mut self, command: Command) -> Result<CommandOutcome, String> {
        match command {
            Command::Help => Ok(CommandOutcome::Message(help_text())),
            Command::Show => Ok(CommandOutcome::Message(self.render())),
            Command::Set { layer, x, y, value } => {
                self.level
                    .set_tile(layer, x, y, value)
                    .map_err(|err| err.to_string())?;
                self.dirty = true;
                Ok(CommandOutcome::Message(format!(
                    "updated {}({}, {}) = {}",
                    layer_name(layer),
                    x,
                    y,
                    value
                )))
            }
            Command::Save(path) => {
                if let Some(path) = path {
                    self.path = Some(path);
                }

                let Some(path) = self.path.as_ref() else {
                    return Err("no active output path; use `save <path>`".to_string());
                };

                self.level
                    .save_to_file(path)
                    .map_err(|err| err.to_string())?;
                self.dirty = false;
                Ok(CommandOutcome::Message(format!("saved {}", path.display())))
            }
            Command::Open(path) => {
                *self = Self::from_path(path)?;
                Ok(CommandOutcome::Message(self.render()))
            }
            Command::New {
                width,
                height,
                path,
            } => {
                *self = Self::blank(width, height, path)?;
                Ok(CommandOutcome::Message(self.render()))
            }
            Command::Quit => Ok(CommandOutcome::Quit),
        }
    }

    pub fn render(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "path: {}",
            self.path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<unsaved>".to_string())
        ));
        lines.push(format!(
            "size: {}x{} | entities: {} | dirty: {}",
            self.level.width,
            self.level.height,
            self.level.entities.len(),
            self.dirty
        ));
        lines.push(String::new());

        for layer in LayerKind::ALL {
            lines.push(format!("[{}]", layer_name(layer)));
            lines.extend(render_layer(&self.level, layer));
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

pub enum CommandOutcome {
    Message(String),
    Quit,
}

fn render_layer(level: &Level, layer: LayerKind) -> Vec<String> {
    let mut rows = Vec::with_capacity(usize::from(level.height));
    for y in 0..level.height {
        let mut row = format!("{y:>3} ");
        for x in 0..level.width {
            let tile = level.tile(layer, x, y).unwrap_or(0);
            row.push_str(&format!("{tile:>4}"));
        }
        rows.push(row);
    }
    rows
}

fn layer_name(layer: LayerKind) -> &'static str {
    match layer {
        LayerKind::Ground => "ground",
        LayerKind::Detail => "detail",
        LayerKind::Logic => "logic",
    }
}

fn help_text() -> String {
    [
        "commands:",
        "  help",
        "  show",
        "  new <width> <height> [path]",
        "  open <path>",
        "  set <ground|detail|logic> <x> <y> <value>",
        "  save [path]",
        "  quit",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::Command;

    #[test]
    fn set_marks_editor_dirty() {
        let level = Level::new(2, 2).unwrap();
        let mut editor = Editor::new(level, None);

        let message = match editor
            .apply(Command::Set {
                layer: LayerKind::Ground,
                x: 1,
                y: 1,
                value: 9,
            })
            .unwrap()
        {
            CommandOutcome::Message(message) => message,
            CommandOutcome::Quit => panic!("expected message"),
        };

        assert!(message.contains("updated ground"));
        assert!(editor.render().contains("dirty: true"));
    }
}
