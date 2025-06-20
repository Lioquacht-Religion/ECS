// tuple_iters.rs

use std::{any::TypeId, marker::PhantomData};

use crate::ecs::{component::Component, storages::{table_soa::TableSoA, thin_blob_vec::{ThinBlobIterMutUnsafe, ThinBlobIterUnsafe}}};

pub trait TupleIterConstructor{
    type Construct<'c>: TupleIterator;
    unsafe fn construct<'s>(source: *mut TableSoA) -> Self::Construct<'s>;
}

impl<T: Component> TupleIterConstructor for &T{
    type Construct<'c> = ThinBlobIterUnsafe<'c, T>;
    unsafe fn construct<'s>(source: *mut TableSoA) -> Self::Construct<'s>{
        unsafe{
            (&*source).columns.get(&TypeId::of::<T>())
            .expect("ERROR: TableSoA does not contain a column with this type id")
            .tuple_iter()
        }
    }
}

impl<T: Component> TupleIterConstructor for &mut T{
    type Construct<'c> = ThinBlobIterMutUnsafe<'c, T>;
    unsafe fn construct<'s>(source: *mut TableSoA) -> Self::Construct<'s>{
        unsafe{
            (&mut *source).columns.get_mut(&TypeId::of::<T>())
            .expect("ERROR: TableSoA does not contain a column with this type id")
            .tuple_iter_mut()
        }
    }
}

impl<T1: TupleIterConstructor, T2: TupleIterConstructor> TupleIterConstructor for (T1, T2){
    type Construct<'c> = (T1::Construct<'c>, T2::Construct<'c>);
    unsafe fn construct<'s>(source: *mut TableSoA) -> Self::Construct<'s>{
        (T1::construct(source), T2::construct(source))
    }
}

//TODO: maybe generize constructor trait
pub trait TupleConstructorSource<T, E>{
    type Item: TupleIterator;
    fn retrieve_tuple(&mut self) -> Self::Item;
    fn retrieve_elem(&mut self) -> Self::Item;
    fn retrieve_elem_mut(&mut self) -> Self::Item;
}

pub trait TupleIterator{
    type Item;
    fn next(&mut self, index: usize) -> Self::Item;
}

pub struct TableSoaTupleIter<T: TupleIterator>{
    tuple_iters: T,
    len: usize,
    index: usize,
}

//impl<T: TupleIterator> TableSoaTupleIter<T>{
    pub fn new<'table, T: TupleIterator, TC: TupleIterConstructor<Construct<'table> = T>>(table : &'table mut TableSoA) 
    -> TableSoaTupleIter<T>{
        unsafe{
        TableSoaTupleIter{
            tuple_iters: TC::construct(table),
            len: table.len,
            index: 0,
        }
        }
    }
//}

impl<T: TupleIterator> Iterator for TableSoaTupleIter<T>{
    type Item = T::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.len{
            self.index += 1;
            Some(self.tuple_iters.next(self.index))
        }
        else{
            None
        }
    }
}

impl<T1: TupleIterator, T2: TupleIterator> TupleIterator for (T1, T2){
    type Item = (T1::Item, T2::Item);
    fn next(&mut self, index: usize) -> Self::Item {
        let (i1, i2) = self;
        (i1.next(index), i2.next(index)) 
    }
}

#[cfg(test)]
mod test{
    #[test]
    fn test_tuple_iter(){
    }
}
