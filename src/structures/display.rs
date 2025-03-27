use std::fmt::Display;

use crate::{structures::format_elements_list, Command};

use super::{condition::Condition, game_status::{CombinationsIterator, GameStatus}, AlchemyElement, Combination};

/// A `Condition` that can be displayed according to a `GameStatus`.
pub struct ConditionDisplay<'a> {
    inner: &'a Condition,
    status: &'a GameStatus,
}

impl<'a> ConditionDisplay<'a> {
    pub fn new(condition: &'a Condition, status: &'a GameStatus) -> Self {
        Self { inner: condition, status }
    }
}

impl Display for ConditionDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            Condition::None => {Ok(())},
            Condition::Progress(total) => {
                writeln!(f, "Will be unlocked after discovering {total} elements")
            },
            Condition::Elements(elements, min) => {
                writeln!(
                    f,
                    "Will be unlocked after discovering {} elements from those: {}",
                    min,
                    format_elements_list(elements.iter().map(| x | &self.status.elements[*x])),
                )
            },
        }
    }
}

/// A `Combination` that can be displayed according to a `GameStatus`.
pub struct CombinationDisplay<'a> {
    inner: &'a Combination,
    status: &'a GameStatus,
}

impl<'a> CombinationDisplay<'a> {
    pub fn new(combination: &'a Combination, status: &'a GameStatus) -> Self {
        Self { inner: combination, status }
    }
}

impl Display for CombinationDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} + {}",
            self.status.elements[self.inner.0].name,
            self.status.elements[self.inner.1].name,
        ))
    }
}

/// An `AlchemyElementDisplay` that can be displayed according to a `GameStatus` and an optional `Command`.
pub struct AlchemyElementDisplay<'a> {
    inner: &'a AlchemyElement,
    status: &'a GameStatus,
    subcommand: Option<&'a Command>,
}

impl<'a> AlchemyElementDisplay<'a> {
    pub fn new(element: &'a AlchemyElement, status: &'a GameStatus, subcommand: Option<&'a Command>) -> Self {
        Self { inner: element, status, subcommand }
    }
}

impl Display for AlchemyElementDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut only_combinations_ = false;
        let mut already_done_ = false;
        let mut unavailable_ = false;
        if let Some(subcommand) = self.subcommand {
            match subcommand {
                Command::Display { only_combinations, already_done, unavailable, .. } => {
                    only_combinations_ = *only_combinations;
                    already_done_ = *already_done;
                    unavailable_ = *unavailable;
                },
                _ => {
                    panic!("called display() with a non-Display subcommand: {:?}", self.subcommand);
                }
            }
        }
        let mut good_combinations: Vec<&Combination> = self.inner.combinations.iter().collect();
        if !unavailable_ {
            good_combinations = good_combinations.iter().filter(| x | self.status.can_do_combination(x)).copied().collect();
        }
        if !already_done_ {
            good_combinations = good_combinations.iter().filter(| x | !self.status.history.has_combination(x)).copied().collect();
        }
        // let good_combinations: Vec<&Combination> = good_combinations.collect();
        if !only_combinations_ || !good_combinations.is_empty() {
            writeln!(f, "Element #{}: {}", self.inner.id, self.inner.name)?;
        }
        if !only_combinations_ {
            if self.inner.prime {
                writeln!(f, "Is a prime element (is present at the start of the game)")?;
            }
            if self.inner.base {
                writeln!(f, "Is a base element (can't be created from other items)")?;
            }
            if self.inner.is_final() {
                writeln!(f, "Is a final element (can't be mixed with other items)")?;
            }
            if self.inner.hidden {
                writeln!(f, "Is a hidden element (this property seems to be unused)")?;
            }
            if self.inner.is_depleted(&self.status.history) {
                writeln!(f, "Is depleted (all combinations with it have been done)")?;
            } else if self.inner.all_target_combinations_done(&self.status.history) {
                writeln!(f, "All combinations that lead to this element have been done")?;
            }
            if self.inner.all_combinations_done(self.status, &self.status.history) {
                writeln!(f, "All combinations with this element have been done (use the --already-done option to show them)")?;
            }
            writeln!(f, "{}", ConditionDisplay::new(&self.inner.condition, self.status))?;
        }
        for comb in &good_combinations {
            writeln!(f, "= {}", CombinationDisplay::new(comb, self.status))?;
        }
        if !only_combinations_ && !self.inner.can_create.is_empty() {
            writeln!(f, "Can create:")?;
            for creation in &self.inner.can_create {
                writeln!(f, "- {}", self.status.elements[*creation].name)?;
            }
        }
        if !only_combinations_ || !good_combinations.is_empty() {
            writeln!(f)?;
        }
        Ok(())
    }
}

/// A list of `Combination`s that can be displayed according to a `GameStatus`, an optional `AlchemyElement`
/// and whether we should display JavaScript commands or not.
pub struct CombinationsListDisplay<'a> {
    pub inner: &'a mut dyn CombinationsIterator,
    pub status: &'a GameStatus,
    pub javascript: bool,
    pub target_element: Option<&'a AlchemyElement>,
    pub finish_game: bool,
}

impl<'a> CombinationsListDisplay<'a> {
    pub fn new(
        combinations_list: &'a mut dyn CombinationsIterator,
        status: &'a GameStatus,
        javascript: bool,
    ) -> Self {
        Self { inner: combinations_list, status, javascript, target_element: None, finish_game: false }
    }

    pub fn new_get_combinations(
        combinations_list: &'a mut dyn CombinationsIterator,
        status: &'a GameStatus,
        javascript: bool,
        target_element: &'a AlchemyElement,
    ) -> Self {
        Self { inner: combinations_list, status, javascript, target_element: Some(target_element), finish_game: false }
    }

    pub fn new_finish_game(
        combinations_list: &'a mut dyn CombinationsIterator,
        status: &'a GameStatus,
        javascript: bool,
    ) -> Self {
        Self { inner: combinations_list, status, javascript, target_element: None, finish_game: true }
    }

    /// Display the list.
    pub fn display(&mut self) {
        if self.javascript {
            if self.inner.is_empty() {
                return;
            }
            println!(r###"localStorage.setItem("stats", '{{"firstLaunch":0,"sessionsCount":1}}');"###);
            println!(r###"localStorage.setItem("tutorials", '{{"shownText":["final","exhausted"]}}');"###);
            println!(r###"var game_history = JSON.parse(localStorage.getItem("history")) || [];"###);
            for combination in &mut *self.inner {
                println!(r###"game_history.push([{}, {}, 0]);"###, combination.0, combination.1);
            }
            println!(r###"localStorage.setItem("history", JSON.stringify(game_history));"###);
            return;
        }
        if let Some(element) = self.target_element {
            let name = &element.name;
            if self.inner.is_empty() {
                assert!(self.status.acquired_elements.contains(&element.id));
                println!("You already have the {name} in your inventory");
                return;
            }
            println!("To get the {name}, you must combine:");
        }
        if self.finish_game {
            if self.inner.is_empty() {
                println!("You already finished the game");
                return;
            }
            println!("To finish the game, you must combine:");
        }
        let mut seen_combinations: Vec<Combination> = vec![];
        // We have to use this because iterating takes ownership of all the list
        // and we need it to call combinations.has_next()
        while let Some(combination) = self.inner.next() {
            let mut next_element_str = String::new();
            // If it's not the last element, check in all the following combinations
            // if there is the result (because there can be multiple results)
            if self.inner.has_next() && self.target_element.is_some() {
                let new_elements = self.status.elements.get_from_combination(&combination);
                'outer: for el in new_elements {
                    for combination_to_try in &seen_combinations {
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
                    format_elements_list(self.status.elements.get_from_combination(&combination).into_iter()),
                );
            }

            println!("- {}{next_element_str}", CombinationDisplay::new(&combination, self.status));
            seen_combinations.push(combination);
        }
    }
}
