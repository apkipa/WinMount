use std::{
    borrow::Borrow,
    ops::Deref,
    sync::atomic::{AtomicU32, Ordering},
};

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

/// An ASCII-caseless string slice type.
pub struct CaselessStr(str);
impl PartialEq for CaselessStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}
impl Eq for CaselessStr {}
impl PartialOrd for CaselessStr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CaselessStr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // We convert everything to lowercase for comparison
        for (x, y) in self.0.bytes().zip(other.0.bytes()) {
            let (x, y) = (x.to_ascii_lowercase(), y.to_ascii_lowercase());
            let r = x.cmp(&y);
            if r != std::cmp::Ordering::Equal {
                return r;
            }
        }
        self.0.len().cmp(&other.0.len())
    }
}
impl CaselessStr {
    pub fn new(value: &str) -> &Self {
        unsafe { std::mem::transmute(value) }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn starts_with(&self, other: &CaselessStr) -> bool {
        let other_len = other.0.len();
        if self.0.len() < other_len {
            return false;
        }
        Self::new(&self.0[..other_len]) == other
    }
}

#[derive(Clone)]
pub struct CaselessString(String);
impl PartialEq for CaselessString {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other)
    }
}
impl Eq for CaselessString {}
impl PartialOrd for CaselessString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CaselessString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deref().cmp(other)
    }
}
impl Deref for CaselessString {
    type Target = CaselessStr;

    fn deref(&self) -> &Self::Target {
        CaselessStr::new(&self.0)
    }
}
impl From<String> for CaselessString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
impl From<&str> for CaselessString {
    fn from(value: &str) -> Self {
        Self::new(value.to_owned())
    }
}
impl AsRef<CaselessStr> for CaselessString {
    fn as_ref(&self) -> &CaselessStr {
        &self
    }
}
impl Borrow<CaselessStr> for CaselessString {
    fn borrow(&self) -> &CaselessStr {
        &self
    }
}
impl CaselessString {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

pub struct CaselessU16CString(widestring::U16CString);
impl PartialEq for CaselessU16CString {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && std::iter::zip(self.0.as_slice(), other.0.as_slice()).all(|(&x, &y)| {
                match (u8::try_from(x), u8::try_from(y)) {
                    (Ok(x), Ok(y)) => x.eq_ignore_ascii_case(&y),
                    _ => false,
                }
            })
    }
}
impl Eq for CaselessU16CString {}
impl PartialOrd for CaselessU16CString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CaselessU16CString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // TODO: Fix cmp (does not correctly take case into consideration)
        self.0.cmp(&other.0)
    }
}
impl From<widestring::U16CString> for CaselessU16CString {
    fn from(value: widestring::U16CString) -> Self {
        Self::new(value)
    }
}
impl CaselessU16CString {
    pub fn new(value: widestring::U16CString) -> Self {
        Self(value)
    }
}

pub fn calculate_hash(v: &impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}
