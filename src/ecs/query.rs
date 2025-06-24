// query.rs

use std::{any::TypeId, cell::UnsafeCell, collections::HashSet, marker::PhantomData};

use crate::{
    all_tuples,
    utils::{
        sorted_vec::SortedVec,
        tuple_iters::{TableSoaTupleIter, TupleIterConstructor, TupleIterator},
    },
};

use super::{
    component::{ArchetypeId, Component, ComponentId},
    system::SystemParam,
    world::WorldData,
};

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

    pub fn iter(&mut self) -> QueryIter<P> {
        QueryIter::new(self)
    }

    unsafe fn get_arch_query_iter(
        &self,
        arch_id: ArchetypeId,
    ) -> TableSoaTupleIter<<P as TupleIterConstructor>::Construct<'w>> {
        self.world
            .get()
            .as_mut()
            .unwrap()
            .entity_storage
            .tables_soa
            .get_mut(&arch_id)
            .expect("Table SoA with archetype id could not be found.")
            .tuple_iter::<P>()
    }
}

pub struct QueryIter<'w, 's, T: QueryParam> {
    query: &'w Query<'w, 's, T>,
    cur_arch_query: TableSoaTupleIter<T::Construct<'w>>,
    cur_arch_index: usize,
}

impl<'w, 's, T: QueryParam> QueryIter<'w, 's, T> {
    pub fn new(query: &'w Query<'w, 's, T>) -> Self {
        let arch_id = query.state.arch_ids[0];

        let arch_query = unsafe{query.get_arch_query_iter(arch_id)}; 

        Self {
            query,
            cur_arch_query: arch_query,
            cur_arch_index: 0,
        }
    }
}

impl<'w, 's, T: QueryParam> Iterator for QueryIter<'w, 's, T> {
    type Item = <<T as TupleIterConstructor>::Construct<'w> as TupleIterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.cur_arch_query.next() {
            return Some(item);
        }
        self.cur_arch_index += 1;
        if self.cur_arch_index < self.query.state.arch_ids.len() {
            let arch_id = self.query.state.arch_ids[self.cur_arch_index];
            self.cur_arch_query = unsafe { self.query.get_arch_query_iter(arch_id) };
            if let Some(item) = self.cur_arch_query.next() {
                return Some(item);
            }
            // recurse until next Some item is returned
            self.next() 
        } else {
            None
        }
    }
}

impl<'w, 's, P: QueryParam> SystemParam for Query<'w, 's, P> {
    type Item<'new> = Query<'new, 's, P>;
    unsafe fn retrieve<'r>(world_data: &'r UnsafeCell<WorldData>) -> Self::Item<'r> {
        let world_data_ref = world_data.get().as_ref().unwrap();
        let mut comp_ids = Vec::new();
        P::comp_ids_rec(world_data_ref, &mut comp_ids);
        let comp_ids: SortedVec<ComponentId> = comp_ids.into();
        if let Some(query_data) = world_data_ref.query_data.get(&comp_ids) {
            return Self::Item::<'r>::new(world_data, query_data);
        }

        let comp_ids: SortedVec<ComponentId> = comp_ids.into();

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

pub trait QueryParam: TupleIterConstructor {
    type QueryItem<'new>: QueryParam;

    fn type_ids_rec(vec: &mut Vec<TypeId>);
    fn comp_ids_rec(world_data: &WorldData, vec: &mut Vec<ComponentId>);
    fn ref_kinds(vec: &mut Vec<RefKind>);
}

impl<T: Component> QueryParam for &T {
    type QueryItem<'new> = &'new T;

    fn type_ids_rec(vec: &mut Vec<TypeId>) {
        vec.push(TypeId::of::<T>());
    }
    fn comp_ids_rec(world_data: &WorldData, vec: &mut Vec<ComponentId>) {
        let comp_id = world_data
            .entity_storage
            .typeid_compid_map
            .get(&TypeId::of::<T>())
            .expect("No component id found for type id.");
        vec.push(*comp_id);
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
    fn comp_ids_rec(world_data: &WorldData, vec: &mut Vec<ComponentId>) {
        let comp_id = world_data
            .entity_storage
            .typeid_compid_map
            .get(&TypeId::of::<T>())
            .expect("No component id found for type id.");
        vec.push(*comp_id);
    }
    fn ref_kinds(vec: &mut Vec<RefKind>) {
        vec.push(RefKind::Exclusive);
    }
}
//impl<'p, P: QueryParam> QueryParam for Option<P> {}

macro_rules! impl_query_param_tuples {
    ($($t:ident), *) => {
       impl<$($t : QueryParam), *> QueryParam for ($($t),*,){
           #[allow(unused_parens)]
               type QueryItem<'new> = ($($t),* );

               fn type_ids_rec(vec: &mut Vec<TypeId>){
                   $($t::type_ids_rec(vec);)*
               }
               fn comp_ids_rec(world_data: &WorldData, vec: &mut Vec<ComponentId>) {
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
        world.data.get_mut().entity_storage.add_entity((Comp1(12,34), Comp2(56, 78)));
        world.run();
    }
}
