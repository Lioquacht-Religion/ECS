use std::{collections::HashMap, hash::Hash};

#[derive(Debug)]
pub enum SplitError<'map, V> {
    SameKey(&'map mut V),
    OnlyOneValue(&'map mut V),
    NoValueFound,
}

pub trait SplitMut<K: Eq, V> {
    fn split_mut2<'a>(
        &'a mut self,
        key1: &K,
        key2: &K,
    ) -> Result<(&'a mut V, &'a mut V), SplitError<'a, V>>;
}

impl<K: Eq + Hash, V> SplitMut<K, V> for HashMap<K, V> {
    fn split_mut2<'a>(
        &'a mut self,
        key1: &K,
        key2: &K,
    ) -> Result<(&'a mut V, &'a mut V), SplitError<'a, V>> {
        if key1 == key2 {
            return match self.get_mut(key1) {
                Some(val) => Err(SplitError::SameKey(val)),
                None => Err(SplitError::NoValueFound),
            };
        }
        let val1 = self.get_mut(key1).map(|v| v as *mut V);
        let val2 = self.get_mut(key2).map(|v| v as *mut V);

        match (val1, val2) {
            (Some(val1), Some(val2)) => unsafe { Ok((&mut *val1, &mut *val2)) },
            (Some(val1), None) => unsafe { Err(SplitError::OnlyOneValue(&mut *val1)) },
            (None, Some(val2)) => unsafe { Err(SplitError::OnlyOneValue(&mut *val2)) },
            (None, None) => Err(SplitError::NoValueFound),
        }
    }
}
