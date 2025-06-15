// tuple_types.rs
//TODO:
// - add get_tuple_length function
// - use length function to initialize vec with capacity
// - add wrapper functions which create their own vecs

use std::{alloc::Layout, any::TypeId};

use crate::{all_tuples, ecs::component::{Component, ComponentId, EntityStorage}};

pub trait TupleTypesExt: 'static {
    fn type_ids(vec: &mut Vec<TypeId>);

    fn type_layouts(vec: &mut Vec<Layout>);

    fn self_type_ids(&self, vec: &mut Vec<TypeId>) {
        Self::type_ids(vec);
    }
    fn self_layouts(&self, vec: &mut Vec<Layout>) {
        Self::type_layouts(vec)
    }

    fn self_get_elem_ptrs(&mut self, vec: &mut Vec<*mut ()>) {
        vec.push(self as *mut Self as *mut ());
    }

    fn create_or_get_component(entity_storage : &mut EntityStorage, vec: &mut Vec<ComponentId>);
}

impl<T: Component> TupleTypesExt for T {
    fn type_ids(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn type_layouts(vec: &mut Vec<Layout>) {
        vec.push(Layout::new::<T>());
    }

    fn create_or_get_component(entity_storage : &mut EntityStorage, vec: &mut Vec<ComponentId>){
        vec.push(entity_storage.create_or_get_component::<T>());
    }
}

impl TupleTypesExt for () {
    fn type_ids(_vec: &mut Vec<TypeId>) {}
    fn type_layouts(_vec: &mut Vec<Layout>) {}
    fn self_get_elem_ptrs(&mut self, _vec: &mut Vec<*mut ()>) {}
    fn create_or_get_component(_entity_storage : &mut EntityStorage, _vec: &mut Vec<ComponentId>) {
        unimplemented!()
    }
}

macro_rules! impl_tuple_ext {
    ($($t:ident), *) => {
       impl<$($t : TupleTypesExt), *> TupleTypesExt for ($($t),*,){
               fn type_ids(vec: &mut Vec<TypeId>){
                   $($t::type_ids(vec);)*
               }
               fn type_layouts(vec: &mut Vec<Layout>) {
                   $($t::type_layouts(vec);)*
               }

               fn self_get_elem_ptrs(&mut self, vec: &mut Vec<*mut ()>){
                     #[allow(non_snake_case)]
                     let ( $($t,)+ ) = self;
                   $($t::self_get_elem_ptrs($t, vec);)*
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

        t.self_type_ids(&mut vec_typeids);
        t.self_layouts(&mut vec_layouts);
        t.self_get_elem_ptrs(&mut vec_ptrs);

        assert_eq!(vec_typeids.len(), 5);
        assert_eq!(vec_layouts.len(), 5);
        assert_eq!(vec_ptrs.len(), 5);

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
        t3.self_type_ids(&mut vec_typeids);
        assert_eq!(vec_typeids.len(), 12);
    }
}
