//! Open-addressed probing, shared by the node intern table and the symbol table.
//!
//! A slot holds `entry + 1` so that zero means vacant. Entries are bounded by
//! the id payload width, so the increment cannot overflow.

const INITIAL_SLOTS: usize = 64;
const LOAD_NUMERATOR: usize = 7;
const LOAD_DENOMINATOR: usize = 10;

/// The outcome of a probe: the entry stored under the key, or the slot it would
/// occupy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Probe {
    Found(u32),
    Vacant(usize),
}

pub(crate) struct ProbeTable {
    slots: Box<[u32]>,
    mask: usize,
    len: usize,
}

impl Default for ProbeTable {
    fn default() -> Self {
        ProbeTable {
            slots: vec![0u32; INITIAL_SLOTS].into_boxed_slice(),
            mask: INITIAL_SLOTS - 1,
            len: 0,
        }
    }
}

impl ProbeTable {
    /// Walk the probe sequence for `hash` until `eq` accepts an entry or a
    /// vacant slot is reached. The load factor is held below one, so the walk
    /// terminates.
    pub(crate) fn probe(&self, hash: u32, eq: impl Fn(u32) -> bool) -> Probe {
        let mut index = hash as usize & self.mask;
        loop {
            let slot = self.slots[index];
            if slot == 0 {
                return Probe::Vacant(index);
            }
            let entry = slot - 1;
            if eq(entry) {
                return Probe::Found(entry);
            }
            index = (index + 1) & self.mask;
        }
    }

    /// Write `entry` into a slot a preceding probe reported vacant.
    pub(crate) fn occupy(&mut self, slot: usize, entry: u32) {
        debug_assert_eq!(self.slots[slot], 0);
        self.slots[slot] = entry + 1;
        self.len += 1;
    }

    pub(crate) fn needs_grow(&self) -> bool {
        self.len * LOAD_DENOMINATOR >= self.slots.len() * LOAD_NUMERATOR
    }

    /// Double the table, recomputing each entry's hash through `rehash`.
    pub(crate) fn grow(&mut self, rehash: impl Fn(u32) -> u32) {
        let count = self.slots.len() * 2;
        let mask = count - 1;
        let mut slots = vec![0u32; count].into_boxed_slice();
        for &slot in &self.slots {
            if slot == 0 {
                continue;
            }
            let mut index = rehash(slot - 1) as usize & mask;
            while slots[index] != 0 {
                index = (index + 1) & mask;
            }
            slots[index] = slot;
        }
        self.slots = slots;
        self.mask = mask;
    }

    pub(crate) fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn heap_bytes(&self) -> usize {
        self.slots.len() * size_of::<u32>()
    }
}

#[cfg(test)]
mod tests {
    use super::{Probe, ProbeTable};

    /// Entries are indices into this vector; the key is the value stored there.
    fn insert(table: &mut ProbeTable, keys: &mut Vec<u32>, key: u32) -> u32 {
        let hash = key.wrapping_mul(0x9e37_79b9);
        let probe = {
            let seen: &[u32] = keys;
            table.probe(hash, |e| seen[e as usize] == key)
        };
        match probe {
            Probe::Found(entry) => entry,
            Probe::Vacant(slot) => {
                let entry = keys.len() as u32;
                keys.push(key);
                table.occupy(slot, entry);
                if table.needs_grow() {
                    let seen: &[u32] = keys;
                    table.grow(|e| seen[e as usize].wrapping_mul(0x9e37_79b9));
                }
                entry
            }
        }
    }

    #[test]
    fn repeated_keys_return_one_entry() {
        let mut table = ProbeTable::default();
        let mut keys = Vec::new();
        let first = insert(&mut table, &mut keys, 42);
        let second = insert(&mut table, &mut keys, 42);
        assert_eq!(first, second);
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn distinct_keys_survive_growth() {
        let mut table = ProbeTable::default();
        let mut keys = Vec::new();
        for key in 0..5_000u32 {
            insert(&mut table, &mut keys, key);
        }
        assert_eq!(keys.len(), 5_000);
        assert!(table.slot_count() > 5_000);
        for key in 0..5_000u32 {
            assert_eq!(insert(&mut table, &mut keys, key), key);
        }
        assert_eq!(keys.len(), 5_000);
    }
}
