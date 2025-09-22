// scheduler.rs

use std::{
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
};

use crate::ecs::{
    system::{SystemId, Systems},
    world::WorldData,
};

struct Constraint {
    system_id: SystemId,
    after: HashSet<SystemId>,
    before: HashSet<SystemId>,
}

pub(crate) trait Scheduler {
    fn execute(&mut self, systems: &mut Systems, world_data: &UnsafeCell<WorldData>);
}

pub(crate) struct SingleThreadScheduler {
    schedule: Vec<SystemId>,
    constraints: HashMap<SystemId, Constraint>,
}

impl Scheduler for SingleThreadScheduler {
    fn execute(&mut self, systems: &mut Systems, world_data: &UnsafeCell<WorldData>) {
        for system_id in self.schedule.iter() {
            systems.run_system(*system_id, &world_data);
        }
    }
}

pub(crate) struct ParallelScheduler {
    schedule: Vec<Vec<SystemId>>,
}
