use std::{fmt, path::PathBuf, str::FromStr};

use tellus_level::LayerKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Show,
    Set {
        layer: LayerKind,
        x: u16,
        y: u16,
        value: u16,
    },
    Save(Option<PathBuf>),
    Open(PathBuf),
    New {
        width: u16,
        height: u16,
        path: Option<PathBuf>,
    },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandParseError {
    message: String,
}

impl CommandParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CommandParseError {}

impl Command {
    pub fn parse(input: &str) -> Result<Self, CommandParseError> {
        let parts: Vec<_> = input.split_whitespace().collect();
        let Some(name) = parts.first().copied() else {
            return Err(CommandParseError::new("empty command"));
        };

        match name {
            "help" => Ok(Self::Help),
            "show" => Ok(Self::Show),
            "quit" | "exit" => Ok(Self::Quit),
            "save" => match parts.as_slice() {
                ["save"] => Ok(Self::Save(None)),
                ["save", path] => Ok(Self::Save(Some(PathBuf::from(path)))),
                _ => Err(CommandParseError::new("usage: save [path]")),
            },
            "open" => match parts.as_slice() {
                ["open", path] => Ok(Self::Open(PathBuf::from(path))),
                _ => Err(CommandParseError::new("usage: open <path>")),
            },
            "new" => match parts.as_slice() {
                ["new", width, height] => Ok(Self::New {
                    width: parse_u16(width, "width")?,
                    height: parse_u16(height, "height")?,
                    path: None,
                }),
                ["new", width, height, path] => Ok(Self::New {
                    width: parse_u16(width, "width")?,
                    height: parse_u16(height, "height")?,
                    path: Some(PathBuf::from(path)),
                }),
                _ => Err(CommandParseError::new("usage: new <width> <height> [path]")),
            },
            "set" => match parts.as_slice() {
                ["set", layer, x, y, value] => Ok(Self::Set {
                    layer: parse_layer(layer)?,
                    x: parse_u16(x, "x")?,
                    y: parse_u16(y, "y")?,
                    value: parse_u16(value, "value")?,
                }),
                _ => Err(CommandParseError::new("usage: set <ground|detail|logic> <x> <y> <value>")),
            },
            _ => Err(CommandParseError::new(format!("unknown command: {name}"))),
        }
    }
}

fn parse_u16(input: &str, name: &'static str) -> Result<u16, CommandParseError> {
    u16::from_str(input).map_err(|_| CommandParseError::new(format!("invalid {name}: {input}")))
}

fn parse_layer(input: &str) -> Result<LayerKind, CommandParseError> {
    match input {
        "ground" => Ok(LayerKind::Ground),
        "detail" => Ok(LayerKind::Detail),
        "logic" => Ok(LayerKind::Logic),
        _ => Err(CommandParseError::new(format!(
            "invalid layer: {input} (expected ground, detail, or logic)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_command() {
        let command = Command::parse("set logic 3 4 7").unwrap();
        assert_eq!(
            command,
            Command::Set {
                layer: LayerKind::Logic,
                x: 3,
                y: 4,
                value: 7,
            }
        );
    }

    #[test]
    fn rejects_unknown_layer() {
        let err = Command::parse("set sky 0 0 1").unwrap_err();
        assert!(err.to_string().contains("invalid layer"));
    }
}
