// table_storage.rs

use std::alloc::Layout;

use crate::{
    ecs::{
        component::{ArchetypeId, Component, ComponentId, ComponentInfo, StorageTypes},
        entity::{Entity, EntityKey, EntityKeyIterUnsafe},
        storages::thin_blob_vec::{
            ThinBlobInnerTypeIterMutUnsafe, ThinBlobInnerTypeIterUnsafe, ThinBlobIterMutUnsafe,
            ThinBlobIterUnsafe,
        },
    },
    utils::{
        tuple_iters::{TupleConstructorSource, TupleIterConstructor, TupleIterator},
        tuple_types::TupleTypesExt,
    },
};

use super::{
    cache::EntityStorageCache, entity_storage::EntityStorage, table_aos::TableAoS,
    table_soa::TableSoA,
};

pub struct TableStorage {
    pub(crate) entities: Vec<EntityKey>,
    pub(crate) table_soa: TableSoA,
    pub(crate) table_aos: TableAoS,
    pub(crate) len: u32,
}

impl TableStorage {
    pub(crate) fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        Self {
            entities: Vec::new(),
            table_soa: TableSoA::new(archetype_id, entity_storage),
            table_aos: TableAoS::new(archetype_id, entity_storage),
            len: 0,
        }
    }

    pub(crate) unsafe fn insert<T: TupleTypesExt>(
        &mut self,
        entity: EntityKey,
        component_infos: &[ComponentInfo],
        soa_comp_ids: &[ComponentId],
        aos_comp_ids: &[ComponentId],
        cache: &mut EntityStorageCache,
        mut value: T,
    ) -> u32 {
        let row_id = self.len;

        let mut soa_ptr_vec = cache.ptr_vec_cache.take_cached();
        let mut aos_ptr_vec = cache.ptr_vec_cache.take_cached();

        value.self_get_value_ptrs_by_storage(&mut soa_ptr_vec, &mut aos_ptr_vec);

        self.table_soa
            .insert(component_infos, &soa_comp_ids, &soa_ptr_vec);
        self.table_aos
            .insert(component_infos, &aos_comp_ids, &aos_ptr_vec, cache);
        std::mem::forget(value);

        cache.ptr_vec_cache.insert(soa_ptr_vec);
        cache.ptr_vec_cache.insert(aos_ptr_vec);

        self.entities.push(entity);
        self.len += 1;
        row_id
    }

    pub(crate) unsafe fn insert_batch<T: TupleTypesExt>(
        &mut self,
        entities: &[EntityKey],
        component_infos: &[ComponentInfo],
        soa_comp_ids: &[ComponentId],
        aos_comp_ids: &[ComponentId],
        cache: &mut EntityStorageCache,
        mut values: Vec<T>,
    ) -> Option<(u32, u32)> {
        if values.len() == 0 {
            return None;
        }

        let mut soa_ptr_vec = cache.ptr_vec_cache.take_cached();
        let mut aos_ptr_vec = cache.ptr_vec_cache.take_cached();

        values[0].self_get_value_ptrs_by_storage(&mut soa_ptr_vec, &mut aos_ptr_vec);
        let value_layout = Layout::new::<T>();

        self.entities.extend(entities.iter());
        self.table_soa.insert_batch(
            component_infos,
            &soa_comp_ids,
            &soa_ptr_vec,
            value_layout,
            values.len(),
        );
        self.table_aos.insert_batch(
            component_infos,
            &aos_comp_ids,
            &aos_ptr_vec,
            value_layout,
            values.len(),
            cache,
        );

        cache.ptr_vec_cache.insert(soa_ptr_vec);
        cache.ptr_vec_cache.insert(aos_ptr_vec);

        let row_id_start = self.len as u32;
        let row_id_end = row_id_start + values.len() as u32;

        let value_len: u32 = values.len().try_into().expect("Max u32 value reached!");
        println!(
            "entities len: {}, table len {}; batch values to add len: {}",
            self.entities.len(),
            self.len,
            value_len
        );
        self.len += value_len;

        while let Some(val) = values.pop() {
            std::mem::forget(val);
        }

        Some((row_id_start, row_id_end))
    }

    pub(crate) fn remove_entity(&mut self, entity: Entity) -> Option<(EntityKey, u32)> {
        assert!(self.len > entity.row_id);
        let key = self.entities.remove(entity.row_id as usize);
        self.table_soa.remove(&entity);
        self.table_aos.remove(&entity);

        self.len -= 1;

        // update moved entities table position
        if entity.row_id == self.len {
            None
        } else {
            Some((key, entity.row_id))
        }
    }

    pub unsafe fn tuple_iter<'a, TC: TupleIterConstructor<TableStorage>>(
        &'a mut self,
    ) -> TableStorageTupleIter<TC::Construct<'a>> {
        new_table_storage_iter::<TC>(self)
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
            let next = unsafe { Some(self.tuple_iters.next(self.index)) };
            self.index += 1;
            next
        } else {
            None
        }
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
