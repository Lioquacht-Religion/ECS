// component.rs

use std::{
    alloc::Layout, any::TypeId, borrow::Cow, collections::HashMap, hash::Hash, mem::needs_drop,
    ptr::drop_in_place, sync::atomic::AtomicUsize, u32, usize,
};

use crate::utils::{gen_vec::Key, sorted_vec::SortedVec};

pub type Map<K, V> = HashMap<K, V>;

pub trait Component: 'static {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;

    //TODO: is this useful?
    fn get_comp_id() -> usize {
        static COMP_ID: AtomicUsize = AtomicUsize::new(0);
        let next_id = COMP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        next_id
    }
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
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) row_id: u32,
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
