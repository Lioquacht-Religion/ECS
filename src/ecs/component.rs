// component.rs

use core::panic;
use std::{
    alloc::Layout,
    any::TypeId,
    borrow::Cow,
    collections::{HashMap, HashSet},
    hash::Hash,
    mem::needs_drop,
    ptr::drop_in_place,
    u32,
};

use crate::utils::{
    gen_vec::{GenVec, Key},
    sorted_vec::SortedVec,
    tuple_iters::{self, TableStorageTupleIter, TupleIterConstructor},
    tuple_types::TupleTypesExt,
};

use super::storages::{table_aos::TableAoS, table_soa::TableSoA};

pub trait Component: 'static {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

#[derive(Debug)]
pub struct ComponentInfo {
    pub(crate) name: Cow<'static, str>,
    pub(crate) comp_id: ComponentId,
    pub(crate) type_id: TypeId,
    pub(crate) layout: Layout,
    pub(crate) drop: Option<unsafe fn(*mut u8)>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ComponentId(pub(crate) u32);

impl From<ComponentId> for usize {
    fn from(value: ComponentId) -> Self {
        value
            .0
            .try_into()
            .expect("Archetype Ids have increased over their max possible u32 value!")
    }
}

pub struct Archetype {
    pub(crate) archetype_id: ArchetypeId,
    //pub(crate) comp_type_ids: Vec<TypeId>, TODO: is this needed?
    pub(crate) soa_comp_ids: SortedVec<ComponentId>,
    pub(crate) aos_comp_ids: SortedVec<ComponentId>,
}

impl Archetype {
    pub fn new(
        archetype_id: ArchetypeId,
        soa_comp_ids: SortedVec<ComponentId>,
        aos_comp_ids: SortedVec<ComponentId>,
    ) -> Self {
        Self {
            archetype_id,
            soa_comp_ids,
            aos_comp_ids,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ArchetypeId(pub u32);

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ArchetypeHash(u32);

impl From<u32> for ArchetypeId {
    fn from(value: u32) -> Self {
        ArchetypeId(value)
    }
}

impl From<usize> for ArchetypeId {
    fn from(value: usize) -> Self {
        ArchetypeId(
            value
                .try_into()
                .expect("Archetype Ids have increased over their max possible u32 value!"),
        )
    }
}

impl From<ArchetypeId> for u32 {
    fn from(value: ArchetypeId) -> Self {
        value.0
    }
}

impl From<ArchetypeId> for usize {
    fn from(value: ArchetypeId) -> Self {
        value
            .0
            .try_into()
            .expect("Archetype Ids have increased over their max possible u32 value!")
    }
}

//TODO: what about generational index?
#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct EntityId(u32);

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct EntityKey(pub(crate) Key);

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct Entity {
    archetype_id: ArchetypeId,
    row_id: u32,
}

impl ComponentInfo {
    unsafe fn drop_ptr<T>(ptr: *mut u8) {
        let typed_ptr: *mut T = ptr.cast::<T>();
        let _ = drop_in_place(typed_ptr);
    }

    pub fn new<T: 'static>(comp_id: u32) -> Self {
        Self {
            name: Cow::Borrowed(core::any::type_name::<T>()),
            comp_id: ComponentId(comp_id),
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T>),
        }
    }
}

pub enum StorageTypes {
    TableAoS,
    TableSoA,
    SparseSet,
}

pub struct TableStorage {
    pub(crate) table_soa: TableSoA,
    pub(crate) table_aos: TableAoS,
    pub(crate) len: u32,
}

impl TableStorage {
    pub(crate) fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        Self {
            table_soa: TableSoA::new(archetype_id, entity_storage),
            table_aos: TableAoS::new(archetype_id, entity_storage),
            len: 0,
        }
    }

    pub(crate) unsafe fn insert<T: TupleTypesExt>(
        &mut self,
        entity: EntityKey,
        component_infos: &[ComponentInfo],
        soa_comp_ids: &[ComponentId],
        aos_comp_ids: &[ComponentId],
        mut value: T,
    ) -> u32 {
        let row_id = self.len;

        let mut soa_ptrs = Vec::new();
        let mut aos_ptrs = Vec::new();

        value.self_get_value_ptrs_by_storage(&mut soa_ptrs, &mut aos_ptrs);

        self.table_soa
            .insert(entity, component_infos, &soa_comp_ids, &soa_ptrs);
        self.table_aos
            .insert(entity, component_infos, &aos_comp_ids, &aos_ptrs);
        std::mem::forget(value);

        self.len += 1;
        row_id
    }

    pub unsafe fn tuple_iter<'a, TC: TupleIterConstructor<TableStorage>>(
        &'a mut self,
    ) -> TableStorageTupleIter<TC::Construct<'a>> {
        tuple_iters::new_table_storage_iter::<TC>(self)
    }
}

pub struct EntityStorage {
    pub(crate) entities: GenVec<Entity>,
    pub(crate) components: Vec<ComponentInfo>,
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) tables: HashMap<ArchetypeId, TableStorage>,
    //mapping data
    pub(crate) typeid_compid_map: HashMap<TypeId, ComponentId>,
    pub(crate) compid_archids_map: HashMap<ComponentId, HashSet<ArchetypeId>>,
    pub(crate) compids_archid_map: HashMap<SortedVec<ComponentId>, ArchetypeId>,
}

impl EntityStorage {
    pub fn new() -> Self {
        Self {
            entities: GenVec::new(),
            components: Vec::new(),
            archetypes: Vec::new(),
            tables: HashMap::new(),
            typeid_compid_map: HashMap::new(),
            compid_archids_map: HashMap::new(),
            compids_archid_map: HashMap::new(),
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

    pub fn find_fitting_archetypes2(&self, comp_ids: &SortedVec<ComponentId>) -> Vec<ArchetypeId> {
        let _archid_set: HashSet<ArchetypeId> = HashSet::new();
        comp_ids.iter().for_each(|_cids| {});
        unimplemented!()
    }

    pub fn add_entity<T: TupleTypesExt>(&mut self, input: T) -> EntityKey {
        let archetype_id = self.create_or_get_archetype::<T>();
        let key = self.entities.insert(Entity {
            archetype_id,
            row_id: 0,
        });

        let mut soa_comp_ids = Vec::new();
        let mut aos_comp_ids = Vec::new();
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
                    input,
                )
        };

        if let Some(e) = self.entities.get_mut(&key) {
            e.row_id = row_id;
        }
        EntityKey(key)
    }

    pub fn create_or_get_archetype<T: TupleTypesExt>(&mut self) -> ArchetypeId {
        //TODO; provide SortedVec from outside for reuse, to avoid allocations
        let mut type_ids = Vec::new();
        T::type_ids_rec(&mut type_ids);
        let mut comp_ids: Vec<ComponentId> = Vec::with_capacity(T::get_tuple_length());
        T::create_or_get_component(self, &mut comp_ids);
        let comp_ids: SortedVec<ComponentId> = comp_ids.into();

        if let Some(archetype_id) = self.compids_archid_map.get(&comp_ids) {
            return *archetype_id;
        }

        if let Some(_dup_compid) = comp_ids.check_duplicates() {
            //TODO: use dup_compid to get more error info
            panic!("INVALID: Same component contained multiple times inside of entity.");
        }

        let mut soa_comp_ids: Vec<ComponentId> = Vec::with_capacity(comp_ids.get_vec().len());
        let mut aos_comp_ids: Vec<ComponentId> = Vec::with_capacity(comp_ids.get_vec().len());
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

#[cfg(test)]
mod test {
    use crate::utils::tuple_types::TupleTypesExt;

    use super::Component;

    struct Type1 {
        f1: usize,
    }
    impl Component for Type1 {}

    #[test]
    fn test_entity_storage() {
        todo!()
    }

    #[test]
    fn test_tuple_ext_methods() {
        let t = Type1 { f1: 4324 };
        let mut vec = Vec::new();
        t.self_type_ids_rec(&mut vec);
    }
}
