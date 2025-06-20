// query.rs

use std::{any::TypeId, cell::UnsafeCell, marker::PhantomData};

use super::{component::Component, system::SystemParam, world::WorldData};

pub struct Query<D: QueryParam> {
    marker: PhantomData<D>,
}

pub enum QueryDataInnerType {
    Single(TypeId),
    List(Vec<TypeId>),
}

impl<C: QueryParam> Query<C> {
    pub fn new() -> Self {
        Self {
            marker: Default::default(),
        }
    }
}

impl<D: QueryParam> SystemParam for Query<D> {
    type Item<'new> = Query<D>;
    unsafe fn retrieve<'r>(world_data: &'r UnsafeCell<WorldData>) -> Self::Item<'r> {
        Self::new()
    }
}

pub trait QueryParam {
    type Item: QueryParam;
}

impl<P1, P2> QueryParam for (P1, P2)
where
    P1: QueryParam,
    P2: QueryParam,
{
    type Item = (P1, P1);
}

impl<'p, T: Component> QueryParam for &'p T {
    type Item = &'p T;
}
impl<'p, T: Component> QueryParam for &'p mut T {
    type Item = &'p mut T;
}
//impl<'p, P: QueryParam> QueryParam for Option<P> {}

#[cfg(test)]
mod test {
    use crate::ecs::{component::Component, system::Res, world::World};

    use super::Query;

    struct Comp1(usize, usize);
    impl Component for Comp1 {}

    struct Comp2(usize, usize);
    impl Component for Comp2 {}

    fn test_system1(
        prm: Res<i32>,
        prm2: Res<usize>,
        query: Query<(&Comp1, &mut Comp2)>,
        query2: Query<&mut Comp1>,
    ) {
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
