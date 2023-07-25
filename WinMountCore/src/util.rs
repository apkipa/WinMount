use std::sync::atomic::{AtomicU32, Ordering};

use atomic_wait::wait;

pub fn real_wait(atomic: &AtomicU32, value: u32) {
    while atomic.load(Ordering::Acquire) == value {
        wait(atomic, value);
    }
}
