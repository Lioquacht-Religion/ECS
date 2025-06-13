// table_soa.rs

use std::{any::TypeId, collections::HashMap};

use crate::ecs::{
    component::{ArchetypeId, Component, EntityStorage}, Entity
};

use super::thin_blob_vec::ThinBlobVec;

pub struct TableSoA {
    archetype_id: ArchetypeId,
    entities: Vec<Entity>,
    columns: HashMap<TypeId, ThinBlobVec>,
    cap: usize,
    free_indexes: Vec<usize>,
}

impl TableSoA {
    pub fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        let mut columns = HashMap::new();
        let archetype = &entity_storage.archetypes[usize::from(archetype_id)];
        archetype.comp_info_ids.iter().for_each(|cid| {
            let cinfo = &entity_storage.components[usize::from(*cid)];
            columns.insert(cinfo.type_id, ThinBlobVec::new(cinfo.layout));
        });

        Self {
            archetype_id,
            entities: Vec::new(),
            columns,
            cap: 0,
            free_indexes: Vec::new(),
        }
    }

    pub fn insert<T1: Component, T2: 'static, T3: 'static>(&mut self, input: (T1, T2, T3)) {
        let (T1, T2, T3) = input;
        /*
        SAFETY:
         Type safety is ensured by comparing of TypeId's.
        */
        unsafe {
            self.columns
                .get_mut(&TypeId::of::<T1>())
                .expect("Type T1 is not stored inside this table!")
                .push_typed(self.cap, self.entities.len(), T1);
            self.columns
                .get_mut(&TypeId::of::<T2>())
                .expect("Type T2 is not stored inside this table!")
                .push_typed(self.cap, self.entities.len(), T2);
            self.columns
                .get_mut(&TypeId::of::<T3>())
                .expect("Type T3 is not stored inside this table!")
                .push_typed(self.cap, self.entities.len(), T3);
        }
    }

    pub fn remove() {}
}

trait TableSoaAddable : 'static{
    type Input : TableSoaAddable;
    fn insert(table_soa: &mut TableSoA, input: Self::Input);
}

impl<T: Component> TableSoaAddable for T{
    type Input = T;
    fn insert(table_soa: &mut TableSoA, input: Self::Input) {
         unsafe {
            table_soa.columns
                .get_mut(&TypeId::of::<T>())
                .expect("Type T1 is not stored inside this table!")
                .push_typed(table_soa.cap, table_soa.entities.len(), input);
             }
    }
}

impl<T1: TableSoaAddable<Input = T1>, T2: TableSoaAddable<Input = T2>, T3: TableSoaAddable<Input = T3>> TableSoaAddable for (T1, T2, T3){
    type Input = (T1, T2, T3);
    fn insert(table_soa: &mut TableSoA, input: Self::Input) {
        let (T1, T2, T3) = input;
        /*
        SAFETY:
         Type safety is ensured by comparing of TypeId's.
        */
        T1::insert(table_soa, T1);
        T2::insert(table_soa, T2);
        T3::insert(table_soa, T3);
    }


}
