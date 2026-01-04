// world.rs

use std::{any::TypeId, cell::UnsafeCell, collections::HashMap};

use crate::{
    ecs::{
        component::{Archetype, ArchetypeId, ComponentId}, ecs_dependency_graph::EcsDependencyGraph, entity::{Entities, EntityKey}, prelude::Component, query::QueryParam, resource::ResourceId, storages::{cache::EntityStorageCache, table_storage::TableStorage}, system::{IntoSystem, System, SystemId}
    },
    utils::{any_map::AnyMap, sorted_vec::SortedVec, tuple_iters::TupleIterator, tuple_types::TupleTypesExt},
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
    entity_storage: EntityStorage,
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
        self.data.get_mut().add_entity(input)
    }

    pub fn add_entities_batch<T: TupleTypesExt>(&mut self, input: Vec<T>) -> Vec<EntityKey> {
        self.data.get_mut().entity_storage.add_entities_batch(input)
    }

    pub fn get_entity_components<P: QueryParam>(
        &mut self,
        entity_key: EntityKey,
    ) -> Option<<P::Construct<'_> as TupleIterator>::Item> {
        self.data
            .get_mut()
            .entity_storage
            .get_entity_components::<P>(entity_key)
    }

    pub fn get_single_component<T: Component>(&mut self, entity_key: EntityKey) -> Option<&T> {
        self.data
            .get_mut()
            .entity_storage.get_single_component(entity_key)
    }

    pub fn get_single_component_mut<T: Component>(
        &mut self,
        entity_key: EntityKey,
    ) -> Option<&mut T> {
        self.data
            .get_mut()
            .entity_storage.get_single_component_mut(entity_key)
    }

    pub fn add_system<Input, S: System + 'static>(
        &mut self,
        value: impl IntoSystem<Input, System = S> + 'static,
    ) -> SystemId {
        self.systems.add_system(value)
    }

    pub fn add_systems<
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

    pub fn add_entity<T: TupleTypesExt>(&mut self, input: T) -> EntityKey {
        let key = self.entity_storage.add_entity(input);
        T::exec_on_add_rec(self, key);
        key
    }

    pub(crate) fn add_entity_with_reserved_key<T: TupleTypesExt>(
        &mut self,
        key: EntityKey,
        input: T,
    ) -> EntityKey {
        let key = self.entity_storage.add_entity_with_reserved_key(key, input);
        T::exec_on_add_rec(self, key);
        key
    }

    pub fn remove_entity(&mut self, entity_key: EntityKey) {
        //TODO: add remove hooks
        //T::exec_on_remove_rec(self, key);
        self.entity_storage.remove_entity(entity_key);
    }

    pub(crate) fn create_or_get_component<T: Component>(&mut self) -> ComponentId {
        self.entity_storage.create_or_get_component::<T>()
    }

    pub(crate) fn find_fitting_archetypes(
        &self,
        query_comp_ids: &SortedVec<ComponentId>,
    ) -> Vec<ArchetypeId> {
        self.entity_storage.find_fitting_archetypes(query_comp_ids)
    }

    pub fn add_resource<T: 'static>(&mut self, value: T) -> ResourceId {
        self.resources.insert(value);
        let resource_id = ResourceId::new(TypeId::of::<T>());
        self.entity_storage
            .depend_graph
            .insert_resource(resource_id);
        resource_id
    }

    pub fn get_entity_components<P: QueryParam>(
        &mut self,
        entity_key: EntityKey,
    ) -> Option<<P::Construct<'_> as TupleIterator>::Item> {
        self.entity_storage
            .get_entity_components::<P>(entity_key)
    }

    pub fn get_single_component<T: Component>(&mut self, entity_key: EntityKey) -> Option<&T> {
        self.entity_storage.get_single_component(entity_key)
    }

    pub fn get_single_component_mut<T: Component>(
        &mut self,
        entity_key: EntityKey,
    ) -> Option<&mut T> {
        self.entity_storage.get_single_component_mut(entity_key)
    }

    #[allow(unused)]
    pub(crate) fn get_depend_graph(&self) -> &EcsDependencyGraph{
        &self.entity_storage.depend_graph
    }

    pub(crate) fn get_depend_graph_mut(&mut self) -> &mut EcsDependencyGraph{
        &mut self.entity_storage.depend_graph
    }

    #[allow(unused)]
    pub(crate) fn get_tables(&self) -> &HashMap<ArchetypeId, TableStorage>{
        &self.entity_storage.tables
    }

    pub(crate) fn get_tables_mut(&mut self) -> &mut HashMap<ArchetypeId, TableStorage>{
        &mut self.entity_storage.tables
    }

    pub(crate) fn get_entities(&self) -> &Entities{
        &self.entity_storage.entities
    }

    #[allow(unused)]
    pub(crate) fn get_entities_mut(&mut self) -> &mut Entities{
        &mut self.entity_storage.entities
    }

    #[allow(unused)]
    pub(crate) fn get_cache(&self) -> &EntityStorageCache{
        &self.entity_storage.cache
    }

    pub(crate) fn get_cache_mut(&mut self) -> &mut EntityStorageCache{
        &mut self.entity_storage.cache
    }

    pub(crate) fn get_archetypes(&self) -> &Vec<Archetype>{
        &self.entity_storage.archetypes
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
