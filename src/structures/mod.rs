//! Data structures used for the program.
use crate::Command;

use database::LittleAlchemy2Database;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A combination between two elements.
pub struct Combination(
    #[serde(with = "serializers::number_as_str")]
    /// The first element to combine.
    pub u16,
    #[serde(with = "serializers::number_as_str")]
    /// The second element to combine.
    pub u16,
);

impl Combination {
    /// Returns a formatted version of the combination according to the given `LittleAlchemy2Database`.
    pub fn display(&self, data: &LittleAlchemy2Database) -> String {
        format!("{} + {}", &data.elements[self.0].name, data.elements[self.1].name)
    }

    /// Returns `true` if the combination contains any of the specified element IDs, `false` otherwise.
    pub fn contains(&self, ids: &[u16]) -> bool {
        ids.contains(&self.0) || ids.contains(&self.1)
    }

    /// Returns `true` if the combination contains the specified element ID, `false` otherwise.
    pub fn has(&self, id: u16) -> bool {
        id == self.0 || id == self.1
    }

    /// Returns the two possible permutations for the combination.
    pub fn permutations(&self) -> [Self; 2] {
        [Self(self.0, self.1), Self(self.1, self.0)]
    }
}

impl PartialEq for Combination {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
        || self.0 == other.1 && self.1 == other.0
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
/// Return the opposite of the boolean passed as référence. For serialization use only.
fn is_false(b: &bool) -> bool {
    !(*b)
}
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Deserialize, Serialize)]
/// An element in Little Alchemy 2.
pub struct AlchemyElement {
    #[serde(skip, default)]
    /// The ID of the element.
    pub id: u16,
    #[serde(rename = "n")]
    /// The name of the element.
    pub name: String,
    #[serde(rename = "p", default)]
    /// The combination that lead to the element.
    pub combinations: Vec<Combination>,
    #[serde(skip_serializing_if = "is_false", default)]
    /// Is the element prime (already present at the start)?
    pub prime: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    /// Is the element a base element (that can't be discovered by combinations,
    /// only after fulfilling a condition)?
    pub base: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    /// Is the element hidden?
    pub hidden: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    /// Is the element final (can't be combined to create other elements)?
    pub final_: bool,
    #[serde(skip_serializing_if = "condition::Condition::is_none", default)]
    /// The condition(s) that can lead to the apparition of the element.
    pub condition: condition::Condition,
    #[serde(rename = "c", default, with = "serializers::number_list_as_str_list")]
    /// The elements that can be created from this element.
    pub can_create: Vec<u16>,
}

impl PartialEq for AlchemyElement {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug)]
/// An error while parsing a string into an `AlchemyElement`.
pub enum AlchemyElementError {
    /// The string was empty.
    EmptyString,
    /// The element was not found. Currently this error is only created when an element ID is passed.
    NotFound(String),
    /// The element number could not be parsed. Currently this error is also created when an element string was not found.
    InvalidNumber(std::num::ParseIntError),
}
impl std::fmt::Display for AlchemyElementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlchemyElementError::EmptyString => f.write_str("empty string"),
            AlchemyElementError::NotFound(s) => f.write_fmt(format_args!("element not found: {s}")),
            AlchemyElementError::InvalidNumber(err) => f.write_fmt(format_args!("error while parsing number: {err}")),
        }
    }
}
impl std::error::Error for AlchemyElementError {}
impl AlchemyElement {
    pub fn from_str<'a>(s: &str, data: &'a LittleAlchemy2Database) -> Result<&'a Self, AlchemyElementError> {
        if s.is_empty() {
            return Err(AlchemyElementError::EmptyString);
        }
        match s.parse::<u16>() {
            Ok(num) => {
                Ok(
                    data.elements.get(num)
                    .ok_or(AlchemyElementError::NotFound(format!("can't find element #{num}")))?
                )
            },
            Err(err) => {
                for item in data.elements.iter() {
                    if s.to_lowercase() == item.name {
                        return Ok(item);
                    }
                }
                Err(AlchemyElementError::InvalidNumber(err))
            },
        }
    }

    pub fn display(
        &self,
        data: &database::LittleAlchemy2Database,
        history: &history::History,
        subcommand: &Command,
    ) {
        let only_combinations_;
        let already_done_;
        let unavailable_;
        match subcommand {
            Command::Display { only_combinations, already_done, unavailable, .. } => {
                only_combinations_ = *only_combinations;
                already_done_ = *already_done;
                unavailable_ = *unavailable;
            },
            _ => {
                panic!("called display() with a non-Display subcommand: {subcommand:?}");
            }
        }
        let mut good_combinations: Vec<&Combination> = self.combinations.iter().collect();
        if !unavailable_ {
            good_combinations = good_combinations.iter().filter(| x | data.can_do_combination(x)).copied().collect();
        }
        if !already_done_ {
            good_combinations = good_combinations.iter().filter(| x | !history.has_combination(x)).copied().collect();
        }
        // let good_combinations: Vec<&Combination> = good_combinations.collect();
        if !only_combinations_ || !good_combinations.is_empty() {
            println!("Element #{}: {}", self.id, self.name);
        }
        if !only_combinations_ {
            if self.prime {
                println!("Is a prime element (is present at the start)");
            }
            if self.base {
                println!("Is a base element (can't be created from other items)");
            }
            if self.final_ {
                println!("Is a final element (can't be mixed with other items)");
            }
            if self.hidden {
                println!("Is a hidden element (this property seems to be unused)");
            }
            self.condition.display(data);
        }
        for comb in &good_combinations {
            println!("= {}", comb.display(data));
        }
        if !only_combinations_ && !self.can_create.is_empty() {
            println!("Can create:");
            for creation in &self.can_create {
                println!("- {}", data.elements[*creation].name);
            }
        }
        if !only_combinations_ || !good_combinations.is_empty() {
            println!();
        }
    }
}

/// Formats a list of `AlchemyElement`s into a string.
pub fn format_elements_list(elements: &[&AlchemyElement]) -> String {
    elements.iter().map(| x | x.name.to_string()).collect::<Vec<String>>().join(", ")
}

/// Displays a list of `Combination`s.
pub fn display_combinations_list(
    combinations: &[Combination],
    data: &LittleAlchemy2Database,
    target_element: Option<&AlchemyElement>,
    javascript: bool,
) {
    if javascript {
        if combinations.is_empty() {
            return;
        }
        println!(r###"localStorage.setItem("stats", '{{"firstLaunch":0,"sessionsCount":1}}');"###);
        println!(r###"localStorage.setItem("tutorials", '{{"shownText":["final","exhausted"]}}');"###);
        println!(r###"var game_history = JSON.parse(localStorage.getItem("history")) || [];"###);
        for combination in combinations {
            println!(r###"game_history.push([{}, {}, 0]);"###, combination.0, combination.1);
        }
        println!(r###"localStorage.setItem("history", JSON.stringify(game_history));"###);
        return;
    }
    let len = combinations.len();
    for (i, combination) in combinations.iter().enumerate() {
        let mut next_element_str = String::new();
        // If it's not the last element, check in all the following combinations
        // if there is the result (because there can be multiple results)
        if i < len - 1 && target_element.is_some() {
            let new_elements = data.elements.get_from_combination(combination);
            'outer: for el in new_elements {
                for combination_to_try in combinations {
                    if combination_to_try.has(el.id) {
                        next_element_str = format!(" (which gives the {})", el.name);
                        break 'outer;
                    }
                }
            }
            assert!(!next_element_str.is_empty());
        } else {
            next_element_str = format!(
                " (which gives the {})",
                format_elements_list(&data.elements.get_from_combination(combination)[..]),
            );
        }

        println!("- {}{next_element_str}", combination.display(data));
    }
}

pub mod condition;
pub mod database;
pub mod history;
pub mod path;
pub mod serializers;
