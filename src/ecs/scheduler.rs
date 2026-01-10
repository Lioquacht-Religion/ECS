// scheduler.rs

use std::{cell::UnsafeCell, collections::HashSet};

use crate::{
    ecs::{
        system::{System, SystemId, SystemParamId, Systems},
        world::{SharedWorldData, WorldData},
    },
    utils::{ecs_id::EcsId, threadpool::ThreadPool},
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
    schedule: Vec<Vec<SystemId>>,
    thread_pool: ThreadPool,
}

impl ParallelScheduler {
    pub(crate) fn new(thread_count: usize) -> Self {
        Self {
            schedule: Vec::new(),
            thread_pool: ThreadPool::new(thread_count),
        }
    }

    fn print_system_id_paths(systems: &Systems, system_id_paths: &[Vec<SystemId>]){
        println!("Loops in system constraints were detected:");
        for path in system_id_paths{
            let path_str : String = path
                .iter().map(
                    |s| systems.get_system(*s).system_name())
                .collect::<Vec<&str>>()
                .join(" -> ");
            println!("{path_str}");
        }
    }

    fn find_loops_for_all_system_constraints(&self, systems: &Systems) -> Result<(), Vec<Vec<SystemId>>>{
        let mut result = Ok(());
        for s in 0..systems.system_vec.len(){
            let mut sys_id_path = Vec::new();
            if self.find_loops_in_system_constraints_rec(
                systems, &mut sys_id_path, s.into()
            ){
                match result {
                    Ok(()) 
                        => result = Err(vec![sys_id_path]),
                    Err(ref mut sys_id_paths) 
                        => sys_id_paths.push(sys_id_path),
                }
            }
        }
        result
    }
    fn find_loops_in_system_constraints_rec(
        &self, systems: &Systems, system_id_path: &mut Vec<SystemId>, cur_system_id: SystemId
    ) -> bool{
        system_id_path.push(cur_system_id);
        for next_system_id in systems.get_constraint(&cur_system_id).before.iter(){
            if system_id_path.contains(next_system_id) {
                system_id_path.push(*next_system_id);
                return true;
            }
            else if self.find_loops_in_system_constraints_rec(
                systems, system_id_path, *next_system_id
            ){
                return true;
            }
            system_id_path.pop();
        }
        false
    }
}

impl Scheduler for ParallelScheduler {
    fn init_schedule(&mut self, systems: &Systems) {
        //TODO:

        if let Err(paths) = self.find_loops_for_all_system_constraints(systems){
            Self::print_system_id_paths(systems, &paths);
            panic!("Detected loops in system constraints.")
        }

        let schd : Vec<HashSet<SystemId>> = Vec::new();

        fn rec(systems: &Systems, system_id: SystemId){
            let constraint = systems.get_constraint(&system_id);


        }

        for i in 0..systems.system_vec.len(){
            let system_id : SystemId = i.into();
            let constraint = systems.get_constraint(&system_id);


        }
    }
    fn execute(&mut self, systems: &mut Systems, world_data: &mut UnsafeCell<WorldData>) {

        fn run_sys<'a>(
            system: &mut dyn System,
            sys_par_data: &Vec<SystemParamId>,
            world_data: &SharedWorldData<'a>,
        ) {
            system.run(sys_par_data, world_data.0.get());
        }

        {
        let sys_par_data = &systems.system_param_data;
        let world_data = SharedWorldData(&*world_data);
        let world_data = &world_data;

        //TODO: need way to receive signal, 
        // when all jobs are finished, to reuse threads in pool
        //TODO: create thread pool that can act like scopes
        std::thread::scope(|s| {
            for (i, sys) in systems.system_vec.iter_mut().enumerate() {
                let sys_id = SystemId::from(i);
                s.spawn(move || {
                    run_sys(
                        sys.as_mut(), 
                        &sys_par_data.get(&sys_id).unwrap(), 
                        &world_data
                    );
                });
            }
        });
        }
        // execute commands after all systems have run
        world_data.get_mut().execute_commands();

        //TODO:
        /*
        self.thread_pool.execute(|| {
            unsafe{
                //(&mut *systems).run_system2(self.schedule[0][0], world_data);
                system.run(sys_par_data, world_data.0.get());
            }
        });
        */
    }
}
