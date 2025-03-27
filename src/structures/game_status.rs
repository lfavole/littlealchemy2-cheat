use std::{collections::{hash_map::{Entry, Values}, HashMap, HashSet}, ops::{Index, IndexMut}, slice::Iter};
use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

use super::{condition::Condition, history::History, path::PathToElement, AlchemyElement, Combination};


#[derive(Debug)]
/// A list of `AlchemyElement`s.
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
    pub acquired_elements: HashSet<u16>,
    pub history: History,
}

impl GameStatus {
    pub fn new(elements: ElementsList, history: History) -> GameStatus {
        let mut ret = Self {
            elements,
            history,
            ..Default::default()
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

    fn add_prime_elements(elements: &ElementsList, acquired_elements: &mut HashSet<u16>) {
        for item in elements.iter() {
            if item.prime {
                acquired_elements.insert(item.id);
            }
        }
    }

    fn add_unlocked_elements(elements: &ElementsList, acquired_elements: &mut HashSet<u16>) {
        for item in elements.iter() {
            match &item.condition {
                Condition::None => {},
                Condition::Progress(total) => {
                    if acquired_elements.len() > *total {
                        acquired_elements.insert(item.id);
                    }
                },
                Condition::Elements(elements, min) => {
                    let mut count = 0;
                    let mut to_add = vec![];
                    for element in acquired_elements.iter() {
                        if elements.contains(element) {
                            count += 1;
                            if count >= *min {
                                to_add.push(item.id);
                                break;
                            }
                        }
                    }
                    for item in to_add {
                        acquired_elements.insert(item);
                    }
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
            self.acquired_elements.insert(element.id);
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

    pub fn finish_game(&self) -> FinishGameIterator {
        FinishGameIterator::new(self)
    }
}

pub trait CombinationsIterator: Iterator<Item = Combination> {
    fn has_next(&mut self) -> bool;

    fn is_empty(&mut self) -> bool;
}

pub struct CombinationsList<'a> {
    inner: Iter<'a, Combination>,
    index: usize,
    length: usize,
}

impl<'a> CombinationsList<'a> {
    pub fn new(combinations: &'a [Combination]) -> Self {
        Self {
            inner: combinations.iter(),
            index: 0,
            length: combinations.len(),
        }
    }
}

impl Iterator for CombinationsList<'_> {
    type Item = Combination;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        self.inner.next().map(std::borrow::ToOwned::to_owned)
    }
}

impl CombinationsIterator for CombinationsList<'_> {
    fn is_empty(&mut self) -> bool {
        self.length == 0
    }

    fn has_next(&mut self) -> bool {
        self.index < self.length
    }
}

#[derive(Debug)]
pub struct FinishGameIterator<'a> {
    index: usize,
    combinations: Vec<Combination>,
    status: &'a GameStatus,
    acquired_elements: HashSet<u16>,
    remaining_elements_to_create: HashMap<u16, HashSet<u16>>,
}

impl<'a> FinishGameIterator<'a> {
    fn new(status: &'a GameStatus) -> Self {
        let acquired_elements = status.acquired_elements.clone();
        let remaining_elements_to_create = status.elements.0.iter()
            .filter(| (k, v) | !acquired_elements.contains(k) && !v.can_create.is_empty())
            .map(| (k, v) | (*k, v.can_create.iter().copied().collect()))
            .collect();
        Self {
            index: 0,
            combinations: vec![],
            status,
            acquired_elements: status.acquired_elements.clone(),
            remaining_elements_to_create,
        }
    }

    /// Add more combinations in the stack.
    /// Return `false` if we have added all the combinations, `true` otherwise.
    fn fill_stack(&mut self) -> bool {
        if self.remaining_elements_to_create.is_empty() {
            return false;
        }
        let orig_length = self.combinations.len();
        for element_id in self.acquired_elements.clone() {
            let element = &self.status.elements[element_id];

            for created_element_id in &element.can_create {
                let created_element = &self.status.elements[*created_element_id];

                for combination in &created_element.combinations {
                    if combination.contains(self.acquired_elements.iter().copied()) {
                        if !self.combinations.contains(combination) && !self.status.history.has_combination(combination) {
                            self.combinations.push(combination.clone());
                        }
                        self.acquired_elements.insert(*created_element_id);

                        if let Entry::Occupied(mut entry) = self.remaining_elements_to_create.entry(element_id) {
                            if entry.get().contains(created_element_id) {
                                entry.get_mut().remove(created_element_id);
                                if entry.get().is_empty() {
                                    entry.remove();
                                }
                            }
                        }
                    }
                }
            }
            GameStatus::add_unlocked_elements(&self.status.elements, &mut self.acquired_elements);
        }
        assert!(self.combinations.len() - orig_length > 0, "No elements have been added when running fill_stack");
        true
    }
}

impl CombinationsIterator for FinishGameIterator<'_> {
    fn is_empty(&mut self) -> bool {
        self.combinations.is_empty() && !self.fill_stack()
    }

    fn has_next(&mut self) -> bool {
        self.index < self.combinations.len() || (self.fill_stack() && self.index < self.combinations.len())
    }
}

impl Iterator for FinishGameIterator<'_> {
    type Item = Combination;

    fn next(&mut self) -> Option<Self::Item> {
        assert!(self.index <= self.combinations.len(), "the index mustn't be strictly greater than the stack length");
        if self.index == self.combinations.len() && !self.fill_stack() {
            return None;
        }
        self.index += 1;
        Some(self.combinations[self.index - 1].clone())
    }
}
