use std::sync::atomic::{AtomicU32, Ordering};

use atomic_wait::wait;

pub fn real_wait(atomic: &AtomicU32, value: u32) {
    while atomic.load(Ordering::Acquire) == value {
        wait(atomic, value);
    }
}

pub const fn parse_u32(s: &str) -> u32 {
    let mut bytes = s.as_bytes();
    let mut val = 0;
    while let [byte, rest @ ..] = bytes {
        assert!(b'0' <= *byte && *byte <= b'9', "invalid digit");
        val = val * 10 + (*byte - b'0') as u32;
        bytes = rest;
    }
    val
}

// mod str_uuid {
//     use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
//     use std::str::FromStr;
//     use uuid::Uuid;

//     pub fn serialize<S>(val: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         val.to_string().serialize(serializer)
//     }

//     pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let val: &str = Deserialize::deserialize(deserializer)?;
//         Uuid::from_str(val).map_err(D::Error::custom)
//     }
// }
