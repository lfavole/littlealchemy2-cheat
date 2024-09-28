use std::{collections::{hash_map::Values, HashMap}, ops::{Index, IndexMut}};
use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

use super::{condition::Condition, path::PathToElement, AlchemyElement, Combination};


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

#[derive(Debug)]
pub struct LittleAlchemy2Database {
    pub elements: ElementsList,
    pub acquired_elements: Vec<u16>,
}

impl LittleAlchemy2Database {
    pub fn new(elements: ElementsList) -> LittleAlchemy2Database {
        let mut ret = Self {
            elements,
            acquired_elements: vec![],
        };
        ret.check();
        ret
    }
}

pub(crate) struct DBVisitor;

impl<'de> Visitor<'de> for DBVisitor {
    type Value = LittleAlchemy2Database;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a LittleAlchemy2Database object (list of AlchemyElement objects)")
    }

    fn visit_map<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where A: serde::de::MapAccess<'de> {
        let mut db = LittleAlchemy2Database::new(ElementsList::new());
        while let Some(key) = seq.next_key()? {
            if let Some(mut value) = seq.next_value::<Option<AlchemyElement>>()? {
                value.id = key;
                db.elements.0.insert(key, value);
            } else {
                break;
            }
        }
        // Manually perform the checks
        db.check();
        Ok(db)
    }
}

impl<'de> Deserialize<'de> for LittleAlchemy2Database {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        deserializer.deserialize_map(DBVisitor)
    }
}

impl Serialize for LittleAlchemy2Database {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut map = serializer.serialize_map(Some(self.elements.len()))?;
        for value in self.elements.iter() {
            map.serialize_entry(&value.id, value)?;
        }
        map.end()
    }
}

impl LittleAlchemy2Database {
    pub fn check(&mut self) {
        self.add_prime_elements();
        self.add_unlocked_elements();
        self.check_can_create();
        self.check_final();
    }

    fn add_prime_elements(&mut self) {
        for item in self.elements.iter() {
            if item.prime {
                self.acquired_elements.push(item.id);
            }
        }
    }

    fn add_unlocked_elements(&mut self) {
        for item in self.elements.iter() {
            match &item.condition {
                Condition::None => {},
                Condition::Progress(total) => {
                    if self.acquired_elements.len() > *total {
                        self.acquired_elements.push(item.id);
                    }
                },
                Condition::Elements(elements, min) => {
                    let mut count = 0;
                    for element in &self.acquired_elements {
                        if elements.contains(element) {
                            count += 1;
                            if count >= *min {
                                self.acquired_elements.push(item.id);
                                break;
                            }
                        }
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
            if item.final_ {
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
        loop {
            match path.advance_one_level(self, &mut element_to_combinations) {
                Ok(()) => {},
                Err(x) => {return x;},
            }
        }
    }
}
