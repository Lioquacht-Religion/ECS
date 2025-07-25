// world.rs

use std::{cell::UnsafeCell, collections::HashMap};

use crate::utils::{any_map::AnyMap, sorted_vec::SortedVec};

use super::{
    component::{ComponentId, EntityStorage},
    query::QueryState,
    system::Systems,
};

pub struct World {
    pub data: UnsafeCell<WorldData>,
    pub systems: Systems,
}

pub struct WorldData {
    pub(crate) resources: AnyMap,
    pub entity_storage: EntityStorage,
    pub(crate) query_data: HashMap<SortedVec<ComponentId>, QueryState>,
}

impl World {
    pub fn new() -> Self {
        World {
            data: WorldData::new().into(),
            systems: Systems::new(),
        }
    }

    pub fn run(&mut self) {
        self.systems.run_systems(&mut self.data);
    }
}

impl WorldData {
    pub fn new() -> Self {
        WorldData {
            resources: AnyMap::new(),
            entity_storage: EntityStorage::new(),
            query_data: HashMap::new(),
        }
    }

    pub fn add_resource<T: 'static>(&mut self, value: T) {
        self.resources.insert(value);
    }
}
