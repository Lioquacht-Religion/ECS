// query.rs

use std::{
    any::TypeId,
    collections::{HashSet, hash_set},
    hash::Hash,
    marker::PhantomData,
};

use crate::{
    all_tuples,
    ecs::{
        ecs_dependency_graph::QueryId,
        entity::EntityKey,
        query::query_filter::{FilterElem, QueryFilter},
        storages::table_storage::TableStorageTupleIter,
        system::{SystemId, SystemParamId},
    },
    utils::{
        ecs_id::EcsId,
        sorted_vec::SortedVec,
        tuple_iters::{TupleIterConstructor, TupleIterator},
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
    world: *mut WorldData,
    state: &'s QueryState,
    _param_marker: PhantomData<fn() -> P>,
    _filter_marker: PhantomData<fn() -> F>,
    _world_lt_marker: PhantomData<&'w WorldData>,
}

unsafe impl<'w, 's, P: QueryParam, F: QueryFilter> Send for Query<'w, 's, P, F> {}
unsafe impl<'w, 's, P: QueryParam, F: QueryFilter> Sync for Query<'w, 's, P, F> {}

#[derive(Debug)]
pub(crate) struct QueryState {
    pub(crate) query_param_meta_data: SortedVec<QueryParamMetaData>,
    pub(crate) arch_ids: HashSet<ArchetypeId>,
    pub(crate) filter: Vec<FilterElem>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct QueryStateKey {
    comp_ids: SortedVec<ComponentId>,
    filter: Vec<FilterElem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefKind {
    Shared,
    Exclusive,
}

impl<'w, 's, P: QueryParam, F: QueryFilter> Query<'w, 's, P, F> {
    pub(crate) fn new(
        world: /*TODO: should be: &'w*/ *mut WorldData,
        state: &'s QueryState,
    ) -> Self {
        Self {
            world,
            state,
            _param_marker: Default::default(),
            _filter_marker: Default::default(),
            _world_lt_marker: Default::default(),
        }
    }

    ///TODO: add documentation and examples
    pub fn iter(&mut self) -> QueryIter<'_, '_, P, F> {
        QueryIter::new(self)
    }

    pub fn get_entry(
        &mut self,
        entity_key: EntityKey,
    ) -> Option<<P::Construct<'_> as TupleIterator>::Item> {
        unsafe { (&mut *self.world).get_entity_components::<P>(entity_key) }
    }

    #[inline(never)]
    #[cold]
    unsafe fn get_arch_query_iter(
        &self,
        arch_id: ArchetypeId,
    ) -> TableStorageTupleIter<<P as TupleIterConstructor<QueryDataType>>::Construct<'w>> {
        unsafe {
            (&mut *self.world)
                .get_tables_mut()
                .get_mut(&arch_id)
                .expect("Table with archetype id could not be found.")
                .tuple_iter::<P>()
        }
    }
}

pub struct QueryIter<'w, 's, T: QueryParam, F: QueryFilter> {
    query: &'w Query<'w, 's, T, F>,
    cur_arch_query: Option<TableStorageTupleIter<T::Construct<'w>>>,
    cur_arch_index: hash_set::Iter<'s, ArchetypeId>,
}

impl<'w, 's, T: QueryParam, F: QueryFilter> QueryIter<'w, 's, T, F> {
    pub fn new(query: &'w Query<'w, 's, T, F>) -> Self {
        let mut arch_query = None;
        let mut arch_ids_iter = query.state.arch_ids.iter();
        if query.state.arch_ids.len() > 0 {
            let arch_id = <_ as Iterator>::next(&mut arch_ids_iter).unwrap();
            arch_query = Some(unsafe { query.get_arch_query_iter(*arch_id) });
        }

        Self {
            query,
            cur_arch_query: arch_query,
            cur_arch_index: arch_ids_iter,
        }
    }
}

impl<'w, 's, T: QueryParam, F: QueryFilter> Iterator for QueryIter<'w, 's, T, F> {
    type Item = <<T as TupleIterConstructor<QueryDataType>>::Construct<'w> as TupleIterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(cur_query) = &mut self.cur_arch_query {
                match <_ as Iterator>::next(cur_query) {
                    Some(elem) => return Some(elem),
                    None => {
                        if let Some(next_arch_id) = <_ as Iterator>::next(&mut self.cur_arch_index)
                        {
                            self.cur_arch_query =
                                Some(unsafe { self.query.get_arch_query_iter(*next_arch_id) });
                        } else {
                            return None;
                        }
                    }
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
        world_data: *mut WorldData,
    ) -> Self::Item<'r> {
        let world_data_mut = unsafe { world_data.as_mut().unwrap() };
        let sys_prm_id = &system_param_ids[*system_param_index];
        if let SystemParamId::Query(qid) = sys_prm_id {
            let qs = &world_data_mut.get_query_data_mut()[qid.id_usize()];
            *system_param_index += 1;
            //TODO: wrap WorldData reference in some temporary unsafe access type that is Sync and
            //Send
            return Query::new(world_data, qs);
        }
        panic!(
            "SystemParamId=<{}> is not a QueryId! param_ids: {:?}",
            *system_param_index, system_param_ids
        )
    }

    fn create_system_param_data(
        system_id: SystemId,
        system_param_ids: &mut Vec<SystemParamId>,
        world_data: &mut WorldData,
    ) {
        let mut comp_ids = world_data.get_cache_mut().compid_vec_cache.take_cached();
        P::comp_ids_rec(world_data, &mut comp_ids);
        let comp_ids: SortedVec<ComponentId> = comp_ids.into();

        let mut query_prm_meta_data = world_data
            .get_cache_mut()
            .query_param_meta_data_vec_cache
            .take_cached();
        P::meta_data(world_data, &mut query_prm_meta_data);
        let query_prm_meta_data: SortedVec<QueryParamMetaData> = query_prm_meta_data.into();

        let mut filter = Vec::new();
        F::get_and_filters(world_data, &mut filter);

        let query_state_key = QueryStateKey { comp_ids, filter };

        let arch_ids = world_data.find_fitting_archetypes(&query_prm_meta_data);

        // remove archetypes that do not match the filter
        let arch_ids: Vec<ArchetypeId> = arch_ids
            .iter()
            .filter(|aid| {
                let arch = &world_data.get_archetypes()[aid.0 as usize];
                let comp_ids_set: HashSet<ComponentId> = HashSet::from_iter(
                    arch.soa_comp_ids
                        .iter()
                        .chain(arch.aos_comp_ids.iter())
                        .map(|cid| *cid),
                );
                let not_filtered_out = query_filter::comp_ids_compatible_with_filter(
                    &comp_ids_set,
                    &query_state_key.filter,
                );
                not_filtered_out
            })
            .cloned()
            .collect();

        let next_query_id = world_data.get_query_data().len().into();
        system_param_ids.push(SystemParamId::Query(next_query_id));

        // adding system dependencies to graph
        // systems <- add queries <- add components and filtered archetypes

        let depend_graph = &mut world_data.get_depend_graph_mut();
        depend_graph.insert_system_components(system_id, &query_prm_meta_data.get_vec());
        depend_graph.insert_query_archetypes(next_query_id, &arch_ids);
        depend_graph.insert_system_query(system_id, next_query_id);

        let QueryStateKey {
            comp_ids: _comp_ids,
            filter,
        } = query_state_key;

        let arch_ids: HashSet<ArchetypeId> = arch_ids.iter().map(|aid| aid.clone()).collect();
        let query_data = QueryState {
            query_param_meta_data: query_prm_meta_data,
            arch_ids: arch_ids,
            filter: filter,
        };

        world_data.get_query_data_mut().push(query_data);
    }
}

#[derive(Debug, Eq, PartialOrd)]
pub struct QueryParamMetaData {
    pub type_id: TypeId,
    pub comp_id: ComponentId,
    pub ref_kind: RefKind,
    pub optional: bool,
}
impl Hash for QueryParamMetaData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.comp_id.hash(state);
    }
}
impl PartialEq for QueryParamMetaData {
    fn eq(&self, other: &Self) -> bool {
        self.comp_id.eq(&other.comp_id)
    }
}
impl Ord for QueryParamMetaData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.comp_id.cmp(&other.comp_id)
    }
}

pub trait QueryParam: TupleIterConstructor<QueryDataType> {
    type QueryItem<'new>: QueryParam;

    fn type_ids_rec(vec: &mut Vec<TypeId>);
    fn comp_ids_rec(world_data: &mut WorldData, vec: &mut Vec<ComponentId>);
    fn ref_kinds(vec: &mut Vec<RefKind>);
    fn optional_param_rec(vec: &mut Vec<bool>);
    fn meta_data(world_data: &mut WorldData, vec: &mut Vec<QueryParamMetaData>);
}

impl<T: Component> QueryParam for &T {
    type QueryItem<'new> = &'new T;

    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn comp_ids_rec(world_data: &mut WorldData, vec: &mut Vec<ComponentId>) {
        let comp_id = world_data.create_or_get_component::<T>();
        vec.push(comp_id);
    }
    fn ref_kinds(vec: &mut Vec<RefKind>) {
        vec.push(RefKind::Shared);
    }
    fn optional_param_rec(vec: &mut Vec<bool>) {
        vec.push(false);
    }
    fn meta_data(world_data: &mut WorldData, vec: &mut Vec<QueryParamMetaData>) {
        let comp_id = world_data.create_or_get_component::<T>();
        vec.push(QueryParamMetaData {
            type_id: TypeId::of::<T>(),
            comp_id,
            ref_kind: RefKind::Shared,
            optional: false,
        });
    }
}
impl<T: Component> QueryParam for &mut T {
    type QueryItem<'new> = &'new mut T;

    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn comp_ids_rec(world_data: &mut WorldData, vec: &mut Vec<ComponentId>) {
        let comp_id = world_data.create_or_get_component::<T>();
        vec.push(comp_id);
    }
    fn ref_kinds(vec: &mut Vec<RefKind>) {
        vec.push(RefKind::Exclusive);
    }
    fn optional_param_rec(vec: &mut Vec<bool>) {
        vec.push(false);
    }
    fn meta_data(world_data: &mut WorldData, vec: &mut Vec<QueryParamMetaData>) {
        let comp_id = world_data.create_or_get_component::<T>();
        vec.push(QueryParamMetaData {
            type_id: TypeId::of::<T>(),
            comp_id,
            ref_kind: RefKind::Exclusive,
            optional: false,
        });
    }
}

impl<'p, T: Component> QueryParam for Option<&T> {
    type QueryItem<'new> = Option<&'new T>;
    fn ref_kinds(vec: &mut Vec<RefKind>) {
        vec.push(RefKind::Shared);
    }
    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn comp_ids_rec(world_data: &mut WorldData, vec: &mut Vec<ComponentId>) {
        vec.push(world_data.create_or_get_component::<T>());
    }
    fn optional_param_rec(vec: &mut Vec<bool>) {
        vec.push(true);
    }
    fn meta_data(world_data: &mut WorldData, vec: &mut Vec<QueryParamMetaData>) {
        let comp_id = world_data.create_or_get_component::<T>();
        vec.push(QueryParamMetaData {
            type_id: TypeId::of::<T>(),
            comp_id,
            ref_kind: RefKind::Shared,
            optional: true,
        });
    }
}

impl<'p, T: Component> QueryParam for Option<&mut T> {
    type QueryItem<'new> = Option<&'new mut T>;
    fn ref_kinds(vec: &mut Vec<RefKind>) {
        vec.push(RefKind::Shared);
    }
    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn comp_ids_rec(world_data: &mut WorldData, vec: &mut Vec<ComponentId>) {
        vec.push(world_data.create_or_get_component::<T>());
    }
    fn optional_param_rec(vec: &mut Vec<bool>) {
        vec.push(true);
    }
    fn meta_data(world_data: &mut WorldData, vec: &mut Vec<QueryParamMetaData>) {
        let comp_id = world_data.create_or_get_component::<T>();
        vec.push(QueryParamMetaData {
            type_id: TypeId::of::<T>(),
            comp_id,
            ref_kind: RefKind::Exclusive,
            optional: true,
        });
    }
}

impl QueryParam for EntityKey {
    type QueryItem<'new> = EntityKey;

    fn type_ids_rec(_vec: &mut Vec<TypeId>) {
        // do nothing here for entity keys
    }
    fn comp_ids_rec(_world_data: &mut WorldData, _vec: &mut Vec<ComponentId>) {
        // do nothing here for entity keys
    }
    fn ref_kinds(_vec: &mut Vec<RefKind>) {
        // do nothing here for entity keys
    }
    fn optional_param_rec(_vec: &mut Vec<bool>) {
        // do nothing here for entity keys
    }
    fn meta_data(_world_data: &mut WorldData, _vec: &mut Vec<QueryParamMetaData>) {
        // do nothing here for entity keys
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
               fn comp_ids_rec(world_data: &mut WorldData, vec: &mut Vec<ComponentId>) {
                   $($t::comp_ids_rec(world_data, vec);)*
               }
               fn ref_kinds(vec: &mut Vec<RefKind>){
                   $($t::ref_kinds(vec);)*
               }
               fn optional_param_rec(vec: &mut Vec<bool>) {
                   $($t::optional_param_rec(vec);)*
               }
               fn meta_data(world_data: &mut WorldData, vec: &mut Vec<QueryParamMetaData>) {
                   $($t::meta_data(world_data, vec);)*
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
        system::{Res, ResMut},
        world::World,
    };

    use super::Query;

    struct Comp1(usize, usize);
    impl Component for Comp1 {}

    struct Comp2(usize, usize);
    impl Component for Comp2 {}

    struct Pos1(usize, usize);
    impl Component for Pos1 {}

    struct Pos2(usize, usize);
    impl Component for Pos2 {}

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

    fn test_system5(mut query2: Query<(&Pos1, &Pos2)>, mut query: Query<(&Comp1, &Marker1)>) {
        for (comp1, _m) in query.iter() {
            println!("comp1: {}; {}", comp1.0, comp1.1);
        }
        assert_eq!(query.iter().count(), 1);
        for (p1, p2) in query2.iter() {
            println!("p1: {}; p2:{}", p1.0, p2.1);
        }
        assert_eq!(query2.iter().count(), 2);
    }

    fn test_system6(
        mut test_sys6_ran: ResMut<TestSystem6Ran>,
        mut query1: Query<&Pos2>,
        mut query2: Query<(&Comp1, Option<&Pos1>)>,
    ) {
        for (comp1, pos1) in query2.iter() {
            println!("comp1: {}; {};", comp1.0, comp1.1);
            if let Some(pos1) = pos1 {
                println!("pos1: {}; {}", pos1.0, pos1.1);
            } else {
                println!("no pos1");
            }
        }
        assert_eq!(query2.iter().count(), 5);
        for p2 in query1.iter() {
            println!("p2: {}", p2.0);
        }
        assert_eq!(query1.iter().count(), 2);
        test_sys6_ran.0 = true;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct TestSystem6Ran(bool);

    #[test]
    fn queries_test1() {
        let mut world = World::new();
        let num1: i32 = 2345678;
        let num2: usize = 33330000;

        world.add_resource(TestSystem6Ran(false));
        world.add_resource(num1);
        world.add_resource(num2);
        world.add_systems(test_system1);
        world.add_systems(test_system2);
        world.add_systems(test_system3);
        world.add_systems(test_system4);
        world.add_systems(test_system5);
        world.add_systems(test_system6);

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
        world.add_entity((Pos1(12, 34), Pos2(12, 43)));
        world.add_entity((Pos1(12, 34), Pos2(12, 43)));
        world.add_entity((Pos1(12, 34), Comp1(12, 34)));

        world.init_and_run();

        assert_eq!(
            world.get_resource::<TestSystem6Ran>(),
            Some(&TestSystem6Ran(true))
        );
    }
}
