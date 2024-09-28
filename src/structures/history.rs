use std::slice::Iter;
use chrono::{DateTime, NaiveDateTime};
use serde::{de::Visitor, ser::SerializeSeq, Deserialize, Serialize};

use super::Combination;

#[derive(Clone, Debug)]
pub struct HistoryItem {
    pub combination: Combination,
    pub datetime: NaiveDateTime,
}

pub(crate) struct HistoryItemVisitor;

impl<'de> Visitor<'de> for HistoryItemVisitor {
    type Value = HistoryItem;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a HistoryItem object (a, b, time)")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, <A as serde::de::SeqAccess<'de>>::Error>
    where A: serde::de::SeqAccess<'de> {
        let a: u16;
        let b: u16;
        let datetime: NaiveDateTime;
        if let Some(a_str) = seq.next_element::<String>()? {
            a = a_str.parse().map_err(serde::de::Error::custom)?;
        } else {
            return Err(serde::de::Error::missing_field("a"));
        }
        if let Some(b_str) = seq.next_element::<String>()? {
            b = b_str.parse().map_err(serde::de::Error::custom)?;
        } else {
            return Err(serde::de::Error::missing_field("b"));
        }
        if let Some(datetime_ms) = seq.next_element::<i64>()? {
            // Copied from chrono
            datetime = DateTime::from_timestamp_millis(datetime_ms)
                .map(|dt| dt.naive_utc())
                .ok_or_else(|| serde::de::Error::custom(format!("value is not a legal timestamp: {datetime_ms}")))?;
        } else {
            return Err(serde::de::Error::missing_field("time"));
        }
        Ok(HistoryItem {
            combination: Combination(a, b),
            datetime
        })
    }
}

impl<'de> Deserialize<'de> for HistoryItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        deserializer.deserialize_struct("HistoryItem", &["a", "b", "time"], HistoryItemVisitor)
    }
}

impl Serialize for HistoryItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.combination.0)?;
        seq.serialize_element(&self.combination.1)?;
        seq.serialize_element(&self.datetime.and_utc().timestamp())?;
        seq.end()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(from = "Vec<HistoryItem>", into = "Vec<HistoryItem>")]
pub struct History(pub Vec<HistoryItem>);

impl From<Vec<HistoryItem>> for History {
    fn from(value: Vec<HistoryItem>) -> Self {
        Self(value)
    }
}

impl From<History> for Vec<HistoryItem> {
    fn from(val: History) -> Self {
        val.0
    }
}

impl History {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> Iter<'_, HistoryItem> {
        self.0.iter()
    }

    pub fn has_combination(&self, combination: &Combination) -> bool {
        self.iter().any(| x | x.combination == *combination)
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}
