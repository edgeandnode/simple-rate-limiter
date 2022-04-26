#[cfg(test)]
mod test;

use parking_lot::RwLock;
use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    hash::Hash,
    iter, mem,
    ops::DerefMut as _,
    sync::{
        atomic::{AtomicUsize, Ordering as MemoryOrdering},
        Arc,
    },
};

pub struct RateLimiter<K: Eq + Hash> {
    limit: usize,
    prior_slots: RwLock<VecDeque<HashMap<K, AtomicUsize>>>,
    current_slot: RwLock<HashMap<K, AtomicUsize>>,
}

impl<K: Eq + Hash> RateLimiter<K> {
    pub fn new(limit: usize, slots: usize) -> Arc<Self> {
        assert!(slots > 0);
        let rate_limiter = Arc::new(Self {
            limit,
            prior_slots: RwLock::new(VecDeque::from_iter(
                iter::repeat_with(|| HashMap::new()).take(slots - 1),
            )),
            current_slot: RwLock::default(),
        });
        rate_limiter
    }

    pub fn rotate_slots(&self) {
        // Take the current slot data. Rotate the previous slots and place the current slot at the
        // back. This automatically prunes entries that are infrequently used.
        let mut prior_slots = self.prior_slots.write();
        let front = prior_slots
            .pop_front()
            .map(|mut front| {
                front.clear();
                front
            })
            .unwrap_or_default();
        let current_slot = mem::replace(self.current_slot.write().deref_mut(), front);
        prior_slots.push_back(current_slot);
    }

    pub fn check_limited(&self, key: K) -> bool {
        // We want to avoid a situation where a maliciously overactive client can degrade the
        // ability to serve others. So we limit the contention and mutually exclusive locking
        // that can be caused by such a client. A malicious client will most often be limited based
        // on their count from prior slots, which are infrequently modified. If we need to check the
        // current slot, then the malicious client will only be able to trigger a write lock
        // acquisition up to `limit` times in the worst case for the entire window.
        let mut sum: usize = self
            .prior_slots
            .read()
            .iter()
            .map(|slot| {
                slot.get(&key)
                    .map(|counter| counter.load(MemoryOrdering::Relaxed))
                    .unwrap_or(0)
            })
            .sum();
        // Don't increment if the limit is already reached from prior slots.
        if sum > self.limit {
            return true;
        }
        sum += self.increment(key);
        sum >= self.limit
    }

    #[inline]
    fn increment(&self, key: K) -> usize {
        if let Some(map) = self.current_slot.try_read() {
            if let Some(counter) = map.get(&key) {
                return counter.fetch_add(1, MemoryOrdering::Relaxed);
            }
        }
        match self.current_slot.write().entry(key) {
            Entry::Occupied(entry) => entry.get().fetch_add(1, MemoryOrdering::Relaxed),
            Entry::Vacant(entry) => {
                entry.insert(AtomicUsize::new(1));
                1
            }
        }
    }
}
