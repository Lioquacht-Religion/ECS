// typeid_map.rs

use std::{any::TypeId, collections::HashMap};

pub struct TypeIdMap<V> {
    map: HashMap<TypeId, V>,
}

impl<V> TypeIdMap<V> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn insert_typed<K: 'static>(&mut self, value: V) {
        self.map.insert(TypeId::of::<K>(), value);
    }
    pub fn get_typed<K: 'static>(&self) -> Option<&V> {
        self.map.get(&TypeId::of::<K>())
    }
    pub fn get_map_mut(&mut self) -> &mut HashMap<TypeId, V> {
        &mut self.map
    }
}
