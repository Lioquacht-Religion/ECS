// table_soa.rs

use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use crate::{
    all_tuples,
    ecs::component::{ArchetypeId, Component, EntityKey, EntityStorage}, utils::tuple_iters::{self, TableSoaTupleIter, TupleIterConstructor, TupleIterator},
};

use super::thin_blob_vec::ThinBlobVec;

//TODO: entities need to be stored too for querying
pub struct TableSoA {
    pub(crate) archetype_id: ArchetypeId,
    entities: Vec<EntityKey>,
    pub(crate) columns: HashMap<TypeId, ThinBlobVec>,
    pub(crate) len: usize,
    pub(crate) cap: usize,
    free_indices: Vec<usize>,
}

impl TableSoA {
    pub fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        let mut columns = HashMap::new();
        let archetype = &entity_storage.archetypes[usize::from(archetype_id)];
        archetype.comp_ids.get_vec().iter().for_each(|cid| {
            let cinfo = &entity_storage.components[usize::from(*cid)];
            columns.insert(cinfo.type_id, ThinBlobVec::new(cinfo.layout, cinfo.drop));
        });

        Self {
            archetype_id,
            entities: Vec::new(),
            columns,
            len: 0,
            cap: 0,
            free_indices: Vec::new(),
        }
    }

    /**
     * Returns the row_id of the inserted input.
     */
    pub fn insert<T: TableSoaAddable<Input = T>>(
        &mut self,
        entity_key: EntityKey,
        input: T,
    ) -> u32 {
        //TODO: use free indices
        let row_id = self.len;
        T::insert(self, input);
        self.update_capacity();
        self.len += 1;
        self.entities.push(entity_key);
        row_id
            .try_into()
            .expect("ERROR max u32 row id space reached!")
    }

    fn update_capacity(&mut self) {
        if self.cap == 0 {
            self.cap = 4;
        }
        else if self.len >= self.cap {
            self.cap *= 2;
        }
    }

    pub fn remove() {}

    pub fn tuple_iter<'a, T: TupleIterator, TC: TupleIterConstructor<Construct<'a> = T>>(&'a mut self) -> TableSoaTupleIter<T>{
        tuple_iters::new::<T, TC>(self)
    }

    pub fn tuple_iter2<T: TupleIterator>(&mut self) -> T{
        unimplemented!()
    }
}

pub trait TableSoaAddable: 'static {
    type Input: TableSoaAddable;
    fn insert(table_soa: &mut TableSoA, input: Self::Input);
}

impl<T: Component> TableSoaAddable for T {
    type Input = T;
    fn insert(table_soa: &mut TableSoA, input: Self::Input) {
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

impl Drop for TableSoA{
    fn drop(&mut self) {
        unsafe {
        self.columns.iter_mut()
            .for_each(|(_k, c)| {c.dealloc(self.cap, self.len);});
        }
    }
}

macro_rules! impl_soa_addable_ext {
    ($($t:ident), *) => {
       impl<$($t : TableSoaAddable<Input = $t>), *> TableSoaAddable for ($($t),*,){
            type Input = ($($t,)*);
            fn insert(table_soa: &mut TableSoA, input: Self::Input) {
                #[allow(non_snake_case)]
                let ($($t,)*) = input;
                $($t::insert(table_soa, $t);)*
            }
       }
    }
}

#[rustfmt::skip]
all_tuples!(
    impl_soa_addable_ext,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

#[cfg(test)]
mod tests {
    use crate::ecs::component::ArchetypeId;
    use crate::ecs::component::Component;
    use crate::ecs::component::EntityStorage;

    use super::TableSoA;

    struct Pos(i32);
    impl Component for Pos {}

    struct Pos2(i32, i32);
    impl Component for Pos2 {}

    struct Pos3(i32, i32, i32);
    impl Component for Pos3 {}

    struct Pos4(i32, Box<Pos3>);
    impl Component for Pos4 {}

    #[test]
    fn test_table_soa() {
        let mut es = EntityStorage::new();
        es.add_entity((Pos(12), Pos3(12, 34, 56)));
        es.add_entity((Pos3(12, 12, 34), Pos(56)));

        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos2(213, 23)));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));

        let table = TableSoA::new(ArchetypeId(0), &es);
 
        /*
        table_soa.insert(
            (
                Pos2(12, 34), Pos3(12, 34, 56),
                (Pos2(12, 34), Pos3(12, 34, 56))
            )
        );
        table_soa.insert((Pos(12), Pos2(12, 34), Pos3(12, 34, 56)));
        */
    }
}
