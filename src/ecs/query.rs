// query.rs

use std::{any::TypeId, cell::UnsafeCell, collections::HashSet, marker::PhantomData};

use crate::{
    all_tuples,
    ecs::{entity::EntityKey, storages::{entity_storage::EntityStorage, table_storage::TableStorageTupleIter}},
    utils::{
        sorted_vec::SortedVec,
        tuple_iters::{
            TupleIterConstructor, TupleIterator,
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

pub struct Query<'w, 's, P: QueryParam, F: QueryFilter = ()> {
    world: &'w UnsafeCell<WorldData>,
    state: &'s QueryState,
    _param_marker: PhantomData<P>,
    _filter_marker: PhantomData<F>,
}

pub struct QueryState {
    comp_ids: SortedVec<ComponentId>,
    optional_comp_ids: SortedVec<ComponentId>,
    shared_ref_comps: HashSet<ComponentId>,
    exclusive_ref_comps: HashSet<ComponentId>,
    arch_ids: Vec<ArchetypeId>,
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

        // remove archetypes that do not match the filter
        let mut filter = Vec::new();
        F::get_filters(&mut world_data_ref.entity_storage, &mut filter);

        let arch_ids = arch_ids.iter().filter(|aid| {
            let arch = &world_data_ref.entity_storage.archetypes[aid.0 as usize];
            let comp_ids_set : HashSet<ComponentId> = HashSet::from_iter(
                arch.soa_comp_ids.iter().chain(arch.aos_comp_ids.iter()).map(|cid| *cid)
            );
            let res = comp_ids_compatible_with_filter(&comp_ids_set, &filter);
            println!("arch with comps: {:?}; filter: {:?}; valid status: {res}", &comp_ids_set, &filter);
            res
        }).cloned().collect();

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

fn comp_ids_compatible_with_filter(comp_ids: &HashSet<ComponentId>, filter: &[FilterElem]) -> bool{
    for el in filter.iter(){
        if !handle_filter_elem(comp_ids, el){
            return false;
        }
    }
    true
}

fn handle_filter_elem(comp_ids: &HashSet<ComponentId>, filter_elem: &FilterElem) -> bool{
        match filter_elem{
            FilterElem::With(id) => {
                comp_ids.contains(id)
            }
            FilterElem::Without(id) => {
                !comp_ids.contains(id)
            }
            FilterElem::Or(or_elems) => {
                handle_or_elems(comp_ids, or_elems)
            }
        }
}

fn handle_or_elems(comp_ids: &HashSet<ComponentId>, or_elems: &[OrFilterElem]) -> bool {
            for or_el in or_elems.iter(){
                    match or_el{
                        OrFilterElem::Single(el) => {
                            if handle_filter_elem(comp_ids, el){
                                return true;
                            }
                        }
                        OrFilterElem::And(and_elems) => {
                            if comp_ids_compatible_with_filter(comp_ids, &and_elems){
                                return true;
                            }
                        }
                    }
            }
    false
}

pub trait QueryFilter{
    fn get_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>);
    fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<OrFilterElem>);
}

pub struct With<T: Component>{
    _marker: PhantomData<T>
}
pub struct Without<T: Component>{
    _marker: PhantomData<T>
}
pub struct Or<F: QueryFilter>{
    _marker: PhantomData<F>
}

#[derive(Debug)]
pub enum FilterElem{
    With(ComponentId),
    Without(ComponentId),
    Or(Vec<OrFilterElem>),
}

#[derive(Debug)]
pub enum OrFilterElem{
    Single(FilterElem),
    And(Vec<FilterElem>),
}

impl QueryFilter for (){
    fn get_filters(_es: &mut EntityStorage, _filter_elems: &mut Vec<FilterElem>) {
    }
    fn get_or_filters(_es: &mut EntityStorage, _filter_elems: &mut Vec<OrFilterElem>) {
    }
}
impl<T: Component> QueryFilter for With<T>{
    fn get_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>) {
        filter_elems.push(FilterElem::With(es.create_or_get_component::<T>()));
    }
    fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<OrFilterElem>) {
        filter_elems.push(OrFilterElem::Single(FilterElem::With(es.create_or_get_component::<T>())));
    }
}
impl<T: Component> QueryFilter for Without<T>{
    fn get_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>) {
        filter_elems.push(FilterElem::Without(es.create_or_get_component::<T>()));
    }
    fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<OrFilterElem>) {
        filter_elems.push(OrFilterElem::Single(FilterElem::Without(es.create_or_get_component::<T>())));
    }
}
impl<F: QueryFilter> QueryFilter for Or<F>{
    fn get_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>) {
        let mut or_inner_elems = Vec::new();
        F::get_or_filters(es, &mut or_inner_elems);
        filter_elems.push(FilterElem::Or(or_inner_elems));
    }
    fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<OrFilterElem>) {
       let mut or_inner_elems = Vec::new();
       F::get_or_filters(es, &mut or_inner_elems); 
       filter_elems.push(OrFilterElem::Single(FilterElem::Or(or_inner_elems)));
    }
}

macro_rules! impl_query_filter_tuples {
    ($($t:ident), *) => {
        impl<$($t : QueryFilter), *> QueryFilter for ($($t),*,){
            fn get_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>) {
                $($t::get_filters(es, filter_elems);) *
            }
            fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<OrFilterElem>) {
                $(
                   let mut $t = Vec::new();
                   $t::get_filters(es, &mut $t);
                   filter_elems.push(OrFilterElem::And($t));
                ) *
            }
        }
    };
}

#[rustfmt::skip]
all_tuples!(
    impl_query_filter_tuples,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

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

    use crate::ecs::{component::Component, query::{Or, With, Without}, system::Res, world::World};

    use super::Query;

    struct Comp1(usize, usize);
    impl Component for Comp1 {}

    struct Comp2(usize, usize);
    impl Component for Comp2 {}

    fn test_system1(
        prm: Res<i32>,
        prm2: Res<usize>,
        mut query: Query<(&Comp1, &mut Comp2), (With<Comp1>, With<Comp1>,With<Comp1>,With<Comp1>,Or<(With<Comp1>,Or<(With<Comp1>,With<Comp1>)>, Without<Comp2>)>)>,
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
