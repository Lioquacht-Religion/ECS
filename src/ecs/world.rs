// world.rs

use std::cell::UnsafeCell;

use crate::utils::any_map::AnyMap;

use super::system::Systems;

pub struct World {
    pub data: UnsafeCell<WorldData>,
    pub systems: Systems,
}

pub struct WorldData {
    pub resources: AnyMap,
    pub components: (),
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
            components: (),
        }
    }

    pub fn add_resource<T: 'static>(&mut self, value: T) {
        self.resources.insert(value);
    }
}
