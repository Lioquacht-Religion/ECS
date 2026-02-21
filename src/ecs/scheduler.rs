// scheduler.rs

use std::{
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
};

use crate::{
    ecs::{
        ecs_dependency_graph::{EcsDependencyGraph, EcsEdge},
        system::{System, SystemId, SystemParamId, Systems},
        world::{SharedWorldData, WorldData},
    },
    utils::threadpool::ThreadPool,
};

pub(crate) trait Scheduler {
    fn init_schedule(&mut self, graph: &mut EcsDependencyGraph, systems: &Systems);
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
    fn init_schedule(&mut self, _graph: &mut EcsDependencyGraph, systems: &Systems) {
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
    schedule: Vec<Vec<HashSet<SystemId>>>,
    //thread_pool: ThreadPool,
}

impl ParallelScheduler {
    pub(crate) fn new(thread_count: usize) -> Self {
        Self {
            schedule: Vec::new(),
            //thread_pool: ThreadPool::new(thread_count),
        }
    }

    fn print_schedule(systems: &Systems, schedule: &[Vec<HashSet<SystemId>>]) {
        println!("System constraint group schedule:");
        for (i, batch) in schedule.iter().enumerate() {
            println!("constraint batch: {i}");
            Self::print_parallel_batch_schedule(systems, batch);
        }
    }

    fn print_parallel_batch_schedule(systems: &Systems, schedule: &[HashSet<SystemId>]) {
        println!("System parallel group schedule:");
        for (i, batch) in schedule.iter().enumerate() {
            let batch_str: String = batch
                .iter()
                .map(|s| systems.get_system(*s).system_name())
                .collect::<Vec<&str>>()
                .join("; ");
            println!("parallel batch: {i} = {batch_str}");
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

    fn build_parallel_based_schedule(
        graph: &mut EcsDependencyGraph,
        systems: &Systems,
    ) -> Vec<Vec<HashSet<SystemId>>> {
        let parallel_schedule = Self::build_constraint_based_schedule(systems)
            .into_iter()
            .map(|batch| Self::create_parallizable_systems_schedule(graph, batch))
            .collect();
        parallel_schedule
    }

    fn create_parallizable_systems_schedule(
        graph: &mut EcsDependencyGraph,
        mut systems_to_check: HashSet<SystemId>,
    ) -> Vec<HashSet<SystemId>> {
        let conflicting_systems_map: HashMap<SystemId, HashSet<SystemId>> = systems_to_check
            .iter()
            .map(|sys_id| (*sys_id, Self::find_conflicting_systems(graph, *sys_id)))
            .collect();
        let mut schedule = Vec::new();

        while !systems_to_check.is_empty() {
            let mut parallel_systems: HashSet<SystemId> = HashSet::new();
            let system_id = *systems_to_check.iter().next().unwrap();
            systems_to_check.remove(&system_id);
            parallel_systems.insert(system_id);
            let conflict_systems = conflicting_systems_map.get(&system_id).unwrap();
            for compatible_sys_id in systems_to_check.difference(&conflict_systems) {
                let conflict_systems = conflicting_systems_map.get(&compatible_sys_id).unwrap();
                if parallel_systems
                    .intersection(conflict_systems)
                    .next()
                    .is_none()
                {
                    parallel_systems.insert(*compatible_sys_id);
                }
            }
            parallel_systems.iter().for_each(|sys_id| {
                systems_to_check.remove(sys_id);
            });
            schedule.push(parallel_systems);
        }
        schedule
    }

    fn find_conflicting_systems(
        graph: &mut EcsDependencyGraph,
        system_id: SystemId,
    ) -> HashSet<SystemId> {
        //TODO:
        let system_row_id = graph.insert_system(system_id) as usize;
        let system_node = &graph.systems[system_row_id];
        let sys_res: &HashMap<u32, EcsEdge> = &system_node.resource_edges;
        let (res_excl, res_shared) = Self::create_excl_shared_sets(sys_res);
        let sys_comps: &HashMap<u32, EcsEdge> = &system_node.component_edges;
        let (comp_excl, comp_shared) = Self::create_excl_shared_sets(sys_comps);

        dbg!(&system_node.resource_edges);
        dbg!(&res_excl);
        dbg!(&res_shared);

        // Finding conflicting systems:
        // - find excl and shared res/comps sets for every system
        // - per system find all to res or comps connected systems
        // - check if other systems access same res/comps mutably
        // or one mut and the other immutable
        // - create set of conflicting systems for one system

        let mut conn_sys_ids: HashSet<SystemId> = HashSet::new();
        // find systems connected through component
        for (comp_id, _edge) in sys_comps.iter() {
            for (sys_id, _edge) in graph.components[*comp_id as usize].system_edges.iter() {
                conn_sys_ids.insert(sys_id.into());
            }
        }
        // find systems connected through resources
        for (res_id, _edge) in sys_res.iter() {
            for (sys_id, _edge) in graph.resources[*res_id as usize].system_edges.iter() {
                conn_sys_ids.insert(sys_id.into());
            }
        }

        let mut conflict_systems: HashSet<SystemId> = HashSet::new();
        'systems_loop: for sys_id in conn_sys_ids.iter() {
            let system_row_id = graph.insert_system(*sys_id) as usize;
            let system_node = &graph.systems[system_row_id];
            let sys_res: &HashMap<u32, EcsEdge> = &system_node.resource_edges;
            let (res_excl2, res_shared2) = Self::create_excl_shared_sets(sys_res);
            let sys_comps: &HashMap<u32, EcsEdge> = &system_node.component_edges;
            let (comp_excl2, comp_shared2) = Self::create_excl_shared_sets(sys_comps);

            for comp_id in comp_excl.iter() {
                if comp_excl2.contains(comp_id) {
                    conflict_systems.insert(*sys_id);
                    continue 'systems_loop;
                } else if comp_shared2.contains(comp_id) {
                    conflict_systems.insert(*sys_id);
                    continue 'systems_loop;
                }
            }
            for comp_id in comp_shared.iter() {
                if comp_excl2.contains(comp_id) {
                    conflict_systems.insert(*sys_id);
                    continue 'systems_loop;
                }
            }

            for res_id in res_excl.iter() {
                if res_excl2.contains(res_id) {
                    conflict_systems.insert(*sys_id);
                    continue 'systems_loop;
                } else if res_shared2.contains(res_id) {
                    conflict_systems.insert(*sys_id);
                    continue 'systems_loop;
                }
            }
            for res_id in res_shared.iter() {
                if res_excl2.contains(res_id) {
                    conflict_systems.insert(*sys_id);
                    continue 'systems_loop;
                }
            }
        }
        dbg!(&conflict_systems);
        return conflict_systems;
    }

    fn create_excl_shared_sets(map: &HashMap<u32, EcsEdge>) -> (HashSet<u32>, HashSet<u32>) {
        let mut excl = HashSet::new();
        let mut shared = HashSet::new();
        for (k, v) in map.iter() {
            match v {
                EcsEdge::Owned | EcsEdge::Excl => excl.insert(*k),
                EcsEdge::Shared => shared.insert(*k),
                EcsEdge::None => false,
            };
        }
        (excl, shared)
    }
}

impl Scheduler for ParallelScheduler {
    fn init_schedule(&mut self, graph: &mut EcsDependencyGraph, systems: &Systems) {
        if let Err(paths) = self.find_loops_for_all_system_constraints(systems) {
            Self::print_system_id_paths(systems, &paths);
            panic!("System scheduling loop detected!")
        }

        // schedule systems in parallel according to their mutable and immutable Systemparams
        let schd = Self::build_parallel_based_schedule(graph, systems);
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
            for batch in batch.iter() {
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
            }
            // execute commands after all systems of one batch have run
            world_data.get_mut().execute_commands();
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ecs::{
        scheduler::ParallelScheduler,
        system::{Res, ResMut},
        world::World,
    };

    struct Resource1(usize);
    struct Resource2(usize);
    struct Resource3(usize);
    fn test_system_shared_access1(res1: Res<Resource1>, res2: Res<Resource2>) {
        println!("sys 1: res1: {}, res2: {}", res1.0, res2.0);
    }
    fn test_system_shared_access2(res1: Res<Resource1>, res2: Res<Resource2>) {
        println!("sys 2: res1: {}, res2: {}", res1.0, res2.0);
    }
    fn test_system_excl_shared_access3(mut res1: ResMut<Resource1>, res2: Res<Resource2>) {
        res1.0 += 1;
        println!("sys 3: resmut1: {}, res2: {}", res1.0, res2.0);
    }
    fn test_system_shared_excl_access4(res1: Res<Resource1>, mut res2: ResMut<Resource2>) {
        res2.0 += 1;
        println!("sys 4: res1: {}, resmut2: {}", res1.0, res2.0);
    }
    fn test_system_excl_shared_access5(mut res3: ResMut<Resource3>, res2: Res<Resource2>) {
        res3.0 += 1;
        println!("sys 4: res3: {}, resmut2: {}", res3.0, res2.0);
    }

    #[test]
    fn test_build_parallel_schedule() {
        let mut world = World::new();

        world.add_resource(Resource1(0));
        world.add_resource(Resource2(0));
        world.add_resource(Resource3(0));

        let sysid1 = world.add_systems(test_system_shared_access1).pop().unwrap();
        let sysid2 = world.add_systems(test_system_shared_access2).pop().unwrap();
        let sysid3 = world
            .add_systems(test_system_excl_shared_access3)
            .pop()
            .unwrap();
        let sysid4 = world
            .add_systems(test_system_shared_excl_access4)
            .pop()
            .unwrap();
        let sysid5 = world
            .add_systems(test_system_excl_shared_access5)
            .pop()
            .unwrap();

        world.init_systems();
        world.run();
        ParallelScheduler::print_schedule(&world.systems, &world.scheduler.schedule);

        let scheduler = &world.scheduler.schedule;

        let set_with_sysid1 = scheduler
            .iter()
            .find_map(|set| set.iter().find(|set| set.contains(&sysid1)))
            .unwrap();

        let set_with_sysid2 = scheduler
            .iter()
            .find_map(|set| set.iter().find(|set| set.contains(&sysid2)))
            .unwrap();

        let set_with_sysid5 = scheduler
            .iter()
            .find_map(|set| set.iter().find(|set| set.contains(&sysid5)))
            .unwrap();

        assert!(!set_with_sysid1.contains(&sysid3));
        assert!(!set_with_sysid1.contains(&sysid4));

        assert!(!set_with_sysid2.contains(&sysid3));
        assert!(!set_with_sysid2.contains(&sysid4));

        assert!(!set_with_sysid5.contains(&sysid3));
        assert!(!set_with_sysid5.contains(&sysid4));
        assert!(false);
    }
}
