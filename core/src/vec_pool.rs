use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// A pool of Vec<u8> buffers, tiered by power-of-two capacity.
#[derive(Clone, Default)]
pub struct VecU8Pool {
    tiers: Rc<RefCell<BTreeMap<usize, VecDeque<Vec<u8>>>>>,
}

impl VecU8Pool {
    /// Create a new empty pool.
    pub fn new() -> Self {
        Self {
            tiers: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }

    /// Allocate a buffer of at least `size` bytes, rounded up to nearest power of two.
    pub fn allocate(&self, size: usize) -> PooledVec {
        let cap = size.next_power_of_two();
        let vec = self
            .tiers
            .borrow_mut()
            .entry(cap)
            .or_default()
            .pop_front()
            .unwrap_or_else(|| Vec::with_capacity(cap));

        PooledVec {
            vec: Some(vec),
            cap,
            pool: self.clone(),
        }
    }

    /// Return a buffer to the pool.
    fn return_to_pool(&self, mut vec: Vec<u8>, cap: usize) {
        vec.clear();
        self.tiers.borrow_mut().entry(cap).or_default().push_back(vec);
    }

    /// Inspect current pool sizes (for testing)
    pub fn tier_sizes(&self) -> BTreeMap<usize, usize> {
        self.tiers
            .borrow()
            .iter()
            .map(|(cap, deque)| (*cap, deque.len()))
            .collect()
    }
    
    // For use in tests.

    /// Allocate a buffer of at least `size` bytes and fill it with the given slice.
    pub fn from_slice(&self, data: &[u8]) -> PooledVec {
        let mut buf = self.allocate(data.len());
        buf.extend_from_slice(data);
        buf
    }

    /// Allocate a buffer from an existing Vec<u8>.
    pub fn from_vec(&self, data: Vec<u8>) -> PooledVec {
        let mut buf = self.allocate(data.len());
        buf.extend(data);
        buf
    }
}

/// A Vec<u8> from the pool, returned automatically when dropped.
pub struct PooledVec {
    vec: Option<Vec<u8>>,
    cap: usize,
    pool: VecU8Pool,
}

impl Deref for PooledVec {
    type Target = Vec<u8>;
    fn deref(&self) -> &Vec<u8> {
        self.vec.as_ref().unwrap()
    }
}

impl DerefMut for PooledVec {
    fn deref_mut(&mut self) -> &mut Vec<u8> {
        self.vec.as_mut().unwrap()
    }
}

impl Drop for PooledVec {
    fn drop(&mut self) {
        if let Some(vec) = self.vec.take() {
            self.pool.return_to_pool(vec, self.cap);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pooled_vec_rc_basic() {
        let pool = VecU8Pool::new();
        {
            // Allocate three buffers in two different tiers.
            let mut a16 = pool.allocate(10); // rounds to 16
            let mut b32 = pool.allocate(20); // rounds to 32
            let mut c16 = pool.allocate(10); // rounds to 16

            // At this point, nothing has been returned to the pool yet
            assert_eq!(pool.tier_sizes().get(&16), Some(&0));
            assert_eq!(pool.tier_sizes().get(&32), Some(&0));

            a16.push(1);
            b32.push(2);
            c16.push(3);

            // Drop `a16` and `b32`. They should be returned to the pool.
            drop(a16);
            drop(b32);

            // The Pool now has one Vec<u8> of capacity 16 (from `a16`)
            // and one Vec<u8> of capacity 32 (from `b32`).
            let sizes = pool.tier_sizes();
            assert_eq!(sizes.get(&16), Some(&1));
            assert_eq!(sizes.get(&32), Some(&1));

            // Allocate a new buffer `d16` of size 15 → rounds to 16
            // This should reuse the 16-capacity Vec that was returned from `a16`.
            let d16 = pool.allocate(15);
            assert_eq!(d16.capacity(), 16);

            // After allocating `d16`, the pool's 16-capacity deque is empty again.
            let sizes_after = pool.tier_sizes();
            assert_eq!(sizes_after.get(&16), Some(&0));
            assert_eq!(sizes_after.get(&32), Some(&1));

            // Drop `c16` which was never returned before, should go back to pool
            drop(c16);

            // Now the pool has one Vec<u8> of capacity 16 (from `c16`)
            let sizes_after_c = pool.tier_sizes();
            assert_eq!(sizes_after_c.get(&16), Some(&1));
            assert_eq!(sizes_after_c.get(&32), Some(&1));

        }
        // Drop d16 implicitly.

        // Now d16 is returned to the pool has two Vec<u8> of capacity 16
        let final_sizes = pool.tier_sizes();
        assert_eq!(final_sizes.get(&16), Some(&2));
        assert_eq!(final_sizes.get(&32), Some(&1));
    }
}
