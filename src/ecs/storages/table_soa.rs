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
        archetype.comp_ids.get_vec().iter().for_each(|cid| {
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

    pub fn insert<T: TableSoaAddable<Input = T>>(&mut self, input: T) {
        T::insert(self, input);
    }

    pub fn remove() {}
}

pub trait TableSoaAddable : 'static{
    type Input : TableSoaAddable;
    fn insert(table_soa: &mut TableSoA, input: Self::Input);
}

impl<T: Component> TableSoaAddable for T{
    type Input = T;
    fn insert(table_soa: &mut TableSoA, input: Self::Input) {
        /*
        SAFETY:
         Type safety is ensured by comparing of TypeId's.
        */
         unsafe {
            table_soa.columns
                .get_mut(&TypeId::of::<T>())
                .expect("Type T is not stored inside this table!")
                .push_typed(table_soa.cap, table_soa.entities.len(), input);
             }
    }
}

impl<T1: TableSoaAddable<Input = T1>, T2: TableSoaAddable<Input = T2>, T3: TableSoaAddable<Input = T3>> TableSoaAddable for (T1, T2, T3){
    type Input = (T1, T2, T3);
    fn insert(table_soa: &mut TableSoA, input: Self::Input) {
        let (T1, T2, T3) = input;
        T1::insert(table_soa, T1);
        T2::insert(table_soa, T2);
        T3::insert(table_soa, T3);
    }
}

#[cfg(test)]
mod tests {
    use crate::ecs::component::Component;
    use crate::ecs::{component::EntityStorage};
    use crate::utils::tuple_types::TupleTypesExt;

    use super::TableSoA;


    struct Pos(i32);
    impl Component for Pos{}

    struct Pos2(i32, i32);
    impl Component for Pos2{}

    struct Pos3(i32, i32, i32);
    impl Component for Pos3{}


    #[test]
    fn test_table_soa() {
        let mut es = EntityStorage::new();
        let archetype_id = es.create_or_get_archetype::<(Pos, Pos3)>();
        let mut table_soa = TableSoA::new(archetype_id, &es);
        table_soa.insert((Pos(12), Pos2(12, 34), Pos3(12, 34, 56)));
    }
}
 
