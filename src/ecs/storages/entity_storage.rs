// entity_storage.rs

use std::any::TypeId;

use crate::{
    ecs::component::{
        Archetype, ArchetypeId, Component, ComponentId, ComponentInfo, Entity, EntityKey, Map
    },
    utils::{gen_vec::GenVec, sorted_vec::SortedVec, tuple_types::TupleTypesExt},
};

use super::{cache::EntityStorageCache, table_storage::TableStorage};

pub struct EntityStorage {
    pub(crate) entities: GenVec<Entity>,
    pub(crate) components: Vec<ComponentInfo>,
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) tables: Map<ArchetypeId, TableStorage>,
    //mapping data
    pub(crate) typeid_compid_map: Map<TypeId, ComponentId>,
    //pub(crate) compid_archids_map: Map<ComponentId, HashSet<ArchetypeId>>,
    pub(crate) compids_archid_map: Map<SortedVec<ComponentId>, ArchetypeId>,
    pub(crate) cache: EntityStorageCache,
}

impl EntityStorage {
    pub fn new() -> Self {
        Self {
            entities: GenVec::new(),
            components: Vec::new(),
            archetypes: Vec::new(),
            tables: Map::new(),
            typeid_compid_map: Map::new(),
            //compid_archids_map: Map::new(),
            compids_archid_map: Map::new(),
            cache: EntityStorageCache::new(),
        }
    }

    //TODO: does still not work
    pub fn find_fitting_archetypes(
        &self,
        query_comp_ids: &SortedVec<ComponentId>,
    ) -> Vec<ArchetypeId> {
        self.compids_archid_map
            .iter()
            .filter_map(|(arch_cids, arch_id)| {
                if query_comp_ids.is_subset_of(arch_cids) {
                    Some(*arch_id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn add_entity<T: TupleTypesExt>(&mut self, input: T) -> EntityKey {
        let archetype_id = self.create_or_get_archetype::<T>();
        let key = self.entities.insert(Entity {
            archetype_id,
            row_id: 0,
        });

        let mut soa_comp_ids = self.cache.compid_vec_cache.take_cached();
        let mut aos_comp_ids = self.cache.compid_vec_cache.take_cached();

        T::get_comp_ids_by_storage(self, &mut soa_comp_ids, &mut aos_comp_ids);

        let row_id = unsafe {
            self.tables
                .get_mut(&archetype_id)
                .expect("ERROR: table does not contain archetype id!")
                .insert(
                    EntityKey(key),
                    &self.components,
                    &soa_comp_ids,
                    &aos_comp_ids,
                    &mut self.cache,
                    input,
                )
        };

        self.cache.compid_vec_cache.insert(soa_comp_ids);
        self.cache.compid_vec_cache.insert(aos_comp_ids);

        if let Some(e) = self.entities.get_mut(&key) {
            e.row_id = row_id;
        }
        EntityKey(key)
    }

    pub fn create_or_get_archetype<T: TupleTypesExt>(&mut self) -> ArchetypeId {
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

        // TODO: should this vecs be taken from cache?
        let mut soa_comp_ids: Vec<ComponentId> = self.cache.compid_vec_cache.take_cached();
        let mut aos_comp_ids: Vec<ComponentId> = self.cache.compid_vec_cache.take_cached();
        T::get_comp_ids_by_storage(self, &mut soa_comp_ids, &mut aos_comp_ids);

        let archetype_id = self.archetypes.len().into();
        let archetype = Archetype::new(archetype_id, soa_comp_ids.into(), aos_comp_ids.into());
        self.archetypes.push(archetype);
        self.tables
            .insert(archetype_id, TableStorage::new(archetype_id, self));
        self.compids_archid_map.insert(comp_ids, archetype_id);

        archetype_id
    }

    pub fn create_or_get_component<T: Component>(&mut self) -> ComponentId {
        self.create_or_get_component_by_typeid::<T>(TypeId::of::<T>())
    }

    pub fn create_or_get_component_by_typeid<T: Component>(
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
        ComponentId(comp_id)
    }
}
