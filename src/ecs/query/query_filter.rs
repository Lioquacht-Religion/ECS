// query_filter.rs

use std::{collections::HashSet, marker::PhantomData};

use crate::{
    all_tuples,
    ecs::{
        component::{Component, ComponentId},
        storages::entity_storage::EntityStorage,
    },
};

pub(crate) fn comp_ids_compatible_with_filter(
    comp_ids: &HashSet<ComponentId>,
    filter: &[FilterElem],
) -> bool {
    for el in filter.iter() {
        if !handle_filter_elem(comp_ids, el) {
            return false;
        }
    }
    true
}

fn handle_filter_elem(comp_ids: &HashSet<ComponentId>, filter_elem: &FilterElem) -> bool {
    match filter_elem {
        FilterElem::With(id) => comp_ids.contains(id),
        FilterElem::Without(id) => !comp_ids.contains(id),
        FilterElem::Or(or_elems) => handle_or_elems(comp_ids, or_elems),
    }
}

fn handle_or_elems(comp_ids: &HashSet<ComponentId>, or_elems: &[Vec<FilterElem>]) -> bool {
    for or_el in or_elems.iter() {
        if comp_ids_compatible_with_filter(comp_ids, &or_el) {
            return true;
        }
    }
    false
}

pub trait QueryFilter {
    fn get_and_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>);
    fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<Vec<FilterElem>>);
}

pub struct With<T: Component> {
    _marker: PhantomData<T>,
}
pub struct Without<T: Component> {
    _marker: PhantomData<T>,
}
pub struct Or<F: QueryFilter> {
    _marker: PhantomData<F>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum FilterElem {
    With(ComponentId),
    Without(ComponentId),
    Or(Vec<Vec<FilterElem>>),
}

impl QueryFilter for () {
    fn get_and_filters(_es: &mut EntityStorage, _filter_elems: &mut Vec<FilterElem>) {
        println!("empty filter: {:?};", &_filter_elems);
    }
    fn get_or_filters(_es: &mut EntityStorage, _filter_elems: &mut Vec<Vec<FilterElem>>) {
        println!("empty filter: {:?};", &_filter_elems);
    }
}
impl<T: Component> QueryFilter for With<T> {
    fn get_and_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>) {
        filter_elems.push(FilterElem::With(es.create_or_get_component::<T>()));

        println!("with filter: {:?};", &filter_elems);
    }
    fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<Vec<FilterElem>>) {
        filter_elems.push(vec![FilterElem::With(es.create_or_get_component::<T>())]);

        println!("with filter: {:?};", &filter_elems);
    }
}
impl<T: Component> QueryFilter for Without<T> {
    fn get_and_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>) {
        filter_elems.push(FilterElem::Without(es.create_or_get_component::<T>()));

        println!("tuple filter: {:?};", &filter_elems);
    }
    fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<Vec<FilterElem>>) {
        filter_elems.push(vec![FilterElem::Without(es.create_or_get_component::<T>())]);

        println!("tuple filter: {:?};", &filter_elems);
    }
}
impl<F: QueryFilter> QueryFilter for Or<F> {
    fn get_and_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>) {
        let mut or_inner_elems = Vec::new();
        F::get_or_filters(es, &mut or_inner_elems);
        filter_elems.push(FilterElem::Or(or_inner_elems));
    }
    fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<Vec<FilterElem>>) {
        let mut or_inner_elems = Vec::new();
        F::get_or_filters(es, &mut or_inner_elems);
        filter_elems.push(vec![FilterElem::Or(or_inner_elems)]);
    }
}

macro_rules! impl_query_filter_tuples {
    ($($t:ident), *) => {
        impl<$($t : QueryFilter), *> QueryFilter for ($($t),*,){
            fn get_and_filters(es: &mut EntityStorage, filter_elems: &mut Vec<FilterElem>) {

                $($t::get_and_filters(es, filter_elems);) *


                println!(
                    "tuple filter: {:?};",
                    &filter_elems
                );
            }
            fn get_or_filters(es: &mut EntityStorage, filter_elems: &mut Vec<Vec<FilterElem>>) {

                $(
                   #[allow(non_snake_case)]
                   let mut $t = Vec::new();
                   $t::get_and_filters(es, &mut $t);
                   filter_elems.push($t);
                ) *

                println!(
                    "tuple filter: {:?};",
                    &filter_elems
                );
            }
        }
    };
}

#[rustfmt::skip]
all_tuples!(
    impl_query_filter_tuples,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

#[cfg(test)]
mod test {
    use crate::ecs::{
        component::{Component, ComponentId},
        query::query_filter::{FilterElem, Or, QueryFilter, With},
        world::World,
    };

    struct Comp1(usize, usize);
    impl Component for Comp1 {}

    type FilterType1 = (
        With<Comp1>,
        With<Comp1>,
        Or<With<Comp1>>,
        Or<(With<Comp1>, With<Comp1>)>,
    );

    #[test]
    fn query_filters_test1() {
        let mut world = World::new();
        let es = &mut world.data.get_mut().entity_storage;
        let mut filter_elems = Vec::new();
        <FilterType1 as QueryFilter>::get_and_filters(es, &mut filter_elems);
        println!("query filters: {:?}", &filter_elems);
        filter_elems.clear();
        FilterType1::get_and_filters(es, &mut filter_elems);
        println!("query filters: {:?}", &filter_elems);
        let filter_cmp = vec![
            FilterElem::With(ComponentId(0)),
            FilterElem::With(ComponentId(0)),
            FilterElem::Or(vec![vec![FilterElem::With(ComponentId(0))]]),
            FilterElem::Or(vec![
                vec![FilterElem::With(ComponentId(0))],
                vec![FilterElem::With(ComponentId(0))],
            ]),
        ];
        assert_eq!(format!("{:?}", filter_elems), format!("{:?}", filter_cmp));
    }
}
