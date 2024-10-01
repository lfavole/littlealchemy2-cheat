//! A program that displays combinations for Little Alchemy 2.
use std::{fs::File, io::BufReader, path::{Path, PathBuf}};

use clap::{CommandFactory, error::ErrorKind, Parser, Subcommand, ValueHint::FilePath};
use structures::{database::LittleAlchemy2Database, display_combinations_list, history::History, AlchemyElement, AlchemyElementError};

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

mod structures;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let file = File::open(&args.file)?;
    let reader = BufReader::new(file);

    let mut data: LittleAlchemy2Database = serde_json::from_reader(reader)?;

    let history: History = if args.no_history {
        History::new()
    } else {
        let file2 = File::open(&args.history_file)?;
        let reader2 = BufReader::new(file2);

        serde_json::from_reader(reader2)?
    };

    if !history.0.is_empty() {
        history.iter().for_each(| item | data.combine(&item.combination));
        data.check();
    }

    if let Command::Display { element, .. } = &args.command {
        let element_or_err = structures::AlchemyElement::from_str(element.as_str(), &data);
        match element_or_err {
            Ok(element) => {
                element.display(&data, &history, &args.command);
            },
            Err(AlchemyElementError::EmptyString) => {
                for item in data.elements.iter() {
                    item.display(&data, &history, &args.command);
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
        let combinations = data.finish_game(&history);
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
