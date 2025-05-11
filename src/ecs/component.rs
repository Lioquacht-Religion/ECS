// component.rs

use std::{alloc::Layout, any::{Any, TypeId}, borrow::Cow, collections::HashMap, mem::needs_drop, ptr::drop_in_place};

use crate::utils::gen_vec::GenVec;

pub trait Component: 'static {}

pub struct ComponentInfo {
    name: Cow<'static, str>,
    comp_id: ComponentId,
    type_id: TypeId,
    layout: Layout,
    drop: Option<unsafe fn(*mut u8)>,
}

#[derive(Eq, PartialEq, Clone, Hash, Debug)]
pub struct ComponentId(usize);

struct Archetype{
    id: u32,
    table_id: u32,
    type_id: TypeId,
    comp_type_ids: Vec<TypeId>,
    comp_info_ids: Vec<u32>,
}

pub struct ArchetypeId(u32);

struct Entity;

impl ComponentInfo {
    unsafe fn drop_ptr<T>(ptr: *mut u8) {
        let typed_ptr: *mut T = ptr.cast::<T>();
        let _ = drop_in_place(typed_ptr);
    }

    pub fn new<T: Component>(comp_id: usize) -> Self {
        Self {
            name: Cow::Borrowed(core::any::type_name::<T>()),
            comp_id: ComponentId(comp_id),
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T>),
        }
    }
}

pub enum StorageTypes{
    TableAoS,
    TableSoA,
    SparseSet,
}

struct EntityStorage {
    entities: GenVec<Entity>,
    components: Vec<ComponentInfo>,
    archetypes: Vec<Archetype>,
    type_arch_map: HashMap<TypeId, ArchetypeId>,
    tables_soa: Vec<Box<dyn Any>>,
}
