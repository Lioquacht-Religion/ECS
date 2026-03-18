// table_soa.rs

use std::{
    alloc::Layout,
    any::{TypeId, type_name},
    ptr::NonNull,
};

use crate::{
    ecs::{
        component::{ArchetypeId, Component, ComponentId, ComponentInfo, Map},
        entity::{Entity, TableRowId},
    },
    utils::{ecs_id::EcsId, tuple_iters::TupleIterator},
};

use super::{
    entity_storage::EntityStorage,
    thin_blob_vec::{ThinBlobIterMutUnsafe, ThinBlobIterUnsafe, ThinBlobVec},
};

#[derive(Debug)]
pub(crate) struct TableSoA {
    #[allow(unused)]
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) columns: Map<TypeId, ThinBlobVec>,
    pub(crate) len: usize,
    pub(crate) cap: usize,
}

impl TableSoA {
    pub(crate) fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
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

    /// #SAFETY:
    ///
    /// ##Input values:
    /// Supplied ComponentInfo, ComponentId and component pointer slice
    /// should be of the same length and component order.
    /// The types and layouts of the inserted components should exactly match
    /// the types of the already inserted components.
    ///
    /// The order in which the components are stored in the table
    /// and in which they are supplied do not need to match.
    ///
    /// ##After call needed actions
    /// The caller of this function
    /// should forget/leak value of type T,
    /// so that the by this value owned allocations will not be dropped
    /// once it goes out of scope.
    pub(crate) unsafe fn insert(
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
            unsafe {
                self.columns
                    .get_mut(&cinfo.type_id)
                    .expect("Type T is not stored inside this table!")
                    .push_untyped(self.cap, self.len, soa_ptrs[i]);
            }
        }

        self.update_capacity();
        self.len += 1;
    }

    /// #SAFETY:
    /// ##Input values:
    /// Supplied ComponentInfo, ComponentId and component pointer slice
    /// should be of the same length and component order.
    /// The types and layouts of the inserted components should exactly match
    /// the types of the already inserted components.
    ///
    /// The order in which the components are stored in the table
    /// and in which they are supplied do not need to match.
    ///
    /// ##After call needed actions:
    /// Caller of this function
    /// should forget/leak batch insterted values of type T,
    /// so by these values owned allocations will not be dropped
    /// once they go out of scope.
    pub(crate) unsafe fn insert_batch(
        &mut self,
        component_infos: &[ComponentInfo],
        soa_comp_ids: &[ComponentId],
        soa_base_ptrs: &[NonNull<u8>],
        value_layout: Layout,
        batch_len: usize,
    ) {
        // no components marked for SOA storage,
        // return early and do no work
        if soa_comp_ids.len() == 0 {
            return;
        }

        //TODO: why did i do this, document
        let mut thin_columns: Vec<NonNull<ThinBlobVec>> = Vec::with_capacity(soa_comp_ids.len());
        for cid in soa_comp_ids.iter() {
            let cinfo: &ComponentInfo = &component_infos[cid.0 as usize];
            thin_columns.push(
                self.columns
                    .get_mut(&cinfo.type_id)
                    .expect("Type T is not stored inside this table!")
                    .into(),
            );
        }

        for i in 0..batch_len {
            for j in 0..soa_comp_ids.len() {
                unsafe {
                    let column: &mut ThinBlobVec = thin_columns[j].as_mut();
                    let value_offset = value_layout.size() * i;
                    let entry_ptr = soa_base_ptrs[j].add(value_offset);
                    column.push_untyped(self.cap, self.len, entry_ptr);
                }
            }
            self.update_capacity();
            self.len += 1;
        }
    }

    //TODO: add safety comments
    pub(crate) fn transfer_entity_with_new_comp<T: Component>(
        from: &mut TableSoA,
        to: &mut TableSoA,
        entity: &Entity,
        component: T,
    ) -> TableRowId {
        let from_iter = from
            .columns
            .iter_mut()
            .filter(|(tid, _)| **tid != TypeId::of::<T>());
        //SAFETY: from_len and the from_iter are from the same table, so from_len and the single
        //columns should have the same length
        unsafe {
            Self::transfer_entity(from.len, from_iter, to, entity);
        }
        let to_col_added_comp = to.columns.get_mut(&TypeId::of::<T>())
            .expect("TableSoA to move entity to does not contain needed column to add the new component to.");
        unsafe {
            to_col_added_comp.push_typed(to.cap, to.len, component);
        }
        from.len -= 1;
        let new_to_table_entity_row_id = to.len.into();
        to.update_capacity();
        to.len += 1;
        new_to_table_entity_row_id
    }

    pub(crate) fn remove_comp_and_transfer_entity<T: Component>(
        from: &mut TableSoA,
        to: &mut TableSoA,
        entity: &Entity,
    ) -> TableRowId {
        // remove and drop component from from_table
        let column_drop_comp_from = from
            .columns
            .get_mut(&TypeId::of::<T>())
            .expect("Component to be removed from entity is not contained in AoS table.");
        //SAFETY: table contains a columns for the to be removed component
        unsafe {
            column_drop_comp_from.drop_and_replace_with_last(from.len, entity.row_id.id_usize());
        }
        let from_iter = from
            .columns
            .iter_mut()
            .filter(|(tid, _)| **tid != TypeId::of::<T>());
        //SAFETY: from_len and the from_iter are from the same table, so from_len and the single
        //columns should have the same length
        unsafe {
            Self::transfer_entity(from.len, from_iter, to, entity);
        }
        from.len -= 1;
        let new_to_table_entity_row_id = to.len.into();
        to.update_capacity();
        to.len += 1;
        new_to_table_entity_row_id
    }

    unsafe fn transfer_entity<'to>(
        from_len: usize,
        from_iter: impl Iterator<Item = (&'to TypeId, &'to mut ThinBlobVec)>,
        to: &mut TableSoA,
        entity: &Entity,
    ) {
        for (type_id, from_col) in from_iter {
            let to_col = to
                .columns
                .get_mut(type_id)
                .expect("TableSoA to move entity to does not contain needed component column.");
            let from_row_id = entity.row_id.id_usize();
            let from_col_elem_ptr = from_col.get_ptr_untyped(from_row_id, from_col.elem_layout);
            //SAFETY: from_len needs to be equal to the length of the columns in the from_iter
            unsafe {
                to_col.push_untyped(to.cap, to.len, from_col_elem_ptr);
                from_col.dont_drop_replace_with_last(from_len, from_row_id);
            }
        }
    }

    fn update_capacity(&mut self) {
        if self.cap == 0 {
            self.cap = 4;
        } else if self.len >= self.cap {
            self.cap *= 2;
        }
    }

    pub(crate) fn remove(&mut self, entity: &Entity) {
        if self.len > entity.row_id.id_usize() {
            for (_tid, col) in self.columns.iter_mut() {
                unsafe {
                    col.drop_and_replace_with_last(self.len, entity.row_id.id_usize());
                }
            }
            self.len -= 1;
        }
    }

    /*
    #[allow(unused)]
    pub(crate) unsafe fn tuple_iter<'a, TC: TupleIterConstructor<TableSoA>>(
        &'a mut self,
    ) -> TableSoaTupleIter<TC::Construct<'a>> {
        unsafe { new_table_soa_iter::<TC>(self) }
    }
    */

    /// #SAFETY:
    /// Component type T needs to be contained by the table,
    /// otherwise this function will panic.
    pub(crate) unsafe fn get_single_comp_iter<'c, T: Component>(
        &'c self,
    ) -> ThinBlobIterUnsafe<'c, T> {
        unsafe {
            self.columns
                .get(&TypeId::of::<T>())
                .expect(&format!(
                    "No column with type id for type: {}.",
                    type_name::<T>()
                ))
                .tuple_iter()
        }
    }

    pub(crate) unsafe fn get_single_comp_iter_opt<'c, T: Component>(
        &'c self,
    ) -> Option<ThinBlobIterUnsafe<'c, T>> {
        unsafe {
            if let Some(col) = self.columns.get(&TypeId::of::<T>()) {
                Some(col.tuple_iter())
            } else {
                None
            }
        }
    }

    /// #SAFETY:
    /// Component type T needs to be contained by the table,
    /// otherwise this function will panic.
    pub(crate) unsafe fn get_single_comp_iter_mut<'c, T: Component>(
        &'c mut self,
    ) -> ThinBlobIterMutUnsafe<'c, T> {
        unsafe {
            self.columns
                .get_mut(&TypeId::of::<T>())
                .expect(&format!(
                    "No column with type id for type: {}.",
                    type_name::<T>()
                ))
                .tuple_iter_mut()
        }
    }

    pub(crate) unsafe fn get_single_comp_iter_mut_opt<'c, T: Component>(
        &'c mut self,
    ) -> Option<ThinBlobIterMutUnsafe<'c, T>> {
        unsafe {
            if let Some(col) = self.columns.get_mut(&TypeId::of::<T>()) {
                Some(col.tuple_iter_mut())
            } else {
                None
            }
        }
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

#[allow(unused)]
pub(crate) struct TableSoaTupleIter<T: TupleIterator> {
    tuple_iters: T,
    len: usize,
    index: usize,
}

/*
#[allow(unused)]
pub(crate) unsafe fn new_table_soa_iter<'table, TC: TupleIterConstructor<TableSoA>>(
    table: &'table mut TableSoA,
) -> TableSoaTupleIter<TC::Construct<'table>> {
    unsafe {
        TableSoaTupleIter {
            tuple_iters: TC::construct(table.into()),
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
        //TODO:?
        todo!()
    }
    unsafe fn get_iter<'c, T: Component>(&'c mut self) -> Self::IterType<'c, T> {
        unsafe {
            self.columns
                .get(&TypeId::of::<T>())
                .expect("ERROR: TableSoA does not contain a column with this type id")
                .tuple_iter()
        }
    }
    unsafe fn get_iter_mut<'c, T: Component>(&'c mut self) -> Self::IterMutType<'c, T> {
        unsafe {
            self.columns
                .get_mut(&TypeId::of::<T>())
                .expect("ERROR: TableSoA does not contain a column with this type id")
                .tuple_iter_mut()
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use crate::ecs::component::StorageTypes;
    use crate::ecs::query::Query;
    use crate::ecs::world::WorldData;
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

            println!("pos4.1 box pointer: {}", pos4.1.0);
            pos4.1.0 = 23234;
            pos4.1.0 -= 2344;
            println!("pos4.1 box pointer: {}", pos4.1.0);
        }

        assert_eq!(query2.iter().count(), 4);
    }

    fn init_es_insert(es: &mut WorldData) {
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
    fn test_table_soa_query_iter() {
        let mut world = World::new();
        let num1: i32 = 2324;
        let num2: usize = 2324;
        world.add_systems(test_system1);
        world.add_resource(num1);
        world.add_resource(num2);

        init_es_insert(world.data.get_mut());

        world.init_and_run();
    }
}
