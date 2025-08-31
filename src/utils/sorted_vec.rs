//sorted_vec.rs
//

use std::{hash::Hash, slice::Iter};

#[derive(Clone, Debug, Hash, Eq)]
pub struct SortedVec<T: Ord + Eq + Hash> {
    vec: Vec<T>,
}

impl<T: Ord + Eq + Hash> PartialEq for SortedVec<T> {
    fn eq(&self, other: &Self) -> bool {
        let v1 = self.get_vec();
        let v2 = other.get_vec();
        if v1.len() != v2.len() {
            return false;
        }

        for i in 0..v1.len() {
            if v1[i] != v2[i] {
                return false;
            }
        }

        return true;
    }
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

    pub fn is_subset_of2(&self, other: &SortedVec<T>) -> bool {
        let mut wholeset_iter = other.iter();
        let mut wholeset_elem_opt = wholeset_iter.next();
        let mut contains_count = 0;
        for subset_elem in self.iter() {
            if let Some(wholeset_elem) = wholeset_elem_opt {
                if subset_elem == wholeset_elem {
                    contains_count += 1;
                    wholeset_elem_opt = wholeset_iter.next();
                }
            } else {
                return false;
            }
        }
        contains_count == self.get_vec().len()
    }
    pub fn is_subset_of(&self, wholeset: &SortedVec<T>) -> bool {
        let subset_iter = self.iter();
        let mut contains_count = 0;

        if self.vec.len() > wholeset.vec.len() {
            return false;
        }

        for el in subset_iter {
            for el2 in wholeset.iter() {
                if el == el2 {
                    contains_count += 1;
                    continue;
                }
            }
        }

        println!(
            "contains count: {}; subset len: {}; wholeset len: {}",
            contains_count,
            self.get_vec().len(),
            wholeset.vec.len()
        );

        contains_count == self.get_vec().len()
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
