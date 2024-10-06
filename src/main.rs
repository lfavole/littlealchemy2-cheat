//! A program that displays combinations for Little Alchemy 2.
use std::{fs::File, io::BufReader, path::{Path, PathBuf}};

use clap::{CommandFactory, error::ErrorKind, Parser, Subcommand, ValueHint::FilePath};
use serde::de::DeserializeOwned;
use structures::{game_status::GameStatus, display_combinations_list, history::History, AlchemyElement, AlchemyElementError};

#[derive(Debug, Subcommand)]
/// The subcommands for the program.
pub enum Command {
    /// Display all the elements
    Display {
        /// Element to display
        #[arg(default_value="")]
        element: String,

        /// Only display combinations
        #[arg(long)]
        only_combinations: bool,

        /// Display already done combinations
        #[arg(long)]
        already_done: bool,

        /// Display unavailable combinations
        #[arg(long)]
        unavailable: bool,
    },
    /// Display how to finish the game
    Finish {
        /// Display JavaScript commands instead of human-readable instructions
        #[arg(long)]
        javascript: bool,
    },
    /// Display how to get an element
    Get {
        /// Element to display
        element: String,

        /// Display JavaScript commands instead of human-readable instructions
        #[arg(long)]
        javascript: bool,
    },
}

/// Display combinations for Little Alchemy 2.
#[derive(Debug, Parser)]
struct Cli {
    /// File with combinations
    #[arg(long, default_value="littlealchemy2.json", value_hint=FilePath)]
    file: PathBuf,

    /// File with history
    #[arg(long, default_value="history.json", value_hint=FilePath)]
    history_file: PathBuf,

    #[arg(long)]
    no_history: bool,

    #[command(subcommand)]
    command: Command,
}

impl Cli {
    // https://www.rustadventure.dev/introducing-clap/clap-v4/accepting-file-paths-as-arguments-in-clap#pathbufexists
    fn check_file_exists(file: &Path) {
        if !file.exists() {
            let mut cmd = Self::command();
            cmd.error(
                ErrorKind::ValueValidation,
                format!(
                    "file `{}` doesn't exist",
                    file.display()
                ),
            )
            .exit();
        }
    }

    fn parse() -> Self {
        let args: Self = Parser::parse();
        Cli::check_file_exists(&args.file);
        Cli::check_file_exists(&args.history_file);
        args
    }
}

fn read_json<T: DeserializeOwned>(file: &Path) -> Result<T, Box<dyn std::error::Error>> {
    let file = File::open(file)?;
    let reader = BufReader::new(file);
    let x = serde_json::from_reader(reader);
    x.map_err(std::convert::Into::into)
}

mod structures;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let mut data = GameStatus {
        elements: read_json(&args.file)?,
        history: if args.no_history {
            History::new()
        } else {
            read_json(&args.history_file)?
        },
        ..Default::default()
    };

    if !data.history.0.is_empty() {
        data.check();
    }

    if let Command::Display { element, .. } = &args.command {
        let element_or_err = AlchemyElement::from_str(element.as_str(), &data);
        match element_or_err {
            Ok(element) => {
                element.display(&data, &data.history, &args.command);
            },
            Err(AlchemyElementError::EmptyString) => {
                for item in data.elements.iter() {
                    item.display(&data, &data.history, &args.command);
                }
            },
            Err(err) => { Err(err)?; },
        }
        return Ok(());
    }

    if let Command::Get { element, javascript } = &args.command {
        let element = AlchemyElement::from_str(element.as_str(), &data)?;
        let name = element.name.clone();
        let combinations = data.obtain(element.id);
        if *javascript {
            display_combinations_list(&combinations[..], &data, Some(element), true);
        } else if combinations.is_empty() {
            assert!(data.acquired_elements.contains(&element.id));
            println!("You already have the {name} in your inventory");
        } else {
            println!("To get the {name}, you must combine:");
            display_combinations_list(&combinations[..], &data, Some(element), false);
        }
        return Ok(());
    }

    if let Command::Finish { javascript } = &args.command {
        let combinations = data.finish_game();
        if *javascript {
            display_combinations_list(&combinations[..], &data, None, true);
        } else if combinations.is_empty() {
            println!("You already finished the game");
        } else {
            println!("To finish the game, you must combine:");
            display_combinations_list(&combinations[..], &data, None, false);
        }
        return Ok(());
    }

    todo!();
}
