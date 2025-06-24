// tuple_iters.rs

use std::any::TypeId;

use crate::{
    all_tuples,
    ecs::{
        component::Component,
        storages::{
            table_soa::TableSoA,
            thin_blob_vec::{ThinBlobIterMutUnsafe, ThinBlobIterUnsafe},
        },
    },
};

pub trait TupleIterConstructor {
    type Construct<'c>: TupleIterator;
    unsafe fn construct<'s>(source: *mut TableSoA) -> Self::Construct<'s>;
}

impl<T: Component> TupleIterConstructor for &T {
    type Construct<'c> = ThinBlobIterUnsafe<'c, T>;
    unsafe fn construct<'s>(source: *mut TableSoA) -> Self::Construct<'s> {
        unsafe {
            (&*source)
                .columns
                .get(&TypeId::of::<T>())
                .expect("ERROR: TableSoA does not contain a column with this type id")
                .tuple_iter()
        }
    }
}

impl<T: Component> TupleIterConstructor for &mut T {
    type Construct<'c> = ThinBlobIterMutUnsafe<'c, T>;
    unsafe fn construct<'s>(source: *mut TableSoA) -> Self::Construct<'s> {
        unsafe {
            (&mut *source)
                .columns
                .get_mut(&TypeId::of::<T>())
                .expect("ERROR: TableSoA does not contain a column with this type id")
                .tuple_iter_mut()
        }
    }
}

macro_rules! impl_tuple_iter_constructor{
    ($($t:ident), *) => {
       impl<$($t : TupleIterConstructor), *> TupleIterConstructor for ($($t),*,){
            #[allow(unused_parens)]
            type Construct<'c> = ($($t::Construct<'c>), *);
            unsafe fn construct<'s>(source: *mut TableSoA) -> Self::Construct<'s> {
                ($($t::construct(source)), *)
            }
        }
    }
}

#[rustfmt::skip]
all_tuples!(
    impl_tuple_iter_constructor,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

//TODO: maybe generize constructor trait
pub trait TupleConstructorSource<T, E> {
    type Item: TupleIterator;
    fn retrieve_tuple(&mut self) -> Self::Item;
    fn retrieve_elem(&mut self) -> Self::Item;
    fn retrieve_elem_mut(&mut self) -> Self::Item;
}

pub trait TupleIterator {
    type Item;
    fn next(&mut self, index: usize) -> Self::Item;
}

pub struct TableSoaTupleIter<T: TupleIterator> {
    tuple_iters: T,
    len: usize,
    index: usize,
}

pub fn new_table_soa_iter<'table, TC: TupleIterConstructor>(
    table: &'table mut TableSoA,
) -> TableSoaTupleIter<TC::Construct<'table>> {
    unsafe {
        TableSoaTupleIter {
            tuple_iters: TC::construct(table),
            len: table.len,
            index: 0,
        }
    }
}

impl<T: TupleIterator> Iterator for TableSoaTupleIter<T> {
    type Item = T::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.len {
            self.index += 1;
            Some(self.tuple_iters.next(self.index))
        } else {
            None
        }
    }
}

macro_rules! impl_tuple_iterator{
    ($($t:ident), *) => {
       impl<$($t : TupleIterator), *> TupleIterator for ($($t),*,){

            #[allow(unused_parens)]
            type Item = ($($t::Item),* );
            fn next(&mut self, index: usize) -> Self::Item {
                #[allow(unused_parens)]
                let ($($t),*) = self;
                ($($t.next(index)),*)
            }
        }
    }
}

#[rustfmt::skip]
all_tuples!(
    impl_tuple_iterator,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

#[cfg(test)]
mod test {
    #[test]
    fn test_tuple_iter() {}
}
