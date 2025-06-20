// tuple_types.rs
//TODO:
// - add get_tuple_length function
// - use length function to initialize vec with capacity
// - add wrapper functions which create their own vecs

use std::{alloc::Layout, any::TypeId};

use crate::{
    all_tuples,
    ecs::component::{Component, ComponentId, EntityStorage},
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

    fn self_get_elem_ptrs(&mut self) -> Vec<*mut u8> {
        let mut vec: Vec<*mut u8> = Vec::with_capacity(Self::get_tuple_length());
        self.self_get_elem_ptrs_rec(&mut vec);
        vec
    }
    fn self_get_elem_ptrs_rec(&mut self, vec: &mut Vec<*mut u8>) {
        vec.push(self as *mut Self as *mut u8);
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
}

impl TupleTypesExt for () {
    fn type_ids_rec(_vec: &mut Vec<TypeId>) {}
    fn type_layouts_rec(_vec: &mut Vec<Layout>) {}
    fn self_get_elem_ptrs_rec(&mut self, _vec: &mut Vec<*mut u8>) {}
    fn create_or_get_component(_entity_storage: &mut EntityStorage, _vec: &mut Vec<ComponentId>) {
        unimplemented!()
    }
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
               fn self_get_elem_ptrs_rec(&mut self, vec: &mut Vec<*mut u8>){
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
