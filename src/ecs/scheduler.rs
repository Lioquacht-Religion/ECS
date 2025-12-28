// scheduler.rs

use std::{cell::UnsafeCell, collections::HashSet};

use crate::ecs::{
    system::{SystemId, Systems},
    world::WorldData,
};

pub(crate) trait Scheduler {
    fn execute(&mut self, systems: &mut Systems, world_data: &UnsafeCell<WorldData>);
}

pub(crate) struct SingleThreadScheduler {
    pub(crate) schedule: Vec<SystemId>,
}

impl SingleThreadScheduler {
    pub(crate) fn new() -> Self {
        Self {
            schedule: Vec::new(),
        }
    }
}

impl Scheduler for SingleThreadScheduler {
    fn execute(&mut self, systems: &mut Systems, world_data: &UnsafeCell<WorldData>) {
        let mut to_run_systems = self.schedule.clone();
        let mut finished_systems: HashSet<SystemId> = HashSet::new();
        let mut cur_sys_ind: usize = 0;

        let max_not_exec_system_count = to_run_systems.len() * 2;
        let mut not_exec_system_count = 0;

        while to_run_systems.len() > 0 {
            let sysid = to_run_systems[cur_sys_ind];
            let run_system: bool = if let Some(constraint) = systems.constraints.get(&sysid) {
                constraint.after.is_subset(&finished_systems)
            } else {
                true
            };

            if run_system {
                systems.run_system(sysid, &world_data);
                let removed_sysid = to_run_systems.remove(cur_sys_ind);
                finished_systems.insert(removed_sysid);
                if to_run_systems.len() >= cur_sys_ind && cur_sys_ind > 0 {
                    cur_sys_ind = 0;
                }
                not_exec_system_count = 0;
            } else {
                cur_sys_ind += 1;
                if cur_sys_ind >= to_run_systems.len() {
                    cur_sys_ind = 0;
                }
                not_exec_system_count += 1;
            }
            if not_exec_system_count >= max_not_exec_system_count {
                panic!("System scheduling loop detected!")
            }
        }
    }
}

pub(crate) struct ParallelScheduler {
    schedule: Vec<Vec<SystemId>>,
}
