mod commands;
mod editor;

use std::io::{self, Write};

use commands::Command;
use editor::{CommandOutcome, Editor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut editor = bootstrap_editor()?;

    println!("tellus_42");
    println!("type `help` for commands");
    println!();
    println!("{}", editor.render());

    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("tlvl> ");
        io::stdout().flush()?;

        line.clear();
        let bytes_read = stdin.read_line(&mut line)?;
        if bytes_read == 0 {
            println!();
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match Command::parse(trimmed) {
            Ok(command) => match editor.apply(command) {
                Ok(CommandOutcome::Message(message)) => {
                    println!("{message}");
                }
                Ok(CommandOutcome::Quit) => break,
                Err(err) => eprintln!("error: {err}"),
            },
            Err(err) => eprintln!("error: {err}"),
        }
    }

    Ok(())
}

fn bootstrap_editor() -> Result<Editor, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(first) = args.next() else {
        return Ok(Editor::blank(8, 6, None)?);
    };

    if first == "--help" || first == "-h" {
        println!("usage: tellus_42 [level.tlvl]");
        println!("without a path, the editor starts with a blank 8x6 level");
        std::process::exit(0);
    }

    Ok(Editor::from_path(first)?)
}
