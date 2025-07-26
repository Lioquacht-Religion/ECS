// table_storage.rs

use crate::{
    ecs::component::{ArchetypeId, ComponentId, ComponentInfo, EntityKey},
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
    pub(crate) table_soa: TableSoA,
    pub(crate) table_aos: TableAoS,
    pub(crate) len: u32,
}

impl TableStorage {
    pub(crate) fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        Self {
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
            .insert(entity, component_infos, &soa_comp_ids, &soa_ptr_vec);
        self.table_aos
            .insert(entity, component_infos, &aos_comp_ids, &aos_ptr_vec);
        std::mem::forget(value);

        cache.ptr_vec_cache.insert(soa_ptr_vec);
        cache.ptr_vec_cache.insert(aos_ptr_vec);

        self.len += 1;
        row_id
    }

    pub unsafe fn tuple_iter<'a, TC: TupleIterConstructor<TableStorage>>(
        &'a mut self,
    ) -> TableStorageTupleIter<TC::Construct<'a>> {
        tuple_iters::new_table_storage_iter::<TC>(self)
    }
}
