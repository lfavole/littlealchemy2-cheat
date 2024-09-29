use std::collections::{hash_map::Entry, HashMap};

use super::{condition::Condition, database::LittleAlchemy2Database, AlchemyElement, Combination};

#[derive(Clone, Debug)]
/// A wrapper for lists of `PathToCombination` objects.
pub struct PathToCombinationList<'a>(
    /// The `PathToCombination` list.
    Vec<PathToCombination<'a>>,
    /// The minimum number of combinations to get.
    usize,
);

#[derive(Clone, Debug)]
/// A container for two `PathToElement` items that represents the path to a combination.
pub struct PathToCombination<'a>(
    pub PathToElement<'a>,
    pub PathToElement<'a>,
);

impl<'a> PathToCombination<'a> {
    pub fn from(value: &Combination, data: &'a LittleAlchemy2Database) -> Self {
        Self(
            PathToElement::new(&data.elements[value.0]),
            PathToElement::new(&data.elements[value.1]),
        )
    }
}

#[derive(Clone, Debug)]
/// A container for a list of `PathWrapper`s that represents the path to an element.
pub struct PathToElement<'a> {
    pub element: &'a AlchemyElement,
}

impl<'a> PathToElement<'a> {
    pub fn new(element: &'a AlchemyElement) -> Self {
        Self { element }
    }

    fn get_path_to_combinations<'b>(&self, data: &'b LittleAlchemy2Database) -> PathToCombinationList<'b> {
        if data.acquired_elements.contains(&self.element.id) || self.element.prime {
            PathToCombinationList(vec![], 0)
        } else {
            match &self.element.condition {
                Condition::None => PathToCombinationList(
                    self.element.combinations
                    .iter()
                    .map(| x | PathToCombination::from(x, data))
                    .collect(),
                    1,
                ),
                Condition::Progress(total) => {
                    PathToCombinationList(
                        data.elements
                        .iter()
                        .flat_map(| x | &x.combinations)
                        .map(| x | PathToCombination::from(x, data))
                        .collect(),
                        *total - data.acquired_elements.len(),
                    )
                    // TODO
                },
                Condition::Elements(elements, min) => {
                    let mut combinations = self.element.combinations.clone();
                    let mut already_acquired = 0;
                    for element_id in elements {
                        if data.acquired_elements.contains(element_id) {
                            already_acquired += 1;
                            continue;
                        }
                        combinations.append(&mut data.elements[*element_id].combinations.clone());
                    }
                    assert!(*min - already_acquired > 0);
                    PathToCombinationList(combinations.iter().map(| x | PathToCombination::from(x, data)).collect(), *min - already_acquired)
                },
            }
        }
    }

    pub fn advance_one_level<'b>(
        &self,
        data: &'b LittleAlchemy2Database,
        element_to_combinations: &mut HashMap<u16, PathToCombinationList<'b>>,
        current_path: &[u16],
        recursive_history: &mut HashMap<u16, bool>,
        recursive: bool,
    ) -> Result<(), Vec<Combination>> {
        if current_path.contains(&self.element.id) {
            return Ok(());
        }
        // If there are no combinations filled in, add them and don't recurse
        if let Entry::Vacant(entry) = element_to_combinations.entry(self.element.id) {
            entry.insert(self.get_path_to_combinations(data));
            assert!(!recursive);
        } else {
            // Don't do assertions here (if an element was already filled in before, don't recurse)

            // puddle
            // = water (doesn't exist in hashmap = not recursive) + pond (not filled = not recursive)
            // = pond (alrady filled but not recursive!) + pond (same thing)
            // = ...
            recursive_history.insert(self.element.id, false);
        }
        let combinations = element_to_combinations[&self.element.id].clone();

        let min = combinations.1;
        let combs = combinations.0;
        // If there are no combinations, stop here and propagate the "error"
        if combs.is_empty() {
            assert_eq!(min, 0);
            return Err(vec![]);
        }
        // If we just filled the combinations, don't recurse and stop here
        if !recursive {
            return Ok(());
        }
        let mut counter = 0;
        let mut ret_chains = vec![];
        // Advance everything from one level
        for comb in combs {
            let id0 = comb.0.element.id;
            let id1 = comb.1.element.id;
            let mut final_chain = vec![];
            let mut matched: u8 = 0;
            let comb_0 = &comb.0;
            let comb_1 = &comb.1;
            for path_to_el in &mut [comb_0, comb_1] {
                let mut new_path = current_path.to_owned();
                new_path.push(self.element.id);
                let recursive = *recursive_history.entry(path_to_el.element.id)
                    .or_insert_with(|| element_to_combinations.contains_key(&path_to_el.element.id));
                match path_to_el.advance_one_level(data, element_to_combinations, &new_path[..], recursive_history, recursive) {
                    Ok(()) => {},
                    Err(mut chain) => {
                        final_chain.append(&mut chain);
                        matched += 1;
                    },
                }
            }
            if matched == 2 {
                final_chain.push(Combination(id0, id1));
                ret_chains.push(final_chain);
                counter += 1;
                if counter >= min {
                    return Err(ret_chains.concat());
                }
            }
        }
        Ok(())
    }
}
