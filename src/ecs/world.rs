// world.rs

use std::{any::TypeId, cell::UnsafeCell};

use crate::{
    ecs::{
        entity::EntityKey,
        resource::ResourceId,
        system::{IntoSystem, System, SystemId},
    },
    utils::{any_map::AnyMap, tuple_types::TupleTypesExt},
};

use super::{
    commands::CommandQueuesStorage,
    query::QueryState,
    scheduler::{Scheduler, SingleThreadScheduler},
    storages::entity_storage::EntityStorage,
    system::{
        Systems,
        builder::{IntoSystemConfig, IntoSystemTuple},
    },
};

pub struct World {
    pub data: UnsafeCell<WorldData>,
    pub systems: Systems,
    pub(crate) scheduler: SingleThreadScheduler,
}

pub struct WorldData {
    pub(crate) resources: AnyMap,
    pub entity_storage: EntityStorage,
    pub(crate) query_data: Vec<QueryState>,
    pub(crate) commands_queues: CommandQueuesStorage,
}

impl World {
    pub fn new() -> Self {
        World {
            data: WorldData::new().into(),
            systems: Systems::new(),
            scheduler: SingleThreadScheduler::new(),
        }
    }

    pub fn add_resource<T: 'static>(&mut self, value: T) -> ResourceId {
        self.data.get_mut().add_resource(value)
    }

    pub fn add_entity<T: TupleTypesExt>(&mut self, input: T) -> EntityKey {
        self.data.get_mut().entity_storage.add_entity(input)
    }

    pub fn add_system<Input, S: System + 'static>(
        &mut self,
        value: impl IntoSystem<Input, System = S> + 'static,
    ) -> SystemId {
        self.systems.add_system(value)
    }

    pub fn add_system_builder<
        I,
        ST: IntoSystemTuple<I>,
        IA,
        AS: IntoSystemTuple<IA>,
        IB,
        BS: IntoSystemTuple<IB>,
    >(
        &mut self,
        value: impl IntoSystemConfig<I, ST, IA, AS, IB, BS>,
    ) -> Vec<SystemId> {
        self.systems.add_system_builder(value)
    }

    pub fn init_systems(&mut self) {
        self.systems.init_systems(&mut self.data);
        (0..self.systems.system_vec.len()).for_each(|n| self.scheduler.schedule.push(n.into()));
    }

    pub fn run(&mut self) {
        self.data.get_mut().entity_storage.entities.reset_barriers();
        self.scheduler.execute(&mut self.systems, &mut self.data);
        //TODO: at what point should commands be executed?: self.data.get_mut().execute_commands();
        self.data
            .get_mut()
            .entity_storage
            .entities
            .update_with_barriers();
    }

    pub fn init_and_run(&mut self) {
        self.init_systems();
        self.run();
    }

    pub fn run_loop(&mut self) {
        self.init_systems();
        loop {
            self.run();
        }
    }
}

impl WorldData {
    pub fn new() -> Self {
        WorldData {
            resources: AnyMap::new(),
            entity_storage: EntityStorage::new(),
            query_data: Vec::new(),
            commands_queues: CommandQueuesStorage::new(),
        }
    }

    pub fn add_resource<T: 'static>(&mut self, value: T) -> ResourceId {
        self.resources.insert(value);
        let resource_id = ResourceId::new(TypeId::of::<T>());
        self.entity_storage
            .depend_graph
            .insert_resource(resource_id);
        resource_id
    }

    pub(crate) fn execute_commands(&mut self) {
        while let Some(mut cq) = self.commands_queues.command_queues_inuse.pop() {
            while let Some(command) = cq.get_mut().pop() {
                command.exec(self);
            }
            self.commands_queues.command_queues_unused.push(cq);
        }
    }
}
