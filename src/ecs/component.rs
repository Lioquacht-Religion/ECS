// component.rs

use std::{
    alloc::Layout,
    any::{Any, TypeId},
    borrow::Cow,
    collections::HashMap,
    mem::needs_drop,
    ptr::drop_in_place,
};

use crate::utils::gen_vec::GenVec;

pub trait Component: 'static {}

pub struct ComponentInfo {
    pub(crate) name: Cow<'static, str>,
    pub(crate) comp_id: ComponentId,
    pub(crate) type_id: TypeId,
    pub(crate) layout: Layout,
    pub(crate) drop: Option<unsafe fn(*mut u8)>,
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ComponentId(u32);

impl From<ComponentId> for usize {
    fn from(value: ComponentId) -> Self {
        value
            .0
            .try_into()
            .expect("Archetype Ids have increased over their max possible u32 value!")
    }
}

pub struct Archetype {
    pub(crate) id: u32,
    pub(crate) table_id: u32,
    pub(crate) type_id: TypeId,
    pub(crate) comp_type_ids: Vec<TypeId>,
    pub(crate) comp_info_ids: Vec<ComponentId>,
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ArchetypeId(u32);

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
pub struct Entity(EntityId, ArchetypeId);

impl ComponentInfo {
    unsafe fn drop_ptr<T>(ptr: *mut u8) {
        let typed_ptr: *mut T = ptr.cast::<T>();
        let _ = drop_in_place(typed_ptr);
    }

    pub fn new<T: Component>(comp_id: u32) -> Self {
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

pub struct EntityStorage {
    pub(crate) entities: GenVec<Entity>,
    pub(crate) components: Vec<ComponentInfo>,
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) type_arch_map: HashMap<TypeId, ArchetypeId>,
    pub(crate) tables_soa: Vec<Box<dyn Any>>,
}

impl EntityStorage {
    pub fn add_new_component<T: Component>(&mut self) {
        let comp_id: u32 = self
            .components
            .len()
            .try_into()
            .expect("Component Ids have increased over their max possible u32 value!");
        let comp_info = ComponentInfo::new::<T>(comp_id);
        self.components.push(comp_info);
    }
}
