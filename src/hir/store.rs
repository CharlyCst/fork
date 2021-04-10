//! # Store
//!
//! A store is a struct holding a map of items associated to globally unique identifiers. Stores
//! are used to keep track of names, types or even structures through the HIR passes and the IDs
//! are then used as global identifiers to access a resource in the Ctx.
//!
//! In order to guarantee that identifiers are unique a single Store per Module ID per kind of ID
//! must be built. Store elements can be transformed using the `transmute` method while conserving
//! their previous IDs.

use crate::ctx::ModId;
use std::collections::{HashMap, HashSet};

use zephyr_lang_derive::Identifier;

// ——————————————————————————— A few kinds of IDs ——————————————————————————— //

pub type Id = u64;

/// A trait implemented by an Identifier type (a type capable of producing an Id)
pub trait Identifier {
    fn new(id: Id) -> Self;
}

/// An helper macro to define new IDs
macro_rules! define_id {
    ($i: ident) => {
        #[derive(Identifier, Eq, PartialEq, Hash, Copy, Clone, Debug, Ord, PartialOrd)]
        pub struct $i(Id);

        impl std::fmt::Display for $i {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id!(FunId);
define_id!(DataId);
define_id!(StructId);
define_id!(TupleId);
define_id!(TypeId);
define_id!(TypeVarId);

/// A list of IDs for known funs and strucs.
pub mod known_ids {
    use super::*;

    // Functions
    pub const MALLOC_ID: FunId = FunId(1);

    // Structs
    pub const STR_ID: StructId = StructId(1);
}

// ———————————————————————————— Store definition ———————————————————————————— //

pub struct Store<I, T> {
    mod_id: ModId,
    counter: u32,
    data: HashMap<I, T>,
    merged_mods: HashSet<ModId>,
}

impl<I, T> Store<I, T>
where
    I: Identifier + Clone + Eq + std::hash::Hash,
{
    pub fn new(mod_id: ModId) -> Self {
        Self {
            mod_id,
            counter: 0,
            data: HashMap::new(),
            merged_mods: HashSet::new(),
        }
    }

    /// Creates a new store that can hold at least `capacity` items without re-allocating.
    pub fn with_capacity(mod_id: ModId, capacity: usize) -> Self {
        Self {
            mod_id,
            counter: 0,
            data: HashMap::with_capacity(capacity),
            merged_mods: HashSet::new(),
        }
    }

    /// Add an item to the store, return a globally unique ID that identifies this item.
    #[allow(dead_code)]
    pub fn add(&mut self, item: T) -> I {
        let id = self.fresh_id();
        self.data.insert(id.clone(), item);
        id
    }

    pub fn insert(&mut self, id: I, item: T) {
        self.data.insert(id, item);
    }

    /// Tries to retrieve an item from its ID.
    ///
    /// This will never return None if the ID has been generated by this store.
    pub fn get(&self, id: I) -> Option<&T> {
        self.data.get(&id)
    }

    /// Generates a globally unique ID for this kind of store.
    pub fn fresh_id(&mut self) -> I {
        let id = (self.counter as u64) + ((self.mod_id as u64) << 32);
        self.counter = self
            .counter
            .checked_add(1)
            .expect("[Internal Error] Unable to generate a unique ID");
        I::new(id)
    }

    /// Extend this store with the Key-Values pairs of another one.
    pub fn extend(&mut self, other: Self) {
        if other.mod_id == self.mod_id {
            panic!("Store with the same module ID should never be merged!");
        } else if self.merged_mods.contains(&other.mod_id) {
            panic!("A store with the same module ID has already been merged!");
        }
        self.data.extend(other.data);
        self.merged_mods.insert(other.mod_id);
    }

    /// Transform a `Store<I, T>` into `Store<I, Q>` by applying a function to all its elements.
    ///
    /// If the transformation function returns None, the item is dropped.
    pub fn transmute<Q, F>(self, mut fun: F) -> Store<I, Q>
    where
        F: FnMut(T) -> Option<Q>,
    {
        let mut data = HashMap::with_capacity(self.data.len());
        for (id, item) in self.data.into_iter() {
            if let Some(transmuted_item) = fun(item) {
                data.insert(id, transmuted_item);
            }
        }

        Store {
            mod_id: self.mod_id,
            counter: self.counter,
            merged_mods: self.merged_mods,
            data,
        }
    }

    /// Iterates over (id, item) tuples.
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, I, T> {
        self.data.iter()
    }
}

impl<I, T> IntoIterator for Store<I, T> {
    type Item = (I, T);
    type IntoIter = std::collections::hash_map::IntoIter<I, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

// ————————————————————————————————— Tests —————————————————————————————————— //

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Identifier, Eq, PartialEq, Hash, Debug, Copy, Clone)]
    pub struct TestId(Id);

    #[test]
    fn store() {
        let mut store = Store::new(1);
        let id: TestId = store.add('a');
        let other_id = store.add('b');
        assert_ne!(id, other_id);
        assert_eq!(store.get(id), Some(&'a'));
        assert_eq!(store.get(other_id), Some(&'b'));
    }
}
