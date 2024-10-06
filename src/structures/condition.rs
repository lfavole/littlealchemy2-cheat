use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

use crate::structures::{format_elements_list, AlchemyElement};

use super::game_status::GameStatus;

#[derive(Clone, Debug, PartialEq)]
/// A condition that needs to be fulfilled in order to unlock an element.
pub enum Condition {
    /// There is no condition.
    None,
    /// At least n elements must be discovered.
    Progress(usize),
    /// At least n elements from the list must be discovered.
    Elements(Vec<u16>, usize),
}

impl Condition {
    /// Returns `true` if there is no condition, `false` otherwise.
    pub fn is_none(&self) -> bool {
        *self == Self::None
    }

    /// Returns a formatted version of the condition according to the given `LittleAlchemy2Database`.
    pub fn display(&self, data: &GameStatus) {
        match self {
            Self::None => {},
            Self::Progress(total) => {
                println!("Will be unlocked after discovering {total} elements");
            },
            Self::Elements(elements, min) => {
                println!(
                    "Will be unlocked after discovering {} elements from those: {}",
                    min,
                    format_elements_list(elements.iter().map(| x | &data.elements[*x]).collect::<Vec<&AlchemyElement>>().as_slice()),
                );
            },
        }
    }
}

impl Default for Condition {
    fn default() -> Self {
        Self::None
    }
}

struct ConditionVisitor;
impl<'de> Visitor<'de> for ConditionVisitor {
    type Value = Condition;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a Condition object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where A: serde::de::MapAccess<'de> {
        let allowed_types: &[&'static str; 3] = &["none", "progress", "elements"];

        let mut total = None;
        let mut elements = None;
        let mut min = 1;
        let mut type_: Option<String> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => {
                    type_ = map.next_value()?;
                    if let Some(ref real_type) = type_ {
                        if !allowed_types.contains(&&real_type[..]) {
                            return Err(serde::de::Error::unknown_variant(&real_type[..], allowed_types));
                        }
                    } else {
                        return Err(serde::de::Error::missing_field("type"))
                    }
                }
                "total" => {
                    let value = map.next_value()?;
                    total = Some(value);
                }
                "elements" => {
                    let elements_str: Vec<String> = map.next_value()?;
                    let mut elements_num = vec![];
                    for element in elements_str {
                        elements_num.push(element.parse().map_err(serde::de::Error::custom)?);
                    }
                    elements = Some(elements_num);
                }
                "min" => {
                    min = map.next_value()?;
                }
                _ => {
                    return Err(serde::de::Error::unknown_field(key.as_str(), &["type", "elements", "min", "total"]));
                }
            }
        }
        if let Some(ref real_type) = type_ {
            match real_type.as_str() {
                "none" => Ok(Condition::None),
                "progress" => {
                    if let Some(real_total) = total {
                        Ok(Condition::Progress(real_total))
                    } else {
                        Err(serde::de::Error::missing_field("total"))
                    }
                },
                "elements" => {
                    if let Some(real_elements) = elements {
                        Ok(Condition::Elements(real_elements, min))
                    } else {
                        Err(serde::de::Error::missing_field("elements"))
                    }
                },
                _ => {
                    Err(serde::de::Error::unknown_variant(&real_type[..], allowed_types))
                },
            }
        } else {
            Ok(Condition::None)
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where E: serde::de::Error, {
        Ok(Condition::None)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where E: serde::de::Error, {
        Ok(Condition::None)
    }
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        deserializer.deserialize_map(ConditionVisitor)
    }
}

impl Serialize for Condition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        match self {
            Condition::None => serializer.serialize_none(),
            Condition::Progress(total) => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "progress")?;
                map.serialize_entry("total", total)?;
                map.end()
            },
            Condition::Elements(elements, min) => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "elements")?;
                map.serialize_entry("elements", elements)?;
                map.serialize_entry("min", min)?;
                map.end()
            },
        }
    }
}
