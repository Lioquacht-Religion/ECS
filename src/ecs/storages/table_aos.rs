//table_aos.rs

use core::panic;
use std::{any::{Any, TypeId}, collections::HashMap, ptr::NonNull};

use crate::{
    ecs::component::{ArchetypeId, ComponentId, EntityKey, EntityStorage},
    utils::{sorted_vec::SortedVec, tuple_iters::{self, TableAosTupleIter, TupleIterConstructor, TupleIterator}, tuple_types::TupleTypesExt},
};

use super::{table_addable::TableAddable, thin_blob_vec::{CompElemPtr, ThinBlobVec}};

#[derive(Debug, Hash, Eq)]
pub(crate) struct TypeMetaData {
    pub(crate) comp_id: ComponentId,
    pub(crate) ptr_offset: usize,
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
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) entities: Vec<EntityKey>,
    pub(crate) vec: ThinBlobVec,
    pub(crate) cap: usize,
    pub(crate) len: usize,
    pub(crate) type_meta_data_map: HashMap<TypeId, usize>,
    pub(crate) type_meta_data: SortedVec<TypeMetaData>,
}

impl TableAoS {
    pub fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        let archetype = &entity_storage.archetypes[usize::from(archetype_id)];

        let comp_ids: &[ComponentId] = &archetype.comp_ids.get_vec();
        let mut meta_data: Vec<TypeMetaData> = Vec::with_capacity(comp_ids.len());

        let mut comp_id_iter = comp_ids.iter();
        let mut type_meta_data_map : HashMap<TypeId, usize> = HashMap::with_capacity(comp_ids.len());

        if let Some(comp_id) = comp_id_iter.next() {
            let comp_info = &entity_storage.components[comp_id.0 as usize];
            let mut row_layout = comp_info.layout;
            meta_data.push(TypeMetaData {
                comp_id: *comp_id,
                ptr_offset: 0,
            });

            // comp_ids vec from entity storage should be ordered
            // so index should line up with type_meta_data order
            let mut index = 0;
            type_meta_data_map.insert(comp_info.type_id(), index);

            while let Some(comp_id) = comp_id_iter.next() {
                index += 1;
                let comp_info = &entity_storage.components[comp_id.0 as usize];
                let (new_row_layout, offset) = row_layout
                    .extend(comp_info.layout)
                    .expect("Allocation error in table AoS!");
                row_layout = new_row_layout;
                meta_data.push(TypeMetaData {
                    comp_id: *comp_id,
                    ptr_offset: offset,
                });
                type_meta_data_map.insert(comp_info.type_id(), index);
            }
            return Self {
                archetype_id,
                entities: Vec::new(),
                vec: ThinBlobVec::new(row_layout, None),
                cap: 0,
                len: 0,
                type_meta_data_map,
                type_meta_data: meta_data.into(),
            };
        }
        panic!("Completely empty Archetype not allowed!")
    }

    fn update_capacity(&mut self) {
        if self.cap == 0 {
            self.cap = 4;
        } else if self.len >= self.cap {
            self.cap *= 2;
        }
    }

    pub unsafe fn insert<T: TupleTypesExt>(
        &mut self,
        entity: EntityKey,
        entity_storage: &mut EntityStorage,
        mut value: T,
    ) {
        self.entities.push(entity);
        let mut comp_ids = Vec::with_capacity(T::get_tuple_length());
        T::create_or_get_component(entity_storage, &mut comp_ids);
        let mut ptrs = value.self_get_elem_ptrs();
        let mut comp_elem_ptrs: Vec<CompElemPtr> = Vec::with_capacity(comp_ids.len());

        for _i in 0..comp_ids.len() {
            comp_elem_ptrs.push(CompElemPtr {
                comp_id: comp_ids.pop().unwrap(),
                ptr: NonNull::new_unchecked(ptrs.pop().unwrap()),
            });
        }

        let comp_elem_ptrs: SortedVec<CompElemPtr> = comp_elem_ptrs.into();
        let offsets: Vec<usize> = self.type_meta_data.iter().map(|t| t.ptr_offset).collect();

        self.vec.push_ptr_vec_untyped(
            &mut self.cap,
            &mut self.len,
            &entity_storage.components,
            &offsets,
            &comp_elem_ptrs.get_vec(),
        );
        self.update_capacity();
        self.len += 1;
    }

    pub unsafe fn batch_insert() {
        unimplemented!()
    }

    pub fn get() {}

    pub(crate) unsafe fn get_by_index<T: TableAddable>(
        &mut self, index: usize
    ) -> Option<&mut T>{
        let row_ptr = self.vec.get_ptr_untyped(index, self.vec.layout);

        unimplemented!()
    }

    pub fn remove() {}

    pub fn iter() {}

    pub fn tuple_iter<'a, TC: TupleIterConstructor<TableAoS>>(
        &'a mut self,
    ) -> TableAosTupleIter<TC::Construct<'a>> {
        tuple_iters::new_table_aos_iter::<TC>(self)
    }

    pub fn tuple_iter_mut() {}
}

impl Drop for TableAoS {
    fn drop(&mut self) {
        //TODO: what about removing already empty entry in thin_blob_vec?
        //TODO: remove free indexes concept everywhere
        // just move last entry to empty spot and updat entity vec with new position
        // generation entity index stored by other systems is not effected
    }
}

#[cfg(test)]
mod test {

    use crate::ecs::component::ArchetypeId;
    use crate::ecs::component::Component;
    use crate::ecs::component::EntityKey;
    use crate::ecs::component::EntityStorage;
    use crate::utils::gen_vec;

    use super::TableAoS;

    struct Pos(i32);
    impl Component for Pos {}

    struct Pos2(i32, i64);
    impl Component for Pos2 {}

    struct Pos3(i32, i32, i32);
    impl Component for Pos3 {}

    struct Pos4(i32, Box<Pos3>);
    impl Component for Pos4 {}

    #[test]
    fn test_table_aos() {
        let mut es = EntityStorage::new();
        //es.add_entity((Pos(12), Pos3(12, 34, 56)));
        //es.add_entity((Pos3(12, 12, 34), Pos(56)));

        //es.add_entity((Pos(12), Pos3(12, 34, 56), Pos2(213, 23)));
        es.add_entity((Pos2(213, 23), Pos(12)));
        //es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));

        unsafe {
            let mut table = TableAoS::new(ArchetypeId(0), &es);
            table.insert(
                EntityKey(gen_vec::Key::new(0, 0)),
                &mut es,
                (Pos(12), Pos2(12, 34)),
            );
            table.insert(
                EntityKey(gen_vec::Key::new(0, 0)),
                &mut es,
                (Pos(-212), Pos2(12122, 11134)),
            );
            table.insert(
                EntityKey(gen_vec::Key::new(0, 0)),
                &mut es,
                (Pos(2312), Pos2(-3412, 934)),
            );

            let iter = table.tuple_iter::<(&mut Pos, &mut Pos2)>();

            for (pos, pos2) in iter {
                pos.0 = 999;
                pos2.1 -= 343;
            }

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
}
