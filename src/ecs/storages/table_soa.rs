// table_soa.rs

use std::{any::TypeId, collections::HashMap, ptr::NonNull};

use crate::{
    ecs::component::{
        ArchetypeId, Component, ComponentId, ComponentInfo, EntityKey, EntityStorage,
    },
    utils::tuple_iters::{self, TableSoaTupleIter, TupleIterConstructor},
};

use super::thin_blob_vec::{ThinBlobIterMutUnsafe, ThinBlobIterUnsafe, ThinBlobVec};

//TODO: entities need to be stored too for querying
pub struct TableSoA {
    pub(crate) archetype_id: ArchetypeId,
    entities: Vec<EntityKey>,
    pub(crate) columns: HashMap<TypeId, ThinBlobVec>,
    pub(crate) len: usize,
    pub(crate) cap: usize,
}

//TODO: make probably every function here unsafe
impl TableSoA {
    pub fn new(archetype_id: ArchetypeId, entity_storage: &EntityStorage) -> Self {
        let mut columns = HashMap::new();
        let archetype = &entity_storage.archetypes[usize::from(archetype_id)];
        archetype.soa_comp_ids.get_vec().iter().for_each(|cid| {
            let cinfo = &entity_storage.components[usize::from(*cid)];
            columns.insert(cinfo.type_id, ThinBlobVec::new(cinfo.layout, cinfo.drop));
        });

        Self {
            archetype_id,
            entities: Vec::new(),
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
        entity: EntityKey,
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
        self.entities.push(entity);
    }

    fn update_capacity(&mut self) {
        if self.cap == 0 {
            self.cap = 4;
        } else if self.len >= self.cap {
            self.cap *= 2;
        }
    }

    pub fn remove() {}

    pub unsafe fn tuple_iter<'a, TC: TupleIterConstructor<TableSoA>>(
        &'a mut self,
    ) -> TableSoaTupleIter<TC::Construct<'a>> {
        tuple_iters::new_table_soa_iter::<TC>(self)
    }

    pub unsafe fn get_single_comp_iter<'c, T: Component>(&'c self) -> ThinBlobIterUnsafe<'c, T> {
        self.columns.get(&TypeId::of::<T>()).unwrap().tuple_iter()
    }
    pub unsafe fn get_single_comp_iter_mut<'c, T: Component>(
        &'c mut self,
    ) -> ThinBlobIterMutUnsafe<'c, T> {
        self.columns
            .get_mut(&TypeId::of::<T>())
            .unwrap()
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

#[cfg(test)]
mod tests {
    use crate::ecs::component::ArchetypeId;
    use crate::ecs::component::Component;
    use crate::ecs::component::EntityKey;
    use crate::ecs::component::EntityStorage;
    use crate::utils::gen_vec;

    use super::TableSoA;

    struct Pos(i32);
    impl Component for Pos {}

    struct Pos2(i32, i64);
    impl Component for Pos2 {}

    struct Pos3(i32, i32, i32);
    impl Component for Pos3 {}

    struct Pos4(i32, Box<Pos3>);
    impl Component for Pos4 {}

    #[test]
    fn test_table_soa() {
        let mut es = EntityStorage::new();
        //es.add_entity((Pos(12), Pos3(12, 34, 56)));
        //es.add_entity((Pos3(12, 12, 34), Pos(56)));

        //es.add_entity((Pos(12), Pos3(12, 34, 56), Pos2(213, 23)));
        es.add_entity((Pos2(213, 23), Pos(12)));
        //es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));

        let mut table = TableSoA::new(ArchetypeId(0), &es);
        table.insert(EntityKey(gen_vec::Key::new(0, 0)), (Pos(12), Pos2(12, 34)));
        table.insert(
            EntityKey(gen_vec::Key::new(0, 0)),
            (Pos(-212), Pos2(12122, 11134)),
        );
        table.insert(
            EntityKey(gen_vec::Key::new(0, 0)),
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
