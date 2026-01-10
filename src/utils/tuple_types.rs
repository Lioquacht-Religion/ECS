// tuple_types.rs
//TODO:
// - add get_tuple_length function
// - use length function to initialize vec with capacity
// - add wrapper functions which create their own vecs

use std::{alloc::Layout, any::TypeId, ptr::NonNull};

use crate::{
    all_tuples,
    ecs::{
        component::{Component, ComponentId, StorageTypes},
        entity::EntityKey,
        storages::entity_storage::EntityStorage,
        world::WorldData,
    },
};

pub trait TupleTypesExt: 'static {
    fn type_ids() -> Vec<TypeId> {
        let mut vec: Vec<TypeId> = Vec::with_capacity(Self::get_tuple_length());
        Self::type_ids_rec(&mut vec);
        vec
    }
    fn type_ids_rec(vec: &mut Vec<TypeId>);
    fn type_layouts() -> Vec<Layout> {
        let mut vec: Vec<Layout> = Vec::with_capacity(Self::get_tuple_length());
        Self::type_layouts_rec(&mut vec);
        vec
    }
    fn type_layouts_rec(vec: &mut Vec<Layout>);

    fn self_type_ids(&self) -> Vec<TypeId> {
        Self::type_ids()
    }
    fn self_type_ids_rec(&self, vec: &mut Vec<TypeId>) {
        Self::type_ids_rec(vec);
    }
    fn self_layouts(&self) -> Vec<Layout> {
        Self::type_layouts()
    }
    fn self_layouts_rec(&self, vec: &mut Vec<Layout>) {
        Self::type_layouts_rec(vec)
    }

    fn self_get_elem_ptrs(&mut self) -> Vec<NonNull<u8>> {
        let mut vec: Vec<NonNull<u8>> = Vec::with_capacity(Self::get_tuple_length());
        self.self_get_elem_ptrs_rec(&mut vec);
        vec
    }
    fn self_get_elem_ptrs_rec(&mut self, vec: &mut Vec<NonNull<u8>>) {
        vec.push(NonNull::new(self as *mut Self as *mut u8).unwrap());
    }

    fn get_tuple_length() -> usize {
        let mut len = 0;
        Self::get_tuple_length_rec(&mut len);
        len
    }
    fn self_get_tuple_length(&mut self) -> usize {
        Self::get_tuple_length()
    }
    fn get_tuple_length_rec(len: &mut usize) {
        *len += 1;
    }
    fn create_or_get_component(entity_storage: &mut EntityStorage, vec: &mut Vec<ComponentId>);
    fn get_comp_ids_by_storage(
        entity_storage: &mut EntityStorage,
        soa_vec: &mut Vec<ComponentId>,
        aos_vec: &mut Vec<ComponentId>,
    );
    fn self_get_comp_ids_by_storage(
        entity_storage: &mut EntityStorage,
        soa_vec: &mut Vec<ComponentId>,
        aos_vec: &mut Vec<ComponentId>,
    ) {
        Self::get_comp_ids_by_storage(entity_storage, soa_vec, aos_vec);
    }
    fn self_get_value_ptrs_by_storage(
        &mut self,
        soa_vec: &mut Vec<NonNull<u8>>,
        aos_vec: &mut Vec<NonNull<u8>>,
    );
    fn on_add() -> Option<for<'a> fn(world: &mut WorldData, entity: EntityKey)> {
        None
    }
    fn exec_on_add_rec(world_data: &mut WorldData, entity: EntityKey);
    fn on_remove() -> Option<for<'a> fn(world_data: &mut WorldData, entity: EntityKey)> {
        None
    }
    fn exec_on_remove_rec(world_data: &mut WorldData, entity: EntityKey);
}

impl<T: Component> TupleTypesExt for T {
    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn type_layouts_rec(vec: &mut Vec<Layout>) {
        vec.push(Layout::new::<T>());
    }

    fn create_or_get_component(entity_storage: &mut EntityStorage, vec: &mut Vec<ComponentId>) {
        vec.push(entity_storage.create_or_get_component::<T>());
    }
    fn get_comp_ids_by_storage(
        entity_storage: &mut EntityStorage,
        soa_vec: &mut Vec<ComponentId>,
        aos_vec: &mut Vec<ComponentId>,
    ) {
        match T::STORAGE {
            StorageTypes::TableSoA => soa_vec.push(entity_storage.create_or_get_component::<T>()),
            StorageTypes::TableAoS => aos_vec.push(entity_storage.create_or_get_component::<T>()),
            StorageTypes::SparseSet => unimplemented!(),
        }
    }
    fn self_get_value_ptrs_by_storage(
        &mut self,
        soa_vec: &mut Vec<NonNull<u8>>,
        aos_vec: &mut Vec<NonNull<u8>>,
    ) {
        match T::STORAGE {
            StorageTypes::TableSoA => self.self_get_elem_ptrs_rec(soa_vec),
            StorageTypes::TableAoS => self.self_get_elem_ptrs_rec(aos_vec),
            StorageTypes::SparseSet => unimplemented!(),
        }
    }
    fn on_add() -> Option<for<'a> fn(world_data: &mut WorldData, entity: EntityKey)> {
        T::on_add()
    }
    fn exec_on_add_rec(world_data: &mut WorldData, entity: EntityKey) {
        if let Some(on_add) = Self::on_add() {
            on_add(world_data, entity);
        }
    }
    fn on_remove() -> Option<for<'a> fn(world_data: &mut WorldData, entity: EntityKey)> {
        T::on_remove()
    }
    fn exec_on_remove_rec(world_data: &mut WorldData, entity: EntityKey) {
        if let Some(on_remove) = Self::on_add() {
            on_remove(world_data, entity);
        }
    }
}

impl TupleTypesExt for () {
    fn type_ids_rec(_vec: &mut Vec<TypeId>) {}
    fn type_layouts_rec(_vec: &mut Vec<Layout>) {}
    fn self_get_elem_ptrs_rec(&mut self, _vec: &mut Vec<NonNull<u8>>) {}
    fn create_or_get_component(_entity_storage: &mut EntityStorage, _vec: &mut Vec<ComponentId>) {}
    fn get_comp_ids_by_storage(
        _entity_storage: &mut EntityStorage,
        _soa_vec: &mut Vec<ComponentId>,
        _aos_vec: &mut Vec<ComponentId>,
    ) {
    }
    fn self_get_value_ptrs_by_storage(
        &mut self,
        _soa_vec: &mut Vec<NonNull<u8>>,
        _aos_vec: &mut Vec<NonNull<u8>>,
    ) {
    }
    fn exec_on_add_rec(_world_data: &mut WorldData, _entity: EntityKey) {}
    fn exec_on_remove_rec(_world_data: &mut WorldData, _entity: EntityKey) {}
}

macro_rules! impl_tuple_ext {
    ($($t:ident), *) => {
       impl<$($t : TupleTypesExt), *> TupleTypesExt for ($($t),*,){
            fn type_ids_rec(vec: &mut Vec<TypeId>){
               $($t::type_ids_rec(vec);)*
            }
            fn type_layouts_rec(vec: &mut Vec<Layout>) {
               $($t::type_layouts_rec(vec);)*
            }
            fn self_get_elem_ptrs_rec(&mut self, vec: &mut Vec<NonNull<u8>>){
                 #[allow(non_snake_case)]
                 let ( $($t,)+ ) = self;
               $($t::self_get_elem_ptrs_rec($t, vec);)*
            }
            fn get_tuple_length() -> usize{
               let mut len = 0;
               $($t::get_tuple_length_rec(&mut len);)*
               len
            }
            fn get_tuple_length_rec(len: &mut usize){
               $($t::get_tuple_length_rec(len);)*
            }
            fn create_or_get_component(entity_storage : &mut EntityStorage, vec: &mut Vec<ComponentId>) {
               $($t::create_or_get_component(entity_storage, vec);)*
            }
            fn get_comp_ids_by_storage(
               entity_storage: &mut EntityStorage,
               soa_vec: &mut Vec<ComponentId>,
               aos_vec: &mut Vec<ComponentId>
            ) {
               $($t::get_comp_ids_by_storage(entity_storage, soa_vec, aos_vec);)*
            }
            fn self_get_value_ptrs_by_storage(
                    &mut self,
                    soa_vec: &mut Vec<NonNull<u8>>,
                    aos_vec: &mut Vec<NonNull<u8>>,
            ) {
                #[allow(non_snake_case)]
                let ( $($t,)+ ) = self;
                $($t::self_get_value_ptrs_by_storage($t, soa_vec, aos_vec);)*
            }
            fn exec_on_add_rec(world_data: &mut WorldData, entity: EntityKey){
                $($t::exec_on_add_rec(world_data, entity);)*
            }
            fn exec_on_remove_rec(world_data: &mut WorldData, entity: EntityKey){
                $($t::exec_on_remove_rec(world_data, entity);)*
            }
       }
    };
}

#[rustfmt::skip]
all_tuples!(
    impl_tuple_ext,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

#[cfg(test)]
mod test {
    use super::{Component, TupleTypesExt};

    impl Component for usize {}
    impl Component for String {}
    impl Component for f32 {}

    #[test]
    fn it_works() {
        let mut t: (usize, (usize, usize), f32, String) = (23, (23, 43), 4.0, String::from("eefe"));
        let mut vec_typeids = Vec::new();
        let mut vec_layouts = Vec::new();
        let mut vec_ptrs = Vec::new();

        t.self_type_ids_rec(&mut vec_typeids);
        t.self_layouts_rec(&mut vec_layouts);
        t.self_get_elem_ptrs_rec(&mut vec_ptrs);

        assert_eq!(vec_typeids.len(), 5);
        assert_eq!(vec_layouts.len(), 5);
        assert_eq!(vec_ptrs.len(), 5);

        assert_eq!(t.self_get_tuple_length(), 5);

        let t3: (
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
        ) = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);

        vec_typeids.clear();
        t3.self_type_ids_rec(&mut vec_typeids);
        assert_eq!(vec_typeids.len(), 12);
    }
}
