//sorted_vec.rs
//

use std::{hash::Hash, slice::Iter};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct SortedVec<T: Ord + Eq + Hash> {
    vec: Vec<T>,
}

impl<T: Ord + Eq + Hash> SortedVec<T> {
    pub fn new() -> Self {
        Self { vec: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            vec: Vec::with_capacity(cap),
        }
    }

    pub fn get_vec(&self) -> &[T] {
        &self.vec
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.vec.iter()
    }

    pub fn check_duplicates(&self) -> Option<&T> {
        for i in 0..(self.get_vec().len() - 1) {
            let elem1 = &self.get_vec()[i];
            let elem2 = &self.get_vec()[i + 1];
            if elem1 == elem2 {
                return Some(elem2);
            }
        }
        None
    }
}

impl<T: Ord + Eq + Hash> From<Vec<T>> for SortedVec<T> {
    fn from(mut value: Vec<T>) -> Self {
        value.sort_unstable();
        SortedVec { vec: value }
    }
}

impl<T: Ord + Eq + Hash> From<SortedVec<T>> for Vec<T> {
    fn from(value: SortedVec<T>) -> Self {
        value.vec
    }
}
