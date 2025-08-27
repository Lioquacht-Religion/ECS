// query.rs

use std::{any::TypeId, cell::UnsafeCell, collections::HashSet, marker::PhantomData};

use crate::{
    all_tuples,
    ecs::entity::EntityKey,
    utils::{
        sorted_vec::SortedVec,
        tuple_iters::{
            TableStorageTupleIter, TupleConstructorSource, TupleIterConstructor, TupleIterator,
        },
    },
};

use super::{
    component::{ArchetypeId, Component, ComponentId},
    storages::table_storage::TableStorage,
    system::SystemParam,
    world::WorldData,
};

type QueryDataType = TableStorage;

pub struct Query<'w, 's, P: QueryParam> {
    world: &'w UnsafeCell<WorldData>,
    state: &'s QueryState,
    marker: PhantomData<P>,
}

pub struct QueryState {
    comp_ids: SortedVec<ComponentId>,
    optional_comp_ids: SortedVec<ComponentId>,
    shared_ref_comps: HashSet<ComponentId>,
    exclusive_ref_comps: HashSet<ComponentId>,
    arch_ids: Vec<ArchetypeId>,
}

pub struct CompRefKind(ComponentId, RefKind);

pub enum RefKind {
    Shared,
    Exclusive,
}

impl<'w, 's, P: QueryParam> Query<'w, 's, P> {
    pub fn new(world: &'w UnsafeCell<WorldData>, state: &'s QueryState) -> Self {
        Self {
            world,
            state,
            marker: Default::default(),
        }
    }

    pub fn iter(&mut self) -> QueryIter<'_, '_, P> {
        QueryIter::new(self)
    }

    unsafe fn get_arch_query_iter(
        &self,
        arch_id: ArchetypeId,
    ) -> TableStorageTupleIter<<P as TupleIterConstructor<QueryDataType>>::Construct<'w>> {
        self.world
            .get()
            .as_mut()
            .unwrap()
            .entity_storage
            .tables
            .get_mut(&arch_id)
            .expect("Table with archetype id could not be found.")
            .tuple_iter::<P>()
    }
}

pub struct QueryIter<'w, 's, T: QueryParam> {
    query: &'w Query<'w, 's, T>,
    cur_arch_query: Option<TableStorageTupleIter<T::Construct<'w>>>,
    cur_arch_index: usize,
}

impl<'w, 's, T: QueryParam> QueryIter<'w, 's, T> {
    pub fn new(query: &'w Query<'w, 's, T>) -> Self {
        let mut arch_query = None;
        if query.state.arch_ids.len() > 0 {
            let arch_id = query.state.arch_ids[0];
            arch_query = Some(unsafe { query.get_arch_query_iter(arch_id) });
        }

        Self {
            query,
            cur_arch_query: arch_query,
            cur_arch_index: 0,
        }
    }
}

impl<'w, 's, T: QueryParam> Iterator for QueryIter<'w, 's, T> {
    type Item = <<T as TupleIterConstructor<QueryDataType>>::Construct<'w> as TupleIterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut cur_query) = &mut self.cur_arch_query {
                match cur_query.next() {
                    None => {
                        self.cur_arch_index += 1;
                        if self.cur_arch_index >= self.query.state.arch_ids.len() {
                            return None;
                        }

                        let next_arch_id = self.query.state.arch_ids[self.cur_arch_index];
                        self.cur_arch_query =
                            Some(unsafe { self.query.get_arch_query_iter(next_arch_id) });
                    }
                    Some(elem) => return Some(elem),
                }
            } else {
                return None;
            }
        }
    }
}

impl<'w, 's, P: QueryParam> SystemParam for Query<'w, 's, P> {
    type Item<'new> = Query<'new, 's, P>;
    unsafe fn retrieve<'r>(world_data: &'r UnsafeCell<WorldData>) -> Self::Item<'r> {
        let world_data_mut: &mut WorldData = world_data.get().as_mut().unwrap();
        let mut comp_ids = world_data_mut
            .entity_storage
            .cache
            .compid_vec_cache
            .take_cached();
        P::comp_ids_rec(world_data, &mut comp_ids);
        let comp_ids: SortedVec<ComponentId> = comp_ids.into();

        if let Some(query_data) = world_data_mut.query_data.get(&comp_ids) {
            world_data_mut
                .entity_storage
                .cache
                .compid_vec_cache
                .insert(comp_ids.into());
            return Self::Item::<'r>::new(world_data, query_data);
        }

        let world_data_ref = world_data.get().as_mut().unwrap();
        let arch_ids = world_data_ref
            .entity_storage
            .find_fitting_archetypes(&comp_ids);
        let query_data = QueryState {
            //TODO: remove comp_ids? already used as key, remove cloning
            comp_ids: comp_ids.clone(),
            optional_comp_ids: SortedVec::new(),
            shared_ref_comps: HashSet::new(),
            exclusive_ref_comps: HashSet::new(),
            arch_ids,
        };

        world_data_ref
            .query_data
            .insert(comp_ids.clone(), query_data);

        let query_data = world_data_ref.query_data.get(&comp_ids).unwrap();
        Query::new(world_data, query_data)
    }
}

pub trait QueryParam: TupleIterConstructor<QueryDataType> {
    type QueryItem<'new>: QueryParam;

    fn type_ids_rec(vec: &mut Vec<TypeId>);
    fn comp_ids_rec(world_data: &UnsafeCell<WorldData>, vec: &mut Vec<ComponentId>);
    fn ref_kinds(vec: &mut Vec<RefKind>);
}

impl<T: Component> QueryParam for &T {
    type QueryItem<'new> = &'new T;

    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn comp_ids_rec(world_data: &UnsafeCell<WorldData>, vec: &mut Vec<ComponentId>) {
        let comp_id = unsafe {
            (&mut *world_data.get())
                .entity_storage
                .create_or_get_component::<T>()
        };
        vec.push(comp_id);
    }
    fn ref_kinds(vec: &mut Vec<RefKind>) {
        vec.push(RefKind::Shared);
    }
}
impl<T: Component> QueryParam for &mut T {
    type QueryItem<'new> = &'new mut T;

    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn comp_ids_rec(world_data: &UnsafeCell<WorldData>, vec: &mut Vec<ComponentId>) {
        let comp_id = unsafe {
            (&mut *world_data.get())
                .entity_storage
                .create_or_get_component::<T>()
        };

        vec.push(comp_id);
    }
    fn ref_kinds(vec: &mut Vec<RefKind>) {
        vec.push(RefKind::Exclusive);
    }
}

//TODO:
//impl<'p, P: QueryParam> QueryParam for Option<P> {}
//
//

/*
impl TupleIterator for EntityKey{
    type Item = EntityKey;
    unsafe fn next(&mut self, index: usize) -> Self::Item {
        todo!()
    }
}

impl<S: TupleConstructorSource> TupleIterConstructor<S> for EntityKey{
    type Construct<'c> = EntityKey;

    unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s> {
       todo!()
    }
}
*/

impl QueryParam for EntityKey {
    type QueryItem<'new> = EntityKey;

    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        //TODO: do nothing here?
    }
    fn comp_ids_rec(world_data: &UnsafeCell<WorldData>, vec: &mut Vec<ComponentId>) {
        //TODO: do nothing here?
    }
    fn ref_kinds(vec: &mut Vec<RefKind>) {
        //TODO: do nothing here?
    }
}

macro_rules! impl_query_param_tuples {
    ($($t:ident), *) => {
       impl<$($t : QueryParam), *> QueryParam for ($($t),*,){
           #[allow(unused_parens)]
               type QueryItem<'new> = ($($t),* );

               fn type_ids_rec(vec: &mut Vec<TypeId>){
                   $($t::type_ids_rec(vec);)*
               }
               fn comp_ids_rec(world_data: &UnsafeCell<WorldData>, vec: &mut Vec<ComponentId>) {
                   $($t::comp_ids_rec(world_data, vec);)*
               }
               fn ref_kinds(vec: &mut Vec<RefKind>){
                   $($t::ref_kinds(vec);)*
               }
        }
    };
}

#[rustfmt::skip]
all_tuples!(
    impl_query_param_tuples,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

#[cfg(test)]
mod test {
    use std::usize;

    use crate::ecs::{component::Component, system::Res, world::World};

    use super::Query;

    struct Comp1(usize, usize);
    impl Component for Comp1 {}

    struct Comp2(usize, usize);
    impl Component for Comp2 {}

    fn test_system1(
        prm: Res<i32>,
        prm2: Res<usize>,
        mut query: Query<(&Comp1, &mut Comp2)>,
        //query2: Query<&mut Comp1>,
    ) {
        println!("testsystem1 res: {}, {}", prm.value, prm2.value);

        for (comp1, comp2) in query.iter() {
            println!("comp1: {}", comp1.0);
            println!("comp2: {}", comp2.0);
            comp2.0 = 2;
        }
    }

    #[test]
    fn it_works() {
        let mut world = World::new();
        let num1: i32 = 2324;
        let num2: usize = 2324;
        world.systems.add_system(test_system1);
        unsafe { (&mut *world.data.get()).add_resource(num1) };
        unsafe { (&mut *world.data.get()).add_resource(num2) };
        let es = &mut world.data.get_mut().entity_storage;
        es.add_entity((Comp2(12, 34), Comp1(56, 78)));
        es.add_entity(Comp1(12, 34));
        es.add_entity((Comp1(12, 34), Comp2(56, 78)));
        es.add_entity((Comp1(12, 34), Comp2(56, 78)));
        world.run();
    }
}
