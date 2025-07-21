// table_addable.rs

use std::any::TypeId;

use crate::{all_tuples, ecs::component::Component, utils::tuple_types::TupleTypesExt};

use super::table_soa::TableSoA;

pub trait TableAddable: 'static + TupleTypesExt {
    type Input: TableAddable;
    //unsafe fn insert(){}
    fn insert_soa(table_soa: &mut TableSoA, input: Self::Input);
}

impl<T: Component> TableAddable for T {
    type Input = T;
    fn insert_soa(table_soa: &mut TableSoA, input: Self::Input) {
        /*
        SAFETY:
         Type safety is ensured by comparing of TypeId's.
        */
        unsafe {
            table_soa
                .columns
                .get_mut(&TypeId::of::<T>())
                .expect("Type T is not stored inside this table!")
                .push_typed(table_soa.cap, table_soa.len, input);
        }
    }
}

/*
macro_rules! impl_soa_addable_ext {
    ($($t:ident), *) => {
       impl<$($t : TableAddable<Input = $t>), *> TableAddable for ($($t),*,){
            type Input = ($($t,)*);
            fn insert_soa(table_soa: &mut TableSoA, input: Self::Input) {
                #[allow(non_snake_case)]
                let ($($t,)*) = input;
                $($t::insert_soa(table_soa, $t);)*
            }
       }
    }
}

#[rustfmt::skip]
all_tuples!(
    impl_soa_addable_ext,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);
*/
