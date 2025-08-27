// tuple_iters.rs

use std::any::TypeId;

use crate::{
    all_tuples,
    ecs::{
        component::{Component, StorageTypes}, entity::EntityKey, storages::{
            table_aos::TableAoS,
            table_soa::TableSoA,
            table_storage::TableStorage,
            thin_blob_vec::{
                ThinBlobInnerTypeIterMutUnsafe, ThinBlobInnerTypeIterUnsafe, ThinBlobIterMutUnsafe,
                ThinBlobIterUnsafe,
            },
        }
    },
};

pub trait TupleIterator {
    type Item;
    ///SAFETY: This function does not check if iterator is still in the valid range.
    /// Bounds check needs to be tracked from outside the function.
    unsafe fn next(&mut self, index: usize) -> Self::Item;
}

pub trait TupleConstructorSource: 'static {
    type IterType<'c, T: Component>: TupleIterator
    where
        Self: 'c;
    type IterMutType<'c, T: Component>: TupleIterator
    where
        Self: 'c;
    fn get_entity_key_iter<'c>(&'c mut self) -> EntityKeyIterUnsafe<'c>;
    unsafe fn get_iter<'c, T: Component>(&'c mut self) -> Self::IterType<'c, T>;
    unsafe fn get_iter_mut<'c, T: Component>(&'c mut self) -> Self::IterMutType<'c, T>;
}

pub trait TupleIterConstructor<S: TupleConstructorSource> {
    type Construct<'c>: TupleIterator;
    unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s>;
}

impl TupleConstructorSource for TableSoA {
    type IterType<'c, T: Component> = ThinBlobIterUnsafe<'c, T>;
    type IterMutType<'c, T: Component> = ThinBlobIterMutUnsafe<'c, T>;
    fn get_entity_key_iter<'c>(&'c mut self) -> EntityKeyIterUnsafe<'c> {
        todo!()
    }
    unsafe fn get_iter<'c, T: Component>(&'c mut self) -> Self::IterType<'c, T> {
        self.columns
            .get(&TypeId::of::<T>())
            .expect("ERROR: TableSoA does not contain a column with this type id")
            .tuple_iter()
    }
    unsafe fn get_iter_mut<'c, T: Component>(&'c mut self) -> Self::IterMutType<'c, T> {
        self.columns
            .get_mut(&TypeId::of::<T>())
            .expect("ERROR: TableSoA does not contain a column with this type id")
            .tuple_iter_mut()
    }
}

impl TupleConstructorSource for TableAoS {
    type IterType<'c, T: Component> = ThinBlobInnerTypeIterUnsafe<'c, T>;
    type IterMutType<'c, T: Component> = ThinBlobInnerTypeIterMutUnsafe<'c, T>;
    fn get_entity_key_iter<'c>(&'c mut self) -> EntityKeyIterUnsafe<'c> {
        todo!()
    }
    unsafe fn get_iter<'c, T: Component>(&'c mut self) -> Self::IterType<'c, T> {
        let index = self
            .type_meta_data_map
            .get(&TypeId::of::<T>())
            .expect("ERROR: TableAoS does not contain a column with this type id");
        let offset = self.type_meta_data.get_vec()[*index].ptr_offset;
        self.vec.tuple_inner_type_iter(offset)
    }
    unsafe fn get_iter_mut<'c, T: Component>(&'c mut self) -> Self::IterMutType<'c, T> {
        let index = self
            .type_meta_data_map
            .get(&TypeId::of::<T>())
            .expect("ERROR: TableAoS does not contain a column with this type id");
        let offset = self.type_meta_data.get_vec()[*index].ptr_offset;
        self.vec.tuple_inner_type_iter_mut(offset)
    }
}

pub enum TableStorageIterUnsafe<'c, T: Component> {
    TableSoaIter(ThinBlobIterUnsafe<'c, T>),
    TableAosIter(ThinBlobInnerTypeIterUnsafe<'c, T>),
}

impl<'c, T: Component> TupleIterator for TableStorageIterUnsafe<'c, T> {
    type Item = &'c T;
    unsafe fn next(&mut self, index: usize) -> Self::Item {
        match self {
            TableStorageIterUnsafe::TableSoaIter(iter) => iter.next(index),
            TableStorageIterUnsafe::TableAosIter(iter) => iter.next(index),
        }
    }
}

pub enum TableStorageIterMutUnsafe<'c, T: Component> {
    TableSoaIterMut(ThinBlobIterMutUnsafe<'c, T>),
    TableAosIterMut(ThinBlobInnerTypeIterMutUnsafe<'c, T>),
}

impl<'c, T: Component> TupleIterator for TableStorageIterMutUnsafe<'c, T> {
    type Item = &'c mut T;
    unsafe fn next(&mut self, index: usize) -> Self::Item {
        match self {
            TableStorageIterMutUnsafe::TableSoaIterMut(iter) => iter.next(index),
            TableStorageIterMutUnsafe::TableAosIterMut(iter) => iter.next(index),
        }
    }
}

impl TupleConstructorSource for TableStorage {
    type IterType<'c, T: Component> = TableStorageIterUnsafe<'c, T>;
    type IterMutType<'c, T: Component> = TableStorageIterMutUnsafe<'c, T>;
    fn get_entity_key_iter<'c>(&'c mut self) -> EntityKeyIterUnsafe<'c> {
        EntityKeyIterUnsafe::new(&self.entities)
    }
    unsafe fn get_iter<'c, T: Component>(&'c mut self) -> Self::IterType<'c, T> {
        match T::STORAGE {
            StorageTypes::TableSoA => {
                TableStorageIterUnsafe::TableSoaIter(self.table_soa.get_single_comp_iter())
            }
            StorageTypes::TableAoS => {
                TableStorageIterUnsafe::TableAosIter(self.table_aos.get_single_comp_iter())
            }
            StorageTypes::SparseSet => unimplemented!(),
        }
    }
    unsafe fn get_iter_mut<'c, T: Component>(&'c mut self) -> Self::IterMutType<'c, T> {
        match T::STORAGE {
            StorageTypes::TableSoA => TableStorageIterMutUnsafe::TableSoaIterMut(
                self.table_soa.get_single_comp_iter_mut(),
            ),
            StorageTypes::TableAoS => TableStorageIterMutUnsafe::TableAosIterMut(
                self.table_aos.get_single_comp_iter_mut(),
            ),
            StorageTypes::SparseSet => unimplemented!(),
        }
    }
}

impl<T: Component, S: TupleConstructorSource> TupleIterConstructor<S> for &T {
    type Construct<'c> = S::IterType<'c, T>;
    unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s> {
        (&mut *source).get_iter()
    }
}

impl<T: Component, S: TupleConstructorSource> TupleIterConstructor<S> for &mut T {
    type Construct<'c> = S::IterMutType<'c, T>;
    unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s> {
        (&mut *source).get_iter_mut()
    }
}

//TODO: move somewhere else
impl Component for EntityKey{
}

pub struct EntityKeyIterUnsafe<'vec> {
    vec: &'vec [EntityKey],
}

impl<'vec> EntityKeyIterUnsafe<'vec> {
    pub fn new(entity_keys: &'vec [EntityKey]) -> Self {
        EntityKeyIterUnsafe{
            vec: entity_keys,
        }
    }
}

impl<'vec> TupleIterator for EntityKeyIterUnsafe<'vec> {
    type Item = EntityKey;
    unsafe fn next(&mut self, index: usize) -> Self::Item {
        println!("entity key vec iter len: {}", self.vec.len());
        self.vec[index]
    }
}

impl<S: TupleConstructorSource> TupleIterConstructor<S> for EntityKey{
    type Construct<'c> = EntityKeyIterUnsafe<'c>;

    unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s> {
        (&mut *source).get_entity_key_iter()
    }
}

macro_rules! impl_tuple_iter_constructor{
    ($($t:ident), *) => {
       impl<S: TupleConstructorSource, $($t : TupleIterConstructor<S>), *> TupleIterConstructor<S> for ($($t),*,){
            #[allow(unused_parens, non_snake_case)]
            type Construct<'c> = ($($t::Construct<'c>), *);
            unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s> {
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

pub struct TableSoaTupleIter<T: TupleIterator> {
    tuple_iters: T,
    len: usize,
    index: usize,
}

pub unsafe fn new_table_soa_iter<'table, TC: TupleIterConstructor<TableSoA>>(
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
            let next = unsafe { Some(self.tuple_iters.next(self.index)) };
            self.index += 1;
            next
        } else {
            None
        }
    }
}

pub struct TableAosTupleIter<T: TupleIterator> {
    tuple_iters: T,
    len: usize,
    index: usize,
}

pub unsafe fn new_table_aos_iter<'table, TC: TupleIterConstructor<TableAoS>>(
    table: &'table mut TableAoS,
) -> TableAosTupleIter<TC::Construct<'table>> {
    unsafe {
        TableAosTupleIter {
            tuple_iters: TC::construct(table),
            len: table.len,
            index: 0,
        }
    }
}

impl<T: TupleIterator> Iterator for TableAosTupleIter<T> {
    type Item = T::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.len {
            let next = unsafe { Some(self.tuple_iters.next(self.index)) };
            self.index += 1;
            next
        } else {
            None
        }
    }
}

pub struct TableStorageTupleIter<T: TupleIterator> {
    tuple_iters: T,
    len: usize,
    index: usize,
}

pub unsafe fn new_table_storage_iter<'table, TC: TupleIterConstructor<TableStorage>>(
    table: &'table mut TableStorage,
) -> TableStorageTupleIter<TC::Construct<'table>> {
    unsafe {
        TableStorageTupleIter {
            tuple_iters: TC::construct(table),
            len: table.len as usize,
            index: 0,
        }
    }
}

impl<T: TupleIterator> Iterator for TableStorageTupleIter<T> {
    type Item = T::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.len {

            println!("table len: {}; index: {}", self.len, self.index);
            let next = unsafe { Some(self.tuple_iters.next(self.index)) };
            self.index += 1;
            next
        } else {
            None
        }
    }
}

macro_rules! impl_tuple_iterator{
    ($($t:ident), *) => {
       impl<$($t : TupleIterator), *> TupleIterator for ($($t),*,){

            #[allow(unused_parens, non_snake_case)]
            type Item = ($($t::Item),* );
            #[allow(unconditional_recursion)]
            unsafe fn next(&mut self, index: usize) -> Self::Item {
                #[allow(unused_parens, non_snake_case)]
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
