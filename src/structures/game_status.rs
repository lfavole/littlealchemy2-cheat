use std::{collections::{hash_map::{Entry, Values}, HashMap}, ops::{Index, IndexMut}};
use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

use super::{condition::Condition, history::History, path::PathToElement, AlchemyElement, Combination};


#[derive(Debug)]
/// A list of `AlchemyElement`s.
///
/// This is different from the `LittleAlchemy2Database` struct as it doesn't contain
/// information like acquired elements.
pub struct ElementsList(pub HashMap<u16, AlchemyElement>);
impl ElementsList {
    /// Returns an empty `ElementsList`.
    pub fn new() -> ElementsList {
        ElementsList(HashMap::new())
    }

    /// Iterate over the elements in this list.
    pub fn iter(&self) -> Values<'_, u16, AlchemyElement> {
        self.0.values()
    }

    /// Return the number of elements in this list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return the item that matches the given `Combination`.
    pub fn get_from_combination(&self, combination: &Combination) -> Vec<&AlchemyElement> {
        self.iter().filter(move | x | x.combinations.contains(combination)).collect()
    }

    /// Return a reference to the element corresponding to the index.
    pub fn get(&self, index: u16) -> Option<&AlchemyElement> {
        self.0.get(&index)
    }

    /// Return a mutable reference to the element corresponding to the index.
    pub fn get_mut(&mut self, index: u16) -> Option<&mut AlchemyElement> {
        self.0.get_mut(&index)
    }
}

pub(crate) struct ElementsListVisitor;

impl<'de> Visitor<'de> for ElementsListVisitor {
    type Value = ElementsList;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an ElementsList object (list of AlchemyElement objects)")
    }

    fn visit_map<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where A: serde::de::MapAccess<'de> {
        let mut db = ElementsList::new();
        while let Some(key) = seq.next_key()? {
            if let Some(mut value) = seq.next_value::<Option<AlchemyElement>>()? {
                value.id = key;
                db.0.insert(key, value);
            } else {
                break;
            }
        }
        Ok(db)
    }
}

impl<'de> Deserialize<'de> for ElementsList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        deserializer.deserialize_map(ElementsListVisitor)
    }
}

impl Serialize for ElementsList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for value in self.0.values() {
            map.serialize_entry(&value.id, value)?;
        }
        map.end()
    }
}

impl Default for ElementsList {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<u16> for ElementsList {
    type Output = AlchemyElement;

    fn index(&self, index: u16) -> &Self::Output {
        self.0.get(&index).unwrap()
    }
}

impl IndexMut<u16> for ElementsList {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        self.0.get_mut(&index).unwrap()
    }
}

#[derive(Debug, Default)]
pub struct GameStatus {
    pub elements: ElementsList,
    pub acquired_elements: Vec<u16>,
    pub history: History,
}

impl GameStatus {
    pub fn new(elements: ElementsList, history: History) -> GameStatus {
        let mut ret = Self {
            elements,
            acquired_elements: vec![],
            history,
        };
        ret.check();
        ret
    }

    pub fn check(&mut self) {
        Self::add_prime_elements(&self.elements, &mut self.acquired_elements);
        Self::add_unlocked_elements(&self.elements, &mut self.acquired_elements);
        self.check_can_create();
        self.check_final();
    }

    fn add_prime_elements(elements: &ElementsList, acquired_elements: &mut Vec<u16>) {
        for item in elements.iter() {
            if item.prime {
                acquired_elements.push(item.id);
            }
        }
    }

    fn add_unlocked_elements(elements: &ElementsList, acquired_elements: &mut Vec<u16>) {
        for item in elements.iter() {
            match &item.condition {
                Condition::None => {},
                Condition::Progress(total) => {
                    if acquired_elements.len() > *total {
                        acquired_elements.push(item.id);
                    }
                },
                Condition::Elements(elements, min) => {
                    let mut count = 0;
                    let mut to_add = vec![];
                    for element in acquired_elements.iter_mut() {
                        if elements.contains(element) {
                            count += 1;
                            if count >= *min {
                                to_add.push(item.id);
                                break;
                            }
                        }
                    }
                    acquired_elements.append(&mut to_add);
                },
            }
        }
    }

    fn check_can_create(&self) {
        let mut can_create: HashMap<u16, Vec<u16>> = HashMap::new();
        for item in self.elements.iter() {
            for comb in &item.combinations {
                can_create.entry(comb.0).or_default().push(item.id);
                can_create.entry(comb.1).or_default().push(item.id);
            }
        }
        for item in self.elements.iter() {
            if let Some(can_create_ok) = can_create.get_mut(&item.id) {
                can_create_ok.sort_unstable();
                can_create_ok.dedup();
                assert!(item.can_create == *can_create_ok, "can_create mismatch: expected {:?}, found {:?}", can_create[&item.id], item.can_create);
            }
        }
    }

    fn check_final(&self) {
        for item in self.elements.iter() {
            if item.is_final() {
                assert!(item.can_create.is_empty());
            }
        }
    }

    pub fn combine(&mut self, combination: &Combination) {
        let combinations = self.elements.get_from_combination(combination);
        if combinations.is_empty() {
            println!("warning: combination between {} and {} doesn't exist", combination.0, combination.1);
        }
        for element in combinations {
            if !self.acquired_elements.contains(&element.id) {
                self.acquired_elements.push(element.id);
            }
            assert!(
                element.combinations.iter().any(| comb | comb == combination),
                "combination between {} and {} found before, but not found again",
                combination.0,
                combination.1,
            );
        }
    }

    pub fn can_do_combination(&self, combination: &Combination) -> bool {
        self.acquired_elements.contains(&combination.0) && self.acquired_elements.contains(&combination.1)
    }

    pub fn obtain(&self, element_id: u16) -> Vec<Combination> {
        let path = PathToElement::new(&self.elements[element_id]);
        let mut element_to_combinations = HashMap::new();
        let mut recursive = false;
        loop {
            match path.advance_one_level(self, &mut element_to_combinations, &[], &mut HashMap::new(), recursive) {
                Ok(()) => {},
                Err(x) => {return x;},
            }
            recursive = true;
        }
    }

    pub fn finish_game(&self) -> Vec<Combination> {
        let mut combinations = vec![];
        let mut acquired_elements = self.acquired_elements.clone();
        let mut remaining_elements_to_create = HashMap::new();
        remaining_elements_to_create.extend(
            self.elements.0.iter()
            .filter(| (k, v) | !acquired_elements.contains(k) && !v.can_create.is_empty())
            .map(| (k, v) | (*k, v.can_create.clone()))
            .collect::<Vec<_>>()
        );

        while !remaining_elements_to_create.is_empty() {
            for element_id in acquired_elements.clone() {
                let element = &self.elements[element_id];

                for created_element_id in &element.can_create {
                    let created_element = &self.elements[*created_element_id];

                    for combination in &created_element.combinations {
                        if combination.contains(&acquired_elements) {
                            if !combinations.contains(combination) && !self.history.has_combination(combination) {
                                combinations.push(combination.clone());
                            }
                            if !acquired_elements.contains(created_element_id) {
                                acquired_elements.push(*created_element_id);
                            }

                            if let Entry::Occupied(mut entry) = remaining_elements_to_create.entry(element_id) {
                                if let Ok(index) = entry.get().binary_search(created_element_id) {
                                    entry.get_mut().swap_remove(index);
                                    if entry.get().is_empty() {
                                        entry.remove();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Self::add_unlocked_elements(&self.elements, &mut acquired_elements);
        }

        combinations
    }
}
