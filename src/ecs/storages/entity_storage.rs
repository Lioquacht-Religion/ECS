// entity_storage.rs

use std::{any::TypeId, collections::HashMap, hash::Hash};

use crate::{
    ecs::{
        component::{Archetype, ArchetypeId, Component, ComponentId, ComponentInfo, Map}, ecs_dependency_graph::EcsDependencyGraph, entity::{Entities, Entity, EntityKey, TableRowId}, prelude::StorageTypes, query::{QueryParam, QueryParamMetaData}, storages::table_soa::TableSoA
    },
    utils::{ecs_id::EcsId, sorted_vec::SortedVec, tuple_iters::TupleIterator, tuple_types::TupleTypesExt},
};

use super::{cache::EntityStorageCache, table_storage::TableStorage};

pub struct EntityStorage {
    pub(crate) entities: Entities,
    pub(crate) components: Vec<ComponentInfo>,
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) tables: Map<ArchetypeId, TableStorage>,
    //mapping data
    pub(crate) typeid_compid_map: Map<TypeId, ComponentId>,
    pub(crate) compids_archid_map: Map<SortedVec<ComponentId>, ArchetypeId>,
    pub(crate) depend_graph: EcsDependencyGraph,
    pub(crate) cache: EntityStorageCache,
}

impl EntityStorage {
    pub(crate) fn new() -> Self {
        Self {
            entities: Entities::new(),
            components: Vec::new(),
            archetypes: Vec::new(),
            tables: Map::new(),
            typeid_compid_map: Map::new(),
            compids_archid_map: Map::new(),
            depend_graph: EcsDependencyGraph::new(),
            cache: EntityStorageCache::new(),
        }
    }

    pub(crate) fn find_fitting_archetypes(
        &self,
        query_comp_ids: &SortedVec<QueryParamMetaData>,
    ) -> Vec<ArchetypeId> {
        self.compids_archid_map
            .iter()
            .filter_map(|(arch_cids, arch_id)| {
                if Self::is_subset_of(query_comp_ids, arch_cids) {
                    Some(*arch_id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn is_subset_of(possible_subset: &SortedVec<QueryParamMetaData>, wholeset: &SortedVec<ComponentId>) -> bool {
        let not_optional_count = possible_subset.iter()
            .filter(|qpmd| !qpmd.optional)
            .count();
        let subset_iter = possible_subset.iter();
        let mut contains_count = 0;

        if not_optional_count > wholeset.get_vec().len() {
            return false;
        }

        for el in subset_iter.filter(|qpmd| !qpmd.optional) 
        {
            for el2 in wholeset.iter() {
                if el.comp_id == *el2 {
                    contains_count += 1;
                    continue;
                }
            }
        }

        contains_count == not_optional_count
    }

    pub(crate) fn add_entity<T: TupleTypesExt>(&mut self, input: T) -> EntityKey {
        let archetype_id = self.create_or_get_archetype::<T>();
        let key = self.entities.insert(Entity {
            archetype_id,
            row_id: TableRowId(0),
        });

        self.add_entity_inner(key, input, archetype_id)
    }

    pub(crate) fn get_entity_components<P: QueryParam>(
        &mut self,
        entity_key: EntityKey,
    ) -> Option<<P::Construct<'_> as TupleIterator>::Item> {
        if let Some(entity) = self.entities.get(entity_key) {
            if let Some(table) = self.tables.get_mut(&entity.archetype_id) {
                return table.get_entity_components::<P>(*entity);
            }
        }
        None
    }

    pub(crate) fn get_single_component<T: Component>(
        &mut self,
        entity_key: EntityKey,
    ) -> Option<&T> {
        self.get_entity_components::<&T>(entity_key)
    }

    pub(crate) fn get_single_component_mut<T: Component>(
        &mut self,
        entity_key: EntityKey,
    ) -> Option<&mut T> {
        self.get_entity_components::<&mut T>(entity_key)
    }

    pub(crate) fn add_entity_with_reserved_key<T: TupleTypesExt>(
        &mut self,
        key: EntityKey,
        input: T,
    ) -> EntityKey {
        let archetype_id = self.create_or_get_archetype::<T>();
        self.entities.insert_with_reserved_key(
            key,
            Entity {
                archetype_id,
                row_id: TableRowId(0),
            },
        );

        self.add_entity_inner(key, input, archetype_id)
    }

    pub(crate) fn add_entity_inner<T: TupleTypesExt>(
        &mut self,
        key: EntityKey,
        input: T,
        archetype_id: ArchetypeId,
    ) -> EntityKey {
        let mut soa_comp_ids = self.cache.compid_vec_cache.take_cached();
        let mut aos_comp_ids = self.cache.compid_vec_cache.take_cached();

        T::get_comp_ids_by_storage(self, &mut soa_comp_ids, &mut aos_comp_ids);

        let row_id = unsafe {
            self.tables
                .get_mut(&archetype_id)
                .expect("ERROR: table does not contain archetype id!")
                .insert(
                    EntityKey::new(key.get_id(), key.get_generation()),
                    &self.components,
                    &soa_comp_ids,
                    &aos_comp_ids,
                    &mut self.cache,
                    input,
                )
        };

        self.cache.compid_vec_cache.insert(soa_comp_ids);
        self.cache.compid_vec_cache.insert(aos_comp_ids);

        if let Some(e) = self.entities.get_mut(key) {
            e.row_id = row_id;
        }
        key
    }

    pub(crate) fn add_entities_batch<T: TupleTypesExt>(&mut self, input: Vec<T>) -> Vec<EntityKey> {
        let archetype_id = self.create_or_get_archetype::<T>();

        let mut soa_comp_ids = self.cache.compid_vec_cache.take_cached();
        let mut aos_comp_ids = self.cache.compid_vec_cache.take_cached();
        T::get_comp_ids_by_storage(self, &mut soa_comp_ids, &mut aos_comp_ids);

        let table = self
            .tables
            .get_mut(&archetype_id)
            .expect("ERROR: table does not contain archetype id!");

        let row_id_start = table.len as usize;
        let row_id_end = row_id_start + input.len();
        let mut entity_keys = Vec::with_capacity(input.len());
        for i in row_id_start..row_id_end {
            let key = self.entities.insert(Entity {
                archetype_id,
                row_id: i.into(),
            });
            entity_keys.push(key);
        }

        unsafe {
            table.insert_batch(
                &entity_keys,
                &self.components,
                &soa_comp_ids,
                &aos_comp_ids,
                &mut self.cache,
                input,
            );
        }

        self.cache.compid_vec_cache.insert(soa_comp_ids);
        self.cache.compid_vec_cache.insert(aos_comp_ids);

        entity_keys
    }

    pub(crate) fn remove_entity(&mut self, entity_key: EntityKey) {
        if let Some(entity) = self.entities.remove(entity_key) {
            if let Some(table) = self.tables.get_mut(&entity.archetype_id) {
                if let Some((key, row_id)) = table.remove_entity(entity) {
                    if let Some(entity) = self.entities.get_mut(key) {
                        entity.row_id = row_id;
                    }
                }
            }
        }
    }

    pub(crate) fn add_component_to_entity<T: Component>(&mut self, entity_key: EntityKey, component: T){
        if let Some(entity) = self.entities.get_mut(entity_key){
            let entity = entity.clone();
            //TODO: how to hanlde if entity already does not contain the to be removed component
            let to_table_arch_id = if let Ok(to_table_arch_id) = 
                self.create_or_get_archetype_adding_comp_to_entity::<T>(entity.archetype_id) {
                to_table_arch_id
            }
            else{
                //TODO should entity be overwritten here? should error be passed further?
                return;
            };
            let (arch_id, row_id) = if let Ok((table_from, table_to)) 
                = self.tables.split_mut2(&entity.archetype_id, &to_table_arch_id)
            {
                match T::STORAGE {
                    StorageTypes::TableAoS => todo!(),
                    StorageTypes::TableSoA => {
                        //TODO: need to transfer aos and soa simultanously
                        let row_id = TableSoA::transfer_entity_with_new_comp(
                            &mut table_from.table_soa, &mut table_to.table_soa, &entity, component
                        ); 
                        (to_table_arch_id, row_id)
                    },
                    StorageTypes::SparseSet => todo!(),
                }
            }
            else{
                panic!("Tables for both from and to archetypes should exist at this point.")
            };
            // update row id and archetype id, because entity moved tables
            let entity = self.entities.get_mut(entity_key).unwrap();
            entity.row_id = row_id;
            entity.archetype_id = arch_id;
        }
    }

    pub(crate) fn remove_component_from_entity<T: Component>(&mut self, entity_key: EntityKey){
        //TODO
        if let Some(entity) = self.entities.get_mut(entity_key){
            let entity = entity.clone();
            let to_table_arch_id = if let Ok(to_table_arch_id) = 
                self.create_or_get_archetype_removing_comp_from_entity::<T>(entity.archetype_id) {
                to_table_arch_id
            }
            else{
                //TODO should component be overwritten here? should error be passed further?
                return;
            };
            let (arch_id, row_id) = 
                match self.tables.split_mut2(&entity.archetype_id, &to_table_arch_id) {
                    Ok((table_from, table_to)) =>
            {
                match T::STORAGE {
                    StorageTypes::TableAoS => todo!(),
                    StorageTypes::TableSoA => {
                        //TODO: need to transfer aos and soa simultanously
                        let row_id = TableSoA::remove_comp_and_transfer_entity::<T>(
                            &mut table_from.table_soa, &mut table_to.table_soa, &entity 
                        ); 
                        (to_table_arch_id, row_id)
                    },
                    StorageTypes::SparseSet => todo!(),
                }
            }  
                    Err(e) => panic!("Tables for both from and to archetypes should exist at this point. error: {:?}", e),
            };
            // update row id and archetype id, because entity moved tables
            let entity = self.entities.get_mut(entity_key).unwrap();
            entity.row_id = row_id;
            entity.archetype_id = arch_id;
        }
    }

    pub(crate) fn create_or_get_archetype<T: TupleTypesExt>(&mut self) -> ArchetypeId {
        let mut comp_ids: Vec<ComponentId> = self.cache.compid_vec_cache.take_cached();
        T::create_or_get_component(self, &mut comp_ids);
        let comp_ids: SortedVec<ComponentId> = comp_ids.into();

        if let Some(archetype_id) = self.compids_archid_map.get(&comp_ids) {
            self.cache.compid_vec_cache.insert(comp_ids.into());
            return *archetype_id;
        }

        if let Some(_dup_compid) = comp_ids.check_duplicates() {
            //TODO: use dup_compid to get more error info
            panic!("INVALID: Same component contained multiple times inside of entity.");
        }

        let mut soa_comp_ids: Vec<ComponentId> = self.cache.compid_vec_cache.take_cached();
        let mut aos_comp_ids: Vec<ComponentId> = self.cache.compid_vec_cache.take_cached();
        T::get_comp_ids_by_storage(self, &mut soa_comp_ids, &mut aos_comp_ids);

        self.create_archetype_inner(comp_ids, soa_comp_ids, aos_comp_ids)
    }

    pub(crate) fn create_or_get_archetype_adding_comp_to_entity<T: Component>(
        &mut self, arch_id: ArchetypeId, 
    ) -> Result<ArchetypeId, ()> {
        let arch = &self.archetypes[arch_id.id_usize()];
        let mut comp_ids = self.cache.compid_vec_cache.take_cached();
        arch.aos_comp_ids.iter().chain(arch.soa_comp_ids.iter()).for_each(|cid| comp_ids.push(*cid));
        let added_compid = self.create_or_get_component::<T>();
        comp_ids.push(added_compid);
        let comp_ids = comp_ids.into();

        if let Some(archetype_id) = self.compids_archid_map.get(&comp_ids) {
            self.cache.compid_vec_cache.insert(comp_ids.into());
            return Ok(*archetype_id);
        }

        if let Some(_dup_compid) = comp_ids.check_duplicates() {
            println!("INVALID: Same component contained multiple times inside of entity.");
            return Err(());
        }

        let arch = &self.archetypes[arch_id.id_usize()];
        let mut soa_compids : Vec<ComponentId> = arch.soa_comp_ids.clone().into();
        let mut aos_compids : Vec<ComponentId> = arch.aos_comp_ids.clone().into();
        match T::STORAGE{
            StorageTypes::TableAoS => aos_compids.push(added_compid),
            StorageTypes::TableSoA => soa_compids.push(added_compid),
            StorageTypes::SparseSet => todo!(),
        }
        Ok(self.create_archetype_inner(comp_ids, soa_compids, aos_compids))
    }

    pub(crate) fn create_or_get_archetype_removing_comp_from_entity<T: Component>(
        &mut self, arch_id: ArchetypeId, 
    ) -> Result<ArchetypeId, ()> {
        // remove comp id from current entity comp ids to find preexisting fitting archetype
        let remove_compid = self.create_or_get_component::<T>();
        let arch = &self.archetypes[arch_id.id_usize()];
        let mut comp_ids = self.cache.compid_vec_cache.take_cached();
        arch.aos_comp_ids.iter().chain(arch.soa_comp_ids.iter())
            .filter(|cid| **cid != remove_compid)
            .for_each(|cid| comp_ids.push(*cid));
        let comp_ids = comp_ids.into();

        if let Some(archetype_id) = self.compids_archid_map.get(&comp_ids) {
            self.cache.compid_vec_cache.insert(comp_ids.into());
            return Ok(*archetype_id);
        }

        //TODO: when removing, handle case if all components of an entity have been removed
        // -> remove entity entirely
        if let Some(_dup_compid) = comp_ids.check_duplicates() {
            println!("INVALID: Same component contained multiple times inside of entity.");
            return Err(());
        }

        fn get_filtered(comp_ids: &SortedVec<ComponentId>, remove_compid: ComponentId) -> Vec<ComponentId> {
                comp_ids.get_vec()
                .iter().filter_map(|cid| if *cid != remove_compid {
                    Some(*cid)
                } else {
                    None
                }
                ).collect()
        }

        let arch = &self.archetypes[arch_id.id_usize()];
        let (soa_compids, aos_compids) : (Vec<ComponentId>, Vec<ComponentId>) = match T::STORAGE{
            StorageTypes::TableAoS => (
                arch.soa_comp_ids.clone().into(), 
                get_filtered(&arch.aos_comp_ids, remove_compid),
            ),
            StorageTypes::TableSoA => (
                get_filtered(&arch.soa_comp_ids, remove_compid),
                arch.aos_comp_ids.clone().into()
            ),
            StorageTypes::SparseSet => todo!(),
        };
        Ok(self.create_archetype_inner(comp_ids, soa_compids, aos_compids))
    }

    fn create_archetype_inner(&mut self, comp_ids: SortedVec<ComponentId>, soa_comp_ids: Vec<ComponentId>, aos_comp_ids: Vec<ComponentId>) -> ArchetypeId {
        let archetype_id = self.archetypes.len().into();
        let archetype = Archetype::new(archetype_id, soa_comp_ids.into(), aos_comp_ids.into());
        self.archetypes.push(archetype);
        self.tables
            .insert(archetype_id, TableStorage::new(archetype_id, self));
        self.compids_archid_map.insert(comp_ids, archetype_id);
        self.depend_graph.insert_archetype(archetype_id);

        archetype_id
    }

    pub(crate) fn create_or_get_component<T: Component>(&mut self) -> ComponentId {
        self.create_or_get_component_by_typeid::<T>(TypeId::of::<T>())
    }

    pub(crate) fn create_or_get_component_by_typeid<T: Component>(
        &mut self,
        type_id: TypeId,
    ) -> ComponentId {
        if let Some(comp_id) = self.typeid_compid_map.get(&type_id) {
            return *comp_id;
        }

        let comp_id: u32 = self
            .components
            .len()
            .try_into()
            .expect("Component Ids have increased over their max possible u32 value!");
        let comp_info = ComponentInfo::new::<T>(comp_id);
        self.components.push(comp_info);
        self.typeid_compid_map.insert(type_id, ComponentId(comp_id));
        let comp_id = ComponentId(comp_id);
        self.depend_graph.insert_component(comp_id);
        comp_id
    }
}

#[derive(Debug)]
pub enum SplitError<'map, V>{
    SameKey(&'map mut V),
    OnlyOneValue(&'map mut V),
    NoValueFound
}

pub trait SplitMut<K: Eq, V>{
    fn split_mut2<'a>(&'a mut self, key1: &K, key2: &K) -> Result<(&'a mut V, &'a mut V), SplitError<'a, V>>;
}

impl<K: Eq + Hash, V> SplitMut<K, V> for HashMap<K, V>{
    fn split_mut2<'a>(&'a mut self, key1: &K, key2: &K) -> Result<(&'a mut V, &'a mut V), SplitError<'a, V>> {
        if key1 == key2{
            return match self.get_mut(key1) {
                Some(val) => Err(SplitError::SameKey(val)),
                None => Err(SplitError::NoValueFound),
            };
        }
        let val1 = self.get_mut(key1).map(|v| v as *mut V);
        let val2 = self.get_mut(key2).map(|v| v as *mut V);

        match (val1, val2) {
            (Some(val1), Some(val2)) => {
                unsafe{Ok((&mut *val1, &mut *val2))}
            }
            (Some(val1), None) => unsafe{ Err(SplitError::OnlyOneValue(&mut *val1)) },
            (None, Some(val2)) => unsafe{ Err(SplitError::OnlyOneValue(&mut *val2)) },
            (None, None) => Err(SplitError::NoValueFound),
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::ecs::prelude::*;

    #[derive(Debug)]
    #[allow(unused)]
    struct Comp1(usize);
    impl Component for Comp1{}

    #[derive(Debug)]
    struct Comp2(u8, String);
    impl Component for Comp2{}

    fn test_add_component_to_entity_system(mut commands: Commands, mut query: Query<(EntityKey, &mut Comp1), Without<Comp2>>){
        if let Some((ek, c)) = query.iter().next(){
            dbg!(ek);
            dbg!(c);
            commands.add_component(ek, Comp2(8, "bebew".into()));
            commands.spawn(Comp2(7, "abw".to_string()));
            commands.remove_component::<Comp1>(ek);
        }
    }

    fn test_add_component_to_entity_system2(mut commands: Commands, mut query: Query<(EntityKey, &mut Comp2), Without<Comp1>>){
        for (ek, c) in query.iter(){
            dbg!(ek);
            dbg!(&c);
            commands.add_component(ek, Comp1(7));
            commands.remove_component::<Comp2>(ek);

            assert!(&c.1 == "abw" || &c.1 == "bebew");
            assert!(c.0 == 7 || c.0 == 8);
        }
    }

    #[test]
    fn test_add_component_to_entity(){
        let mut world = World::new();
        world.add_systems(test_add_component_to_entity_system.before(test_add_component_to_entity_system2));
        for _i in 0..10 {
            world.add_entity(Comp1(90));
            world.add_entity(Comp2(7, "abw".to_string()));
        }

        world.init_and_run();
        world.run();
    }
}
