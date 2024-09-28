pub mod number_as_str {
    use serde::{self, Deserialize, Deserializer, Serializer};

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(number: &u16, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&number.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u16, D::Error>
    where D: Deserializer<'de> {
        let s: String = String::deserialize(deserializer)?;
        s.parse::<u16>().map_err(serde::de::Error::custom)
    }
}

pub mod number_list_as_str_list {
    use serde::{self, Deserialize, Deserializer, Serializer, ser::SerializeSeq};

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(list: &Vec<u16>, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(list.len()))?;
        for item in list {
            seq.serialize_element(&item.to_string())?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u16>, D::Error>
    where D: Deserializer<'de> {
        let list: Vec<String> = Vec::deserialize(deserializer)?;
        let mut ret: Vec<u16> = vec![];
        for item in list {
            ret.push(item.parse().map_err(serde::de::Error::custom)?);
        }
        Ok(ret)
    }
}
