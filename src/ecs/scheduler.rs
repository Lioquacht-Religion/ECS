// scheduler.rs

use std::{cell::UnsafeCell, collections::HashSet};

use crate::{
    ecs::{
        system::{System, SystemId, SystemParamId, Systems},
        world::{SharedWorldData, WorldData},
    },
    utils::threadpool::ThreadPool,
};

pub(crate) trait Scheduler {
    fn init_schedule(&mut self, systems: &Systems);
    fn execute(&mut self, systems: &mut Systems, world_data: &mut UnsafeCell<WorldData>);
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
    fn init_schedule(&mut self, systems: &Systems) {
        (0..systems.system_vec.len()).for_each(|n| self.schedule.push(n.into()));
    }
    fn execute(&mut self, systems: &mut Systems, world_data: &mut UnsafeCell<WorldData>) {
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
                systems.run_system(sysid, world_data.get_mut());
                // execute commands after each sequential system run immediately
                // so that the next running system has the most up to date world state
                world_data.get_mut().execute_commands();
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

//TODO: add parallel execution and scheduling
pub(crate) struct ParallelScheduler {
    schedule: Vec<HashSet<SystemId>>,
    thread_pool: ThreadPool,
}

impl ParallelScheduler {
    pub(crate) fn new(thread_count: usize) -> Self {
        Self {
            schedule: Vec::new(),
            thread_pool: ThreadPool::new(thread_count),
        }
    }

    fn print_schedule(systems: &Systems, schedule: &[HashSet<SystemId>]) {
        println!("System schedule:");
        for (i, batch) in schedule.iter().enumerate() {
            let batch_str: String = batch
                .iter()
                .map(|s| systems.get_system(*s).system_name())
                .collect::<Vec<&str>>()
                .join("; ");
            println!("batch: {i} = {batch_str}");
        }
    }

    fn print_system_id_paths(systems: &Systems, system_id_paths: &[Vec<SystemId>]) {
        println!("Loops in system constraints were detected:");
        for path in system_id_paths {
            let path_str: String = path
                .iter()
                .map(|s| systems.get_system(*s).system_name())
                .collect::<Vec<&str>>()
                .join(" -> ");
            println!("{path_str}");
        }
    }

    fn find_loops_for_all_system_constraints(
        &self,
        systems: &Systems,
    ) -> Result<(), Vec<Vec<SystemId>>> {
        let mut result = Ok(());
        for s in 0..systems.system_vec.len() {
            let mut sys_id_path = Vec::new();
            if self.find_loops_in_system_constraints_rec(systems, &mut sys_id_path, s.into()) {
                match result {
                    Ok(()) => result = Err(vec![sys_id_path]),
                    Err(ref mut sys_id_paths) => sys_id_paths.push(sys_id_path),
                }
            }
        }
        result
    }
    fn find_loops_in_system_constraints_rec(
        &self,
        systems: &Systems,
        system_id_path: &mut Vec<SystemId>,
        cur_system_id: SystemId,
    ) -> bool {
        system_id_path.push(cur_system_id);
        if let Some(constraint) = systems.get_constraint(&cur_system_id) {
            for next_system_id in constraint.before.iter() {
                if system_id_path.contains(next_system_id) {
                    system_id_path.push(*next_system_id);
                    return true;
                } else if self.find_loops_in_system_constraints_rec(
                    systems,
                    system_id_path,
                    *next_system_id,
                ) {
                    return true;
                }
                system_id_path.pop();
            }
        }
        false
    }

    fn build_constraint_based_schedule(systems: &Systems) -> Vec<HashSet<SystemId>> {
        let mut schd: Vec<HashSet<SystemId>> = Vec::new();

        //find constraint roots, end, unconstraint systems
        let mut roots: HashSet<SystemId> = HashSet::new();
        let mut unconstrained: HashSet<SystemId> = HashSet::new();
        //TODO: some unconstraint systems do not seem to have a constraints struct at all
        // registered
        // TODO: add unconstraint systems into schedule
        for (s, c) in &systems.constraints {
            if c.after.is_empty() && !c.before.is_empty() {
                roots.insert(*s);
            } else if c.after.is_empty() && c.before.is_empty() {
                unconstrained.insert(*s);
            }
        }

        schd.push(roots);
        let mut next_batch: HashSet<SystemId> = HashSet::new();
        for s in schd[0].iter() {
            if let Some(constraint) = systems.get_constraint(s) {
                next_batch.extend(constraint.before.iter());
            }
        }

        schd.push(next_batch);

        let mut batch_index = 1;
        loop {
            Self::print_schedule(systems, &schd);
            let mut next_batch: HashSet<SystemId> = HashSet::new();
            for s1 in schd[batch_index].iter() {
                if let Some(s1_c) = systems.get_constraint(s1) {
                    for s2 in schd[batch_index].iter() {
                        if s1 != s2 {
                            if s1_c.before.contains(s2) {
                                next_batch.insert(*s2);
                            }
                        }
                    }
                }
            }

            for s in schd[batch_index].iter() {
                if let Some(constraint) = systems.get_constraint(s) {
                    next_batch.extend(constraint.before.iter());
                }
            }

            // remove already existing system ids in the upper batches
            for batch in schd.iter_mut() {
                for s in next_batch.iter() {
                    batch.remove(s);
                }
            }

            if next_batch.is_empty() {
                break;
            }

            schd.push(next_batch);

            batch_index += 1;
        }

        // insert unconstrained systems into smallest batches possible
        for s in unconstrained.iter() {
            if let Some(smallest_batch) = schd.iter_mut().min_by(|x, y| x.len().cmp(&y.len())) {
                smallest_batch.insert(*s);
            }
        }

        schd
    }
}

impl Scheduler for ParallelScheduler {
    fn init_schedule(&mut self, systems: &Systems) {
        //TODO: schedule systems in parallel according to their mutable and immutable Systemparams

        if let Err(paths) = self.find_loops_for_all_system_constraints(systems) {
            Self::print_system_id_paths(systems, &paths);
            panic!("System scheduling loop detected!")
        }

        let schd: Vec<HashSet<SystemId>> = Self::build_constraint_based_schedule(systems);
        Self::print_schedule(systems, &schd);
        self.schedule = schd;
    }
    fn execute(&mut self, systems: &mut Systems, world_data: &mut UnsafeCell<WorldData>) {
        fn run_sys<'a>(
            system: &mut dyn System,
            sys_par_data: &Vec<SystemParamId>,
            world_data: &SharedWorldData<'a>,
        ) {
            system.run(sys_par_data, world_data.0.get());
        }

        let sys_par_data = &systems.system_param_data;

        //TODO: create thread pool that can act like scopes
        //TODO: need way to receive signal,
        // when all jobs are finished, to reuse threads in pool
        for (i, batch) in self.schedule.iter().enumerate() {
            println!("exec batch {}:", i);
            {
                let world_data = SharedWorldData(&*world_data);
                let world_data = &world_data;
                std::thread::scope(|s| {
                    for (i, sys) in systems
                        .system_vec
                        .iter_mut()
                        .enumerate()
                        .filter(|(i, _s)| batch.contains(&SystemId::from(i)))
                    {
                        println!("exec system: name {}; id {}:", sys.system_name(), i);
                        let sys_id = SystemId::from(i);
                        s.spawn(move || {
                            run_sys(
                                sys.as_mut(),
                                &sys_par_data.get(&sys_id).unwrap(),
                                &world_data,
                            );
                        });
                    }
                });
            }
            // execute commands after all systems of one batch have run
            world_data.get_mut().execute_commands();
        }
    }
}
