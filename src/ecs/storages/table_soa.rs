// table_soa.rs

use std::{
    alloc::Layout,
    any::{type_name, TypeId},
    ptr::NonNull,
};

use crate::{
    ecs::{
        component::{ArchetypeId, Component, ComponentId, ComponentInfo, Map},
        entity::{Entity, EntityKeyIterUnsafe},
    },
    utils::tuple_iters::{TupleConstructorSource, TupleIterConstructor, TupleIterator},
};

use super::{
    entity_storage::EntityStorage,
    thin_blob_vec::{ThinBlobIterMutUnsafe, ThinBlobIterUnsafe, ThinBlobVec},
};

//TODO: entities need to be stored too for querying
pub struct TableSoA {
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) columns: Map<TypeId, ThinBlobVec>,
    pub(crate) len: usize,
    pub(crate) cap: usize,
}

//TODO: make probably every function here unsafe
impl TableSoA {
    pub fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        let mut columns = Map::new();
        let archetype = &entity_storage.archetypes[usize::from(archetype_id)];
        archetype.soa_comp_ids.get_vec().iter().for_each(|cid| {
            let cinfo = &entity_storage.components[usize::from(*cid)];
            columns.insert(cinfo.type_id, ThinBlobVec::new(cinfo.layout, cinfo.drop));
        });

        Self {
            archetype_id,
            columns,
            len: 0,
            cap: 0,
        }
    }

    ///SAFETY: Caller of this function
    ///        should forget/leak value of type T,
    ///        so it does not get dropped.
    pub unsafe fn insert(
        &mut self,
        component_infos: &[ComponentInfo],
        soa_comp_ids: &[ComponentId],
        soa_ptrs: &[NonNull<u8>],
    ) {
        if soa_comp_ids.len() <= 0 {
            return;
        }

        for (i, cid) in soa_comp_ids.iter().enumerate() {
            let cinfo = &component_infos[cid.0 as usize];
            self.columns
                .get_mut(&cinfo.type_id)
                .expect("Type T is not stored inside this table!")
                .push_untyped(self.cap, self.len, soa_ptrs[i]);
        }

        self.update_capacity();
        self.len += 1;
    }

    pub unsafe fn insert_batch(
        &mut self,
        component_infos: &[ComponentInfo],
        soa_comp_ids: &[ComponentId],
        soa_base_ptrs: &[NonNull<u8>],
        value_layout: Layout,
        batch_len: usize,
    ) {
        if soa_comp_ids.len() <= 0 {
            return;
        }

        let mut thin_columns: Vec<NonNull<ThinBlobVec>> = Vec::with_capacity(soa_comp_ids.len());
        for cid in soa_comp_ids.iter() {
            let cinfo = &component_infos[cid.0 as usize];
            thin_columns.push(
                self.columns
                    .get_mut(&cinfo.type_id)
                    .expect("Type T is not stored inside this table!")
                    .into(),
            );
        }

        for i in 0..batch_len {
            for j in 0..soa_comp_ids.len() {
                let column = thin_columns[j].as_mut();
                let value_offset = value_layout.size() * i;
                let entry_ptr = soa_base_ptrs[j].add(value_offset);
                column.push_untyped(self.cap, self.len, entry_ptr);
            }
            self.update_capacity();
            self.len += 1;
        }
    }

    fn update_capacity(&mut self) {
        if self.cap == 0 {
            self.cap = 4;
        } else if self.len >= self.cap {
            self.cap *= 2;
        }
    }

    pub fn remove(&mut self, entity: &Entity) {
        if self.len > entity.row_id as usize {
            for (_tid, col) in self.columns.iter_mut() {
                unsafe {
                    col.remove_and_replace_with_last(self.len, entity.row_id as usize);
                }
            }
            self.len -= 1;
        }
    }

    pub unsafe fn tuple_iter<'a, TC: TupleIterConstructor<TableSoA>>(
        &'a mut self,
    ) -> TableSoaTupleIter<TC::Construct<'a>> {
        new_table_soa_iter::<TC>(self)
    }

    pub unsafe fn get_single_comp_iter<'c, T: Component>(&'c self) -> ThinBlobIterUnsafe<'c, T> {
        self.columns
            .get(&TypeId::of::<T>())
            .expect(&format!(
                "No column with type id for type: {}.",
                type_name::<T>()
            ))
            .tuple_iter()
    }
    pub unsafe fn get_single_comp_iter_mut<'c, T: Component>(
        &'c mut self,
    ) -> ThinBlobIterMutUnsafe<'c, T> {
        self.columns
            .get_mut(&TypeId::of::<T>())
            .expect(&format!(
                "No column with type id for type: {}.",
                type_name::<T>()
            ))
            .tuple_iter_mut()
    }
}

impl Drop for TableSoA {
    fn drop(&mut self) {
        unsafe {
            self.columns.iter_mut().for_each(|(_k, c)| {
                c.dealloc(self.cap, self.len);
            });
        }
    }
}

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

#[cfg(test)]
mod tests {
    use crate::ecs::component::StorageTypes;
    use crate::ecs::query::Query;
    use crate::ecs::storages::entity_storage::EntityStorage;
    use crate::ecs::{component::Component, system::Res, world::World};

    struct Pos(i32);
    impl Component for Pos {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableSoA;
    }

    struct Pos2(i32, i64);
    impl Component for Pos2 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableSoA;
    }

    struct Pos3(i32, i32, i32);
    impl Component for Pos3 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableSoA;
    }

    struct Pos4(i32, Box<Pos3>);
    impl Component for Pos4 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableSoA;
    }

    struct Comp1(usize, usize);
    impl Component for Comp1 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableSoA;
    }

    struct Comp2(usize, usize);
    impl Component for Comp2 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableSoA;
    }

    fn test_system1(
        prm: Res<i32>,
        prm2: Res<usize>,
        mut query: Query<(&Comp1, &mut Comp2)>,
        mut query2: Query<(&Pos, &mut Pos4, &Pos3)>,
    ) {
        println!("testsystem1 res: {}, {}", prm.value, prm2.value);

        let mut count = 0;
        for (comp1, comp2) in query.iter() {
            println!("comp1: {}", comp1.0);
            println!("comp2: {}", comp2.0);
            comp2.0 = 2;
            println!("comp2: {}", comp2.0);
            count += 1;
        }

        assert_eq!(count, 3);
        assert_eq!(query.iter().count(), 3);

        for (_pos, pos4, _pos3) in query2.iter() {
            println!("pos4 : {}", pos4.0);
            pos4.0 = 23234;
            pos4.0 -= 2344;
            println!("pos4 : {}", pos4.0);

            println!("pos4.1 box pointer: {}", pos4.1 .0);
            pos4.1 .0 = 23234;
            pos4.1 .0 -= 2344;
            println!("pos4.1 box pointer: {}", pos4.1 .0);
        }

        assert_eq!(query2.iter().count(), 4);
    }

    fn init_es_insert(es: &mut EntityStorage) {
        es.add_entity((Comp1(12, 34), Comp2(12, 34)));
        es.add_entity((Comp1(12, 34), Comp2(12, 34)));
        es.add_entity((Comp2(12, 34), Comp1(12, 34)));

        es.add_entity((Pos(12), Pos3(12, 34, 56)));
        es.add_entity((Pos3(12, 12, 34), Pos(56)));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos2(213, 23)));
        es.add_entity((Pos2(213, 23), Pos(12)));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
    }

    #[test]
    fn test_table_soa_insert() {
        let mut es = EntityStorage::new();
        init_es_insert(&mut es);
    }

    #[test]
    fn test_table_soa_query_iter() {
        let mut world = World::new();
        let num1: i32 = 2324;
        let num2: usize = 2324;
        world.add_system(test_system1);
        unsafe { (&mut *world.data.get()).add_resource(num1) };
        unsafe { (&mut *world.data.get()).add_resource(num2) };

        let es = &mut world.data.get_mut().entity_storage;

        init_es_insert(es);

        world.run();
    }
}
