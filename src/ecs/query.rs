// query.rs

use std::{any::TypeId, cell::UnsafeCell, collections::HashSet, marker::PhantomData};

use crate::{
    all_tuples,
    ecs::{
        entity::EntityKey,
        query::query_filter::{FilterElem, QueryFilter},
        storages::table_storage::TableStorageTupleIter,
        system::{SystemId, SystemParamId},
    },
    utils::{
        ecs_id::EcsId, sorted_vec::SortedVec, tuple_iters::{TupleIterConstructor, TupleIterator}
    },
};

use super::{
    component::{ArchetypeId, Component, ComponentId},
    storages::table_storage::TableStorage,
    system::SystemParam,
    world::WorldData,
};

pub mod query_filter;

type QueryDataType = TableStorage;

pub struct Query<'w, 's, P: QueryParam, F: QueryFilter = ()> {
    world: &'w UnsafeCell<WorldData>,
    state: &'s QueryState,
    _param_marker: PhantomData<P>,
    _filter_marker: PhantomData<F>,
}

#[derive(Debug)]
pub struct QueryState {
    comp_ids: SortedVec<ComponentId>,
    optional_comp_ids: SortedVec<ComponentId>,
    shared_ref_comps: HashSet<ComponentId>,
    exclusive_ref_comps: HashSet<ComponentId>,
    arch_ids: Vec<ArchetypeId>,
    filter: Vec<FilterElem>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct QueryStateKey {
    comp_ids: SortedVec<ComponentId>,
    filter: Vec<FilterElem>,
}

//pub struct CompRefKind(ComponentId, RefKind);

pub enum RefKind {
    Shared,
    Exclusive,
}

impl<'w, 's, P: QueryParam, F: QueryFilter> Query<'w, 's, P, F> {
    pub fn new(world: &'w UnsafeCell<WorldData>, state: &'s QueryState) -> Self {
        Self {
            world,
            state,
            _param_marker: Default::default(),
            _filter_marker: Default::default(),
        }
    }

    pub fn iter(&mut self) -> QueryIter<'_, '_, P, F> {
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

pub struct QueryIter<'w, 's, T: QueryParam, F: QueryFilter> {
    query: &'w Query<'w, 's, T, F>,
    cur_arch_query: Option<TableStorageTupleIter<T::Construct<'w>>>,
    cur_arch_index: usize,
}

impl<'w, 's, T: QueryParam, F: QueryFilter> QueryIter<'w, 's, T, F> {
    pub fn new(query: &'w Query<'w, 's, T, F>) -> Self {
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

impl<'w, 's, T: QueryParam, F: QueryFilter> Iterator for QueryIter<'w, 's, T, F> {
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

impl<'w, 's, P: QueryParam, F: QueryFilter> SystemParam for Query<'w, 's, P, F> {
    type Item<'new> = Query<'new, 's, P, F>;
    unsafe fn retrieve<'r>(
        system_param_index: &mut usize,
        system_param_ids: &[SystemParamId],
        world_data: &'r UnsafeCell<WorldData>,
    ) -> Self::Item<'r> {
        let world_data_mut = world_data.get().as_mut().unwrap();
        let sys_prm_id = &system_param_ids[*system_param_index];
        if let SystemParamId::Query(qid) = sys_prm_id{
           let qs = &world_data_mut.query_data[qid.id_usize()];
           println!("init querystate: {:?}", qs);
           
           return Query::new(world_data, qs);
        }
        panic!("SystemParamId=<{}> is not a QueryId! param_ids: {:?}", *system_param_index, system_param_ids)
    }

    fn create_system_param_data(
        system_id: SystemId,
        system_param_ids: &mut Vec<SystemParamId>,
        world_data: &UnsafeCell<WorldData>,
    ) {
        let world_data_mut = unsafe { world_data.get().as_mut().unwrap() };
        let mut comp_ids = world_data_mut
            .entity_storage
            .cache
            .compid_vec_cache
            .take_cached();
        P::comp_ids_rec(world_data, &mut comp_ids);
        let comp_ids: SortedVec<ComponentId> = comp_ids.into();
        let mut ref_kinds: Vec<RefKind> = Vec::with_capacity(comp_ids.get_vec().len());
        P::ref_kinds(&mut ref_kinds);

        let mut filter = Vec::new();
        F::get_and_filters(&mut world_data_mut.entity_storage, &mut filter);

        let query_state_key = QueryStateKey { comp_ids, filter };

        let arch_ids = world_data_mut
            .entity_storage
            .find_fitting_archetypes(&query_state_key.comp_ids);

        // remove archetypes that do not match the filter
        let arch_ids : Vec<ArchetypeId> = arch_ids
            .iter()
            .filter(|aid| {
                let arch = &world_data_mut.entity_storage.archetypes[aid.0 as usize];
                let comp_ids_set: HashSet<ComponentId> = HashSet::from_iter(
                    arch.soa_comp_ids
                        .iter()
                        .chain(arch.aos_comp_ids.iter())
                        .map(|cid| *cid),
                );
                let res = query_filter::comp_ids_compatible_with_filter(
                    &comp_ids_set,
                    &query_state_key.filter,
                );
                res
            })
            .cloned()
            .collect();

        let next_query_id = world_data_mut.query_data.len().into();
        system_param_ids.push(SystemParamId::Query(next_query_id));

        // adding system dependencies to graph
        // systems <- add queries <- add components and filtered archetypes

        let depend_graph = &mut world_data_mut.entity_storage.depend_graph;
        depend_graph.insert_system_components(
            system_id,
            &query_state_key.comp_ids.get_vec(),
            &ref_kinds,
        );
        depend_graph.insert_query_archetypes(
            next_query_id,
            &arch_ids
        );
        depend_graph.insert_system_query(system_id, next_query_id);

        let query_data = QueryState {
            //TODO: remove comp_ids/filter ? already used as key, remove cloning
            comp_ids: query_state_key.comp_ids.clone(),
            optional_comp_ids: SortedVec::new(),
            shared_ref_comps: HashSet::new(),
            exclusive_ref_comps: HashSet::new(),
            arch_ids,
            filter: query_state_key.filter.clone(),
        };

        world_data_mut.query_data.push(query_data);
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

impl QueryParam for EntityKey {
    type QueryItem<'new> = EntityKey;

    fn type_ids_rec(_vec: &mut Vec<TypeId>) {
        //TODO: do nothing here?
    }
    fn comp_ids_rec(_world_data: &UnsafeCell<WorldData>, _vec: &mut Vec<ComponentId>) {
        //TODO: do nothing here?
    }
    fn ref_kinds(_vec: &mut Vec<RefKind>) {
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

    use crate::ecs::{
        component::Component,
        query::query_filter::{Or, With, Without},
        system::Res,
        world::World,
    };

    use super::Query;

    struct Comp1(usize, usize);
    impl Component for Comp1 {}

    struct Comp2(usize, usize);
    impl Component for Comp2 {}

    struct Marker1();
    impl Component for Marker1 {}
    struct Marker2();
    impl Component for Marker2 {}
    struct Marker3();
    impl Component for Marker3 {}

    fn test_system1(prm: Res<i32>, prm2: Res<usize>, mut query: Query<(&Comp1, &mut Comp2)>) {
        println!("testsystem1 res: {}, {}", prm.value, prm2.value);
        assert!(*prm.value == 2345678_i32);
        assert!(*prm2.value == 33330000);

        for (comp1, comp2) in query.iter() {
            println!("comp1: {}; {}", comp1.0, comp1.1);
            println!("comp2: {}; {}", comp2.0, comp2.1);
            comp2.0 = 2;
            assert_eq!(comp2.0, 2);
        }
        assert_eq!(query.iter().count(), 3);
    }

    fn test_system2(mut query: Query<&Comp1>) {
        for comp1 in query.iter() {
            println!("comp1: {}; {}", comp1.0, comp1.1);
            assert_eq!(comp1.0, 12);
            assert_eq!(comp1.1, 34);
        }
        assert_eq!(query.iter().count(), 4);
    }

    fn test_system3(mut query: Query<(&Comp1, &Marker1), Without<Marker2>>) {
        for (comp1, _) in query.iter() {
            println!("comp1: {}; {}", comp1.0, comp1.1);
            assert_eq!(comp1.0, 12);
            assert_eq!(comp1.1, 34);
        }
        assert_eq!(query.iter().count(), 0);
    }

    fn test_system4(mut query: Query<&Comp1, Or<(With<Marker1>, With<Marker2>, With<Marker3>)>>) {
        for comp1 in query.iter() {
            println!("comp1: {}; {}", comp1.0, comp1.1);
        }
        assert_eq!(query.iter().count(), 3);
    }

    #[test]
    fn queries_test1() {
        let mut world = World::new();
        let num1: i32 = 2345678;
        let num2: usize = 33330000;

        world.add_resource(num1);
        world.add_resource(num2);
        world.add_system(test_system1);
        world.add_system(test_system2);
        world.add_system(test_system3);
        world.add_system(test_system4);

        world.add_entity((
            Comp2(56, 78),
            Comp1(12, 34),
            Marker1(),
            Marker2(),
            Marker3(),
        ));
        world.add_entity((Comp1(12, 34), Comp2(56, 78), Marker2()));
        world.add_entity((Comp1(12, 34), Comp2(56, 78), Marker3()));
        world.add_entity(Comp1(12, 34));

        world.init_and_run();
    }
}
