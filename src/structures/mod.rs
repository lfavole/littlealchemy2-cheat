//! Data structures used for the program.
use std::fmt::Display;

use game_status::GameStatus;
use history::History;
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
    /// Returns `true` if the combination contains any of the specified element IDs, `false` otherwise.
    pub fn contains(&self, mut ids: impl Iterator<Item = u16>) -> bool {
        ids.any(| x | self.has(x))
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
    pub fn from_str<'a>(s: &str, data: &'a GameStatus) -> Result<&'a Self, AlchemyElementError> {
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

    /// Return true if the element is final (if it can't be combined to create other elements), false otherwise.
    fn is_final(&self) -> bool {
        self.can_create.is_empty()
    }

    /// Return true if all the combinations that lead to the element have been done, false otherwise.
    fn all_target_combinations_done(&self, history: &History) -> bool {
        self.combinations.iter().all(| x | history.has_combination(x))
    }

    /// Return true if the element is depleted (everything has been done with ir), false otherwise.
    fn is_depleted(&self, history: &History) -> bool {
        self.is_final() && self.all_target_combinations_done(history)
    }

    /// Return true if all the combinations that contain the element have been done, false otherwise.
    fn all_combinations_done(&self, data: &GameStatus, history: &History) -> bool {
        for item in &self.can_create {
            for combination in &data.elements[*item].combinations {
                if combination.has(self.id) && !history.has_combination(combination) {
                    return false;
                }
            }
        }
        true
    }
}

impl std::fmt::Display for AlchemyElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<AlchemyElement> for String {
    fn from(val: AlchemyElement) -> Self {
        val.name.clone()
    }
}

/// Joins the given `elements` with a `separator`.
fn join<T: Display>(mut elements: impl Iterator<Item = T>, separator: impl Display) -> String {
    let mut ret;
    if let Some(first) = elements.next() {
        ret = format!("{first}");
    } else {
        return String::new();
    }
    for item in elements {
        ret.push_str(format!("{separator}{item}").as_str());
    }
    ret
}

/// Formats a list of `AlchemyElement`s into a string.
pub fn format_elements_list<'a>(elements: impl Iterator<Item = &'a AlchemyElement>) -> String {
    join(elements, ", ")
}

pub mod display;
pub mod condition;
pub mod game_status;
pub mod history;
pub mod path;
pub mod serializers;
