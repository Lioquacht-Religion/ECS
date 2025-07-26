// cache.rs

use std::{any::TypeId, ptr::NonNull};

use crate::ecs::component::ComponentId;

pub(crate) struct CollectionCache<T: Cachable> {
    vec: Vec<T>,
}

impl<T: Cachable> CollectionCache<T> {
    pub(crate) fn new() -> Self {
        Self { vec: Vec::new() }
    }

    pub(crate) fn take_cached(&mut self) -> T {
        match self.vec.pop() {
            Some(elem) => {
                elem
            }
            None => T::default(),
        }
    }

    pub(crate) fn insert(&mut self, mut to_cache: T) {
        to_cache.reset();
        self.vec.push(to_cache);
    }
}

pub(crate) trait Cachable: Default {
    fn reset(&mut self);
}

impl<T> Cachable for Vec<T> {
    fn reset(&mut self) {
        self.clear();
    }
}

pub(crate) struct EntityStorageCache {
    pub(crate) ptr_vec_cache: CollectionCache<Vec<NonNull<u8>>>,
    pub(crate) compid_vec_cache: CollectionCache<Vec<ComponentId>>,
    pub(crate) typeid_vec_cache: CollectionCache<Vec<TypeId>>,
}

impl EntityStorageCache {
    pub(crate) fn new() -> Self {
        Self {
            ptr_vec_cache: CollectionCache::new(),
            compid_vec_cache: CollectionCache::new(),
            typeid_vec_cache: CollectionCache::new(),
        }
    }
}
