// tuple_types.rs

use std::{alloc::Layout, any::TypeId};

use crate::all_tuples;

pub trait TupleTypesExt {
    fn type_ids() -> Vec<TypeId>;
    fn type_layouts() -> Vec<Layout>;

    fn self_type_ids(&self) -> Vec<TypeId> {
        Self::type_ids()
    }
    fn self_layouts(&self) -> Vec<Layout> {
        Self::type_layouts()
    }

    fn self_get_elem_ptrs(&mut self) -> Vec<*mut ()>;
}

/*
impl<T : 'static> TupleTypesExt for T
{
    fn type_ids() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }
    fn type_layouts() -> Vec<Layout> {
        vec![Layout::new::<T>()]
    }
    fn self_get_elem_ptrs(&mut self) -> Vec<*mut ()> {
        vec![self as *mut Self as *mut ()]
    }
}*/

impl TupleTypesExt for () {
    fn type_ids() -> Vec<TypeId> {
        vec![]
    }
    fn type_layouts() -> Vec<Layout> {
        vec![]
    }
    fn self_get_elem_ptrs(&mut self) -> Vec<*mut ()> {
        vec![]
    }
}

macro_rules! impl_tuple_ext {
    ($($t:ident), *) => {
       impl<$($t : 'static), *> TupleTypesExt for ($($t),*,){
               fn type_ids() -> Vec<TypeId> {
                   vec![$(TypeId::of::<$t>()), *]
               }
               fn type_layouts() -> Vec<Layout> {
                   vec![$(Layout::new::<$t>()), *]
               }
               fn self_get_elem_ptrs(&mut self) -> Vec<*mut ()>{
                     #[allow(non_snake_case)]
                     let ( $($t,)+ ) = self;
                   vec![
                   $(
                       $t as *mut $t as *mut (),
                   )*
                   ]
               }
       }
    };
}

macro_rules! impl_tuple_ext2 {
    ($($t:ident), *) => {
       impl<$($t : 'static), *> TupleTypesExt for ($($t),*,){
               fn type_ids() -> Vec<TypeId> {
                   vec![$(TypeId::of::<$t>()), *]
               }
               fn type_layouts() -> Vec<Layout> {
                   vec![$(Layout::new::<$t>()), *]
               }
               fn self_get_elem_ptrs(&mut self) -> Vec<*mut ()>{
                     #[allow(non_snake_case)]
                     let ( $($t,)+ ) = self;
                   vec![
                   $(
                       $t as *mut $t as *mut (),
                   )*
                   ]
               }
       }
    };
}

all_tuples!(
    impl_tuple_ext,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    T9,
    T10,
    T11,
    T12,
    T13,
    T14,
    T15,
    T16
);

#[cfg(test)]
mod test {
    use super::TupleTypesExt;

    #[test]
    fn it_works() {
        let t: (usize, (usize, usize), f32, String) = (23, (23, 43), 4.0, String::from("eefe"));

        t.self_layouts();

        let t2: (usize) = (43);
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
    }
}
