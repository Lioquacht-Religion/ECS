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
};

use crate::utils::{
    gen_vec::{GenVec, Key}, sorted_vec::SortedVec, tuple_types::TupleTypesExt
};

use super::storages::{table_addable::TableAddable, table_aos::TableAoS, table_soa::TableSoA};

pub trait Component: 'static {
    const STORAGE : StorageTypes = StorageTypes::TableSoA;
}

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
    pub(crate) comp_ids: SortedVec<ComponentId>,
}

impl Archetype {
    pub fn new(archetype_id: ArchetypeId, comp_ids: SortedVec<ComponentId>) -> Self {
        Self {
            archetype_id,
            comp_ids,
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

pub struct TableStorage{
    table_soa: TableSoA,
    table_aos: TableAoS,
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

    pub fn find_fitting_archetypes(&self, comp_ids: &SortedVec<ComponentId>) -> Vec<ArchetypeId> {
        self.compids_archid_map
            .iter()
            .filter_map(|(cids, arch_id)| {
                if cids == comp_ids {
                    Some(*arch_id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn find_fitting_archetypes2(&self, comp_ids: &SortedVec<ComponentId>) -> Vec<ArchetypeId> {
        let archid_set: HashSet<ArchetypeId> = HashSet::new();
        comp_ids.iter().for_each(|cids| {});
        unimplemented!()
    }

    pub fn add_entity<T: TupleTypesExt + TableAddable<Input = T>>(
        &mut self,
        input: T,
    ) -> EntityKey {
        let archetype_id = self.create_or_get_archetype::<T>();
        let key = self.entities.insert(Entity {
            archetype_id,
            row_id: 0,
        });

        let row_id = self
            .tables
            .get_mut(&archetype_id)
            .expect("ERROR: table does not contain archetype id!")
            .insert(EntityKey(key), input);
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

        let archetype_id = self.archetypes.len().into();
        let archetype = Archetype::new(archetype_id, comp_ids.clone());
        self.archetypes.push(archetype);
        self.tables_soa
            .insert(archetype_id, TableSoA::new(archetype_id, self));
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
