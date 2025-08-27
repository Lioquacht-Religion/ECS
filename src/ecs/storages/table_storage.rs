// table_storage.rs

use std::alloc::Layout;

use crate::{
    ecs::{
        component::{ArchetypeId, ComponentId, ComponentInfo},
        entity::EntityKey,
    },
    utils::{
        tuple_iters::{self, TableStorageTupleIter, TupleIterConstructor},
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

        let value_len : u32 = values.len()
            .try_into()
            .expect("Max u32 value reached!");
        println!("entities len: {}, table len {}; batch values to add len: {}", self.entities.len(), self.len, value_len);
        self.len += value_len;
        //self.len += 1;

        while let Some(val) = values.pop() {
            std::mem::forget(val);
        }

        Some((row_id_start, row_id_end))
    }

    pub unsafe fn tuple_iter<'a, TC: TupleIterConstructor<TableStorage>>(
        &'a mut self,
    ) -> TableStorageTupleIter<TC::Construct<'a>> {
        tuple_iters::new_table_storage_iter::<TC>(self)
    }
}
