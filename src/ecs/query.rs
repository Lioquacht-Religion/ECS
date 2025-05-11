// query.rs

use std::{any::TypeId, cell::UnsafeCell, marker::PhantomData};

use super::{system::SystemParam, world::WorldData, Component};

pub struct Query<D : QueryData>{
    marker: PhantomData<D>,
}

pub enum QueryDataInnerType{
    Single(TypeId),
    List(Vec<TypeId>),
}

pub trait QueryData{
    fn get_type_ids() -> QueryDataInnerType;
}

impl<C: QueryData> Query<C>{
    pub fn new() -> Self{
        Self { marker: Default::default() }
    }
}

impl<D: QueryData> SystemParam for Query<D>{
    type Item<'new> = Query<D>;
    unsafe fn retrieve<'r>(world_data: &'r UnsafeCell<WorldData>) -> Self::Item<'r> {
        Self::new()
    }
}

impl<C : Component + 'static> QueryData for &C{
    fn get_type_ids() -> QueryDataInnerType {
        QueryDataInnerType::Single(TypeId::of::<C>())
    }
}

impl<C : Component + 'static> QueryData for &mut C{
    fn get_type_ids() -> QueryDataInnerType {
        QueryDataInnerType::Single(TypeId::of::<C>())
    }
}

impl<D : QueryData> QueryData for Option<D>{
    fn get_type_ids() -> QueryDataInnerType {
        D::get_type_ids()
    }
}

impl<T1 : QueryData, T2 : QueryData> QueryData for (T1, T2){
    fn get_type_ids() -> QueryDataInnerType {
        let mut type_ids = Vec::with_capacity(2);
        match T1::get_type_ids(){
            QueryDataInnerType::Single(single) => {type_ids.push(single)},
            QueryDataInnerType::List(mut list) => {type_ids.append(&mut list);},
        }
        match T2::get_type_ids(){
            QueryDataInnerType::Single(single) => {type_ids.push(single)},
            QueryDataInnerType::List(mut list) => {type_ids.append(&mut list);},
        } 
        QueryDataInnerType::List(type_ids)
    }

}

#[cfg(test)]
mod test {
    use crate::ecs::{system::Res, world::World, Component};

    use super::Query;

    struct Comp1(usize, usize);
    impl Component for Comp1{}

    struct Comp2(usize, usize);
    impl Component for Comp2{}

    fn test_system1(prm: Res<i32>, prm2: Res<usize>, query: Query<(&Comp1, &mut Comp2)>, query2: Query<&mut Comp1>) {
        println!("testsystem1 res: {}, {}", prm.value, prm2.value);
    }

    #[test]
    fn it_works() {
        let mut world = World::new();
        let num1: i32 = 2324;
        world.systems.add_system(test_system1);
        unsafe { (&mut *world.data.get()).add_resource(num1) };
    }
}
