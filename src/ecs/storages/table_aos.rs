//table_aos.rs

use std::{
    alloc::Layout,
    any::{TypeId, type_name},
    ptr::NonNull,
};

use crate::{
    ecs::{
        component::{ArchetypeId, Component, ComponentId, ComponentInfo, Map},
        entity::{Entity, EntityKeyIterUnsafe},
    },
    utils::{
        sorted_vec::SortedVec,
        tuple_iters::{TupleConstructorSource, TupleIterConstructor, TupleIterator},
        tuple_types::TupleTypesExt,
    },
};

use super::{
    cache::EntityStorageCache,
    entity_storage::EntityStorage,
    thin_blob_vec::{
        CompElemPtr, ThinBlobInnerTypeIterMutUnsafe, ThinBlobInnerTypeIterUnsafe, ThinBlobVec,
    },
};

#[derive(Debug, Hash, Eq)]
pub(crate) struct TypeMetaData {
    pub(crate) comp_id: ComponentId,
    pub(crate) ptr_offset: usize,
    pub(crate) drop_fn: Option<unsafe fn(*mut u8)>,
}

impl PartialEq for TypeMetaData {
    fn eq(&self, other: &Self) -> bool {
        self.comp_id.eq(&other.comp_id)
    }
}

impl PartialOrd for TypeMetaData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.comp_id.partial_cmp(&other.comp_id)
    }
}

impl Ord for TypeMetaData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.comp_id.cmp(&other.comp_id)
    }
}

pub struct TableAoS {
    #[allow(unused)]
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) vec: ThinBlobVec,
    pub(crate) cap: usize,
    pub(crate) len: usize,
    pub(crate) type_meta_data_map: Map<TypeId, usize>,
    pub(crate) type_meta_data: SortedVec<TypeMetaData>,
}

impl TableAoS {
    pub fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        let archetype = &entity_storage.archetypes[usize::from(archetype_id)];

        let comp_ids: &[ComponentId] = &archetype.aos_comp_ids.get_vec();
        let mut meta_data: Vec<TypeMetaData> = Vec::with_capacity(comp_ids.len());

        let mut comp_id_iter = comp_ids.iter();
        let mut type_meta_data_map: Map<TypeId, usize> = Map::with_capacity(comp_ids.len());

        if let Some(comp_id) = comp_id_iter.next() {
            let comp_info = &entity_storage.components[comp_id.0 as usize];
            let mut row_layout = comp_info.layout.pad_to_align();
            meta_data.push(TypeMetaData {
                comp_id: *comp_id,
                ptr_offset: 0,
                drop_fn: comp_info.drop,
            });

            // comp_ids vec from entity storage should be ordered
            // so index should line up with type_meta_data order
            let mut index = 0;
            type_meta_data_map.insert(comp_info.type_id, index);

            while let Some(comp_id) = comp_id_iter.next() {
                index += 1;
                let comp_info = &entity_storage.components[comp_id.0 as usize];
                let (new_row_layout, offset) = row_layout
                    .extend(comp_info.layout)
                    .expect("Allocation error in table AoS!");
                row_layout = new_row_layout.pad_to_align();
                meta_data.push(TypeMetaData {
                    comp_id: *comp_id,
                    ptr_offset: offset,
                    drop_fn: comp_info.drop,
                });
                type_meta_data_map.insert(comp_info.type_id, index);
            }
            row_layout = row_layout.pad_to_align();
            return Self {
                archetype_id,
                vec: ThinBlobVec::new(row_layout, None),
                cap: 0,
                len: 0,
                type_meta_data_map,
                type_meta_data: meta_data.into(),
            };
        }
        //TODO: rethink how to handle this situation, probably put table into option outside
        //panic!("Completely empty Archetype not allowed!")
        Self {
            archetype_id,
            vec: ThinBlobVec::new(Layout::new::<()>(), None),
            cap: 0,
            len: 0,
            type_meta_data_map: Map::new(),
            type_meta_data: SortedVec::new(),
        }
    }

    #[allow(unused)]
    pub(crate) fn print_internals(&self, component_infos: &[ComponentInfo]) {
        println!("TableAoS: ");
        println!("layout: {:?}", self.vec.elem_layout);
        for (i, tm) in self.type_meta_data.iter().enumerate() {
            println!(
                "i: {}; tm: {:?}, cinfo: {:?}",
                i, tm, component_infos[tm.comp_id.0 as usize]
            );
        }
    }

    ///SAFETY: Caller of this function
    ///        should forget/leak value of type T,
    ///        so it does not get dropped.
    pub(crate) unsafe fn insert(
        &mut self,
        component_infos: &[ComponentInfo],
        aos_comp_ids: &[ComponentId],
        aos_ptrs: &[NonNull<u8>],
        cache: &mut EntityStorageCache,
    ) {
        if aos_comp_ids.len() == 0 {
            return;
        }

        let mut comp_elem_ptrs: Vec<CompElemPtr> = cache.compelemptr_vec_cache.take_cached();

        for i in 0..aos_comp_ids.len() {
            comp_elem_ptrs.push(CompElemPtr {
                comp_id: *&aos_comp_ids[i],
                ptr: *&aos_ptrs[i],
            });
        }

        let comp_elem_ptrs: SortedVec<CompElemPtr> = comp_elem_ptrs.into();

        unsafe {
            self.vec.push_ptr_vec_untyped(
                &mut self.cap,
                &mut self.len,
                component_infos,
                &self.type_meta_data.get_vec(),
                &comp_elem_ptrs.get_vec(),
            );
        }

        cache.compelemptr_vec_cache.insert(comp_elem_ptrs.into());
    }

    pub(crate) unsafe fn insert_batch(
        &mut self,
        component_infos: &[ComponentInfo],
        aos_comp_ids: &[ComponentId],
        aos_base_ptrs: &[NonNull<u8>],
        value_layout: Layout,
        batch_len: usize,
        cache: &mut EntityStorageCache,
    ) {
        if aos_comp_ids.len() == 0 {
            return;
        }

        let mut comp_elem_ptrs: Vec<CompElemPtr> = cache.compelemptr_vec_cache.take_cached();
        for i in 0..aos_comp_ids.len() {
            comp_elem_ptrs.push(CompElemPtr {
                comp_id: *&aos_comp_ids[i],
                ptr: *&aos_base_ptrs[i],
            });
        }
        let comp_elem_ptrs: SortedVec<CompElemPtr> = comp_elem_ptrs.into();

        for i in 0..batch_len {
            let offset = value_layout.size() * i;
            unsafe {
                self.vec.push_ptr_vec_untyped_with_offset(
                    &mut self.cap,
                    &mut self.len,
                    component_infos,
                    &self.type_meta_data.get_vec(),
                    &comp_elem_ptrs.get_vec(),
                    offset,
                );
            }
        }
        cache.compelemptr_vec_cache.insert(comp_elem_ptrs.into());
    }

    pub(crate) unsafe fn get_mut_by_index<T: TupleTypesExt>(
        &mut self,
        index: usize,
    ) -> Option<&mut T> {
        let _row_ptr = unsafe { self.vec.get_ptr_untyped(index, self.vec.elem_layout) };

        unimplemented!()
    }

    pub(crate) fn remove(&mut self, entity: &Entity) {
        if self.len > entity.row_id as usize {
            //TODO: drop inner packed components individually too
            unsafe {
                self.drop_entity_row(entity.row_id as usize);
                self.vec.remove_and_replace_with_last(self.len, entity.row_id as usize);
            }
            self.len -= 1;
        }
    }

    pub(crate) unsafe fn tuple_iter<'a, TC: TupleIterConstructor<TableAoS>>(
        &'a mut self,
    ) -> TableAosTupleIter<TC::Construct<'a>> {
        unsafe { new_table_aos_iter::<TC>(self) }
    }

    pub(crate) unsafe fn get_single_comp_iter<'c, T: Component>(
        &'c self,
    ) -> ThinBlobInnerTypeIterUnsafe<'c, T> {
        let index = self
            .type_meta_data_map
            .get(&TypeId::of::<T>())
            .expect(&format!(
                "No component id found for type id of type: {}.",
                type_name::<T>()
            ));

        let offset = &self.type_meta_data.get_vec()[*index].ptr_offset;
        unsafe { self.vec.tuple_inner_type_iter(*offset) }
    }

    pub(crate) unsafe fn get_single_comp_iter_mut<'c, T: Component>(
        &'c mut self,
    ) -> ThinBlobInnerTypeIterMutUnsafe<'c, T> {
        let index = self
            .type_meta_data_map
            .get(&TypeId::of::<T>())
            .expect(&format!(
                "No component id found for type id of type: {}.",
                type_name::<T>()
            ));
        let offset = &self.type_meta_data.get_vec()[*index].ptr_offset;
        unsafe { self.vec.tuple_inner_type_iter_mut(*offset) }
    }

    unsafe fn drop_entity_row(&mut self, index: usize){
        let base_ptr = self.vec.data_ptr;
        let row_size = self.vec.elem_layout.size();
        let row_ptr = unsafe{ base_ptr.add(row_size * index) };
        for meta_data in self.type_meta_data.iter() {
            if let Some(drop_fn) = meta_data.drop_fn {
                unsafe {
                    let elem_ptr = row_ptr.add(meta_data.ptr_offset);
                    drop_fn(elem_ptr.as_ptr());
                }
            }
        }
    }
}

impl Drop for TableAoS {
    fn drop(&mut self) {
        //TODO: what about removing already empty entry in thin_blob_vec?
        //TODO: remove free indexes concept everywhere
        // just move last entry to empty spot and updat entity vec with new position
        // generation entity index stored by other systems is not effected

        for meta_data in self.type_meta_data.iter() {
            if let Some(drop_fn) = meta_data.drop_fn {
                let base_ptr = self.vec.data_ptr;
                let row_size = self.vec.elem_layout.size();
                for i in 0..self.len {
                    unsafe {
                        let row_ptr = base_ptr.add(row_size * i);
                        let elem_ptr = row_ptr.add(meta_data.ptr_offset);
                        drop_fn(elem_ptr.as_ptr());
                    }
                }
            }
        }
        // deallocate allocation of ThinBlobVec owned memory range
        unsafe {
            self.vec.dealloc(self.cap, self.len);
        }
    }
}

pub(crate) struct TableAosTupleIter<T: TupleIterator> {
    tuple_iters: T,
    len: usize,
    index: usize,
}

pub unsafe fn new_table_aos_iter<'table, TC: TupleIterConstructor<TableAoS>>(
    table: &'table mut TableAoS,
) -> TableAosTupleIter<TC::Construct<'table>> {
    unsafe {
        TableAosTupleIter {
            tuple_iters: TC::construct(table.into()),
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
        unsafe { self.vec.tuple_inner_type_iter(offset) }
    }
    unsafe fn get_iter_mut<'c, T: Component>(&'c mut self) -> Self::IterMutType<'c, T> {
        let index = self
            .type_meta_data_map
            .get(&TypeId::of::<T>())
            .expect("ERROR: TableAoS does not contain a column with this type id");
        let offset = self.type_meta_data.get_vec()[*index].ptr_offset;
        unsafe { self.vec.tuple_inner_type_iter_mut(offset) }
    }
}

#[cfg(test)]
mod test {

    use crate::ecs::component::{ArchetypeId, StorageTypes};
    use crate::ecs::query::Query;
    use crate::ecs::storages::entity_storage::EntityStorage;
    use crate::ecs::{component::Component, system::Res, world::World};

    #[derive(Debug)]
    struct Pos(i32);
    impl Component for Pos {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableAoS;
    }

    #[derive(Debug)]
    struct Pos2(i32, i64);
    impl Component for Pos2 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableAoS;
    }

    #[derive(Debug)]
    struct Pos3(i32, i32, i32);
    impl Component for Pos3 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableAoS;
    }

    #[derive(Debug)]
    struct Pos4(i32, Box<Pos3>);
    impl Component for Pos4 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableAoS;
    }

    #[derive(Debug)]
    struct Comp1(u32, usize);
    impl Component for Comp1 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableAoS;
    }

    #[derive(Debug)]
    struct Comp2(usize, usize);
    impl Component for Comp2 {
        const STORAGE: crate::ecs::component::StorageTypes = StorageTypes::TableAoS;
    }

    fn test_system1(
        prm: Res<i32>,
        prm2: Res<usize>,
        mut query: Query<(&Comp1, &mut Comp2)>,
        mut query2: Query<(&Pos, &mut Pos4, &Pos3)>,
    ) {
        println!("testsystem1 res: {}, {}", prm.value, prm2.value);

        for (comp1, comp2) in query.iter() {
            println!("comp1: {:?}", comp1);
            println!("comp2: {:?}", comp2);
            println!("comp1: {}", comp1.0);
            println!("comp2: {}", comp2.0);
            comp2.0 = 2;
            println!("comp2: {}", comp2.0);
        }

        assert_eq!(query.iter().count(), 3);

        for (pos, pos4, pos3) in query2.iter() {
            println!("pos1 : {:?}", pos);
            println!("pos3: {:?}", pos3);

            println!("pos4 : {}", pos4.0);
            pos4.0 = 23234;
            assert_eq!(pos4.0, 23234);
            pos4.0 -= 2344;
            assert_eq!(pos4.0, 23234 - 2344);
            println!("pos4 : {}", pos4.0);

            //println!("pos4.1: {:?}", pos4);
            //println!("pos4.1 box pointer: {}", pos4.1.0);
            pos4.1.0 = 23234;
            assert_eq!(pos4.1.0, 23234);
            pos4.1.0 -= 2344;
            assert_eq!(pos4.1.0, 23234 - 2344);
            println!("pos4.1 box pointer: {}", pos4.1.0);
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

        es.add_entity((Pos4(12, Box::new(Pos3(1, 1, 1))), Pos(12), Pos3(12, 34, 56)));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
        es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
    }

    #[test]
    fn test_table_aos_insert() {
        let mut es = EntityStorage::new();
        init_es_insert(&mut es);
    }

    #[test]
    fn test_table_aos_query_iter() {
        let mut world = World::new();
        let num1: i32 = 2324;
        let num2: usize = 2324;
        world.add_system(test_system1);
        world.add_resource(num1);
        world.add_resource(num2);

        let es = &mut world.data.get_mut().entity_storage;
        init_es_insert(es);
        world.init_systems();
        println!(
            "archid and comps: {:?}",
            world.data.get_mut().entity_storage.compids_archid_map
        );

        let es = &mut world.data.get_mut().entity_storage;
        es.tables
            .get_mut(&ArchetypeId(0))
            .unwrap()
            .table_aos
            .print_internals(&es.components);

        world.run();
    }
}
