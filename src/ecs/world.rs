// world.rs

use std::{any::TypeId, cell::UnsafeCell, collections::HashMap};

use crate::{
    ecs::{
        entity::EntityKey,
        query::QueryStateKey,
        resource::ResourceId,
        system::{IntoSystem, System, SystemId},
    },
    utils::{any_map::AnyMap, tuple_types::TupleTypesExt},
};

use super::{
    commands::CommandQueuesStorage, query::QueryState, storages::entity_storage::EntityStorage,
    system::Systems,
};

pub struct World {
    pub data: UnsafeCell<WorldData>,
    pub systems: Systems,
}

pub struct WorldData {
    pub(crate) resources: AnyMap,
    pub entity_storage: EntityStorage,
    pub(crate) query_data: HashMap<QueryStateKey, QueryState>,
    pub(crate) query_data2: Vec<QueryState>,
    pub(crate) commands_queues: CommandQueuesStorage,
}

impl World {
    pub fn new() -> Self {
        World {
            data: WorldData::new().into(),
            systems: Systems::new(),
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
        value: impl IntoSystem<Input, System = S>,
    ) -> SystemId {
        self.systems.add_system(value, &self.data)
    }

    pub fn run(&mut self) {
        self.data.get_mut().entity_storage.entities.reset_barriers();
        self.systems.run_systems(&mut self.data);
        self.data.get_mut().execute_commands();
        self.data
            .get_mut()
            .entity_storage
            .entities
            .update_with_barriers();
    }
}

impl WorldData {
    pub fn new() -> Self {
        WorldData {
            resources: AnyMap::new(),
            entity_storage: EntityStorage::new(),
            query_data: HashMap::new(),
            query_data2: Vec::new(),
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
