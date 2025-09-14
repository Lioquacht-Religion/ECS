// system.rs

use std::{any::TypeId, cell::UnsafeCell, collections::HashMap, marker::PhantomData};

use crate::{
    all_tuples,
    ecs::{
        ecs_dependency_graph::{EcsEdge, QueryId},
        resource::ResourceId,
    },
    utils::ecs_id::{impl_ecs_id, EcsId},
};

use super::world::WorldData;

type StoredSystem = Box<dyn System>;

/*
 * Storage for systems.
 */
pub struct Systems {
    system_vec: Vec<StoredSystem>,
    system_param_data: HashMap<SystemId, Vec<SystemParamId>>,
}

#[derive(Debug)]
pub enum RefType {
    Shared,
    Exclusive,
    Owned,
}

#[derive(Debug)]
pub enum SystemParamId {
    Resource(ResourceId, RefType),
    Query(QueryId),
    NotRelevant,
}

impl Systems {
    pub fn new() -> Self {
        Systems {
            system_vec: Vec::new(),
            system_param_data: HashMap::new(),
        }
    }

    pub fn add_system<Input, S: System + 'static>(
        &mut self,
        value: impl IntoSystem<Input, System = S>
    ) -> SystemId {
        let next_id : SystemId = self.system_vec.len().into();
        let system = value.into_system();
        self.system_vec.push(Box::new(system));
        next_id.into()
    }

    pub fn init_systems(
        &mut self,
        world_data: &mut UnsafeCell<WorldData>
    ){
        for (system_id, system) in self.system_vec.iter_mut().enumerate(){
            let system_id : SystemId = system_id.into();
            let mut system_param_ids = Vec::new();
            system.init(system_id, &mut system_param_ids, world_data);
            self.system_param_data.insert(system_id, system_param_ids);
        }
    }

    pub fn run_systems(&mut self, world_data: &mut UnsafeCell<WorldData>) {
        for (i, sys) in self.system_vec.iter_mut().enumerate() {
            let system_id: SystemId = i.into();
            let system_params = self
                .system_param_data
                .get(&system_id)
                .expect("Invalid systems storage state!");
            sys.run(&system_params, world_data);
        }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct SystemId(u32);
impl_ecs_id!(SystemId);

pub trait System {
    fn init(&mut self, system_id: SystemId, system_param_ids: &mut Vec<SystemParamId>, world_data: &UnsafeCell<WorldData>);
    fn run(&mut self, system_param_ids: &[SystemParamId], world_data: &UnsafeCell<WorldData>);
}

pub trait SystemParam {
    type Item<'new>;

    unsafe fn retrieve<'r>(
        system_param_index: &mut usize,
        system_param_ids: &[SystemParamId],
        world_data: &'r UnsafeCell<WorldData>,
    ) -> Self::Item<'r>;
    fn create_system_param_data(
        system_id: SystemId,
        system_param_ids: &mut Vec<SystemParamId>,
        world_data: &UnsafeCell<WorldData>,
    );
}

pub struct Res<'a, T> {
    pub value: &'a T,
}

pub struct ResMut<'a, T> {
    pub value: &'a mut T,
}

pub struct ResOwned<T> {
    pub value: T,
}

impl<'res, T: 'static> SystemParam for Res<'res, T> {
    type Item<'new> = Res<'new, T>;

    unsafe fn retrieve<'r>(
        system_param_index: &mut usize,
        _system_param_ids: &[SystemParamId],
        world_data: &'r UnsafeCell<WorldData>,
    ) -> Self::Item<'r> {
        *system_param_index += 1;
        Res {
            value: (&*world_data.get()).resources.get().unwrap(),
        }
    }

    fn create_system_param_data(
        system_id: SystemId,
        system_param_ids: &mut Vec<SystemParamId>,
        world_data: &UnsafeCell<WorldData>,
    ) {
        let world_data = unsafe { &mut *world_data.get() };
        let resource_id = ResourceId::new(TypeId::of::<T>());
        world_data
            .entity_storage
            .depend_graph
            .insert_system_resource(system_id, resource_id, EcsEdge::Shared);
        system_param_ids.push(SystemParamId::Resource(
            ResourceId::new(TypeId::of::<T>()),
            RefType::Shared,
        ));
    }
}

impl<'res, T: 'static> SystemParam for ResMut<'res, T> {
    type Item<'new> = ResMut<'new, T>;

    unsafe fn retrieve<'r>(
        system_param_index: &mut usize,
        _system_param_ids: &[SystemParamId],
        world_data: &'r UnsafeCell<WorldData>,
    ) -> Self::Item<'r> {
        *system_param_index += 1;
        ResMut {
            value: (&mut *world_data.get()).resources.get_mut().unwrap(),
        }
    }

    fn create_system_param_data(
        system_id: SystemId,
        system_param_ids: &mut Vec<SystemParamId>,
        world_data: &UnsafeCell<WorldData>,
    ) {
        let world_data = unsafe { &mut *world_data.get() };
        let resource_id = ResourceId::new(TypeId::of::<T>());
        world_data
            .entity_storage
            .depend_graph
            .insert_system_resource(system_id, resource_id, EcsEdge::Excl);
        system_param_ids.push(SystemParamId::Resource(
            ResourceId::new(TypeId::of::<T>()),
            RefType::Exclusive,
        ));
    }
}

impl<T: 'static> SystemParam for ResOwned<T> {
    type Item<'new> = ResOwned<T>;

    unsafe fn retrieve<'r>(
        system_param_index: &mut usize,
        _system_param_ids: &[SystemParamId],
        world_data: &'r UnsafeCell<WorldData>,
    ) -> Self::Item<'r> {
        *system_param_index += 1;
        (*world_data.get()).resources.remove().unwrap()
    }
    fn create_system_param_data(
        system_id: SystemId,
        system_param_ids: &mut Vec<SystemParamId>,
        world_data: &UnsafeCell<WorldData>,
    ) {
        let world_data = unsafe { &mut *world_data.get() };
        let resource_id = ResourceId::new(TypeId::of::<T>());
        world_data
            .entity_storage
            .depend_graph
            .insert_system_resource(system_id, resource_id, EcsEdge::Owned);
        system_param_ids.push(SystemParamId::Resource(
            ResourceId::new(TypeId::of::<T>()),
            RefType::Owned,
        ));
    }
}

macro_rules! impl_systemparam_for_tuples {
    ( $($t:ident), * ) => {
       impl<$($t : SystemParam,)*> SystemParam for ($($t,)*){
          type Item<'new> = ($($t::Item<'new>,)*);

         unsafe fn retrieve<'r>(system_param_index: &mut usize, system_param_ids: &[SystemParamId], world_data: &'r UnsafeCell<WorldData>) -> Self::Item<'r> {
             (
               $(
                   $t::retrieve(system_param_index, system_param_ids, world_data),
               )*
             )
          }

         fn create_system_param_data(system_id: SystemId, system_param_ids: &mut Vec<SystemParamId>, world_data: &UnsafeCell<WorldData>){
             $(
                $t::create_system_param_data(system_id, system_param_ids, world_data);
             )*
         }
       }
    }
}

#[rustfmt::skip]
all_tuples!(
    impl_systemparam_for_tuples,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

pub struct FunctionSystem<Input, F> {
    pub f: F,
    pub marker: PhantomData<fn() -> Input>,
}

impl<F: FnMut()> System for FunctionSystem<(), F> {
    fn init(&mut self, _system_id: SystemId, _system_param_ids: &mut Vec<SystemParamId>, _world_data: &UnsafeCell<WorldData>){}
    fn run(&mut self, _system_params: &[SystemParamId], _world_data: &UnsafeCell<WorldData>) {
        (self.f)();
    }
}

macro_rules! impl_system_for_params {
    ( $($t:ident), * ) => {
       impl<F, $($t : SystemParam,)*> System for FunctionSystem<($($t,)*), F>
       where
         for<'a, 'b> &'a mut F : FnMut($($t,)*)
         + FnMut($(<$t as SystemParam>::Item<'b>,)*),
       {
           fn init(&mut self, system_id: SystemId, system_param_ids: &mut Vec<SystemParamId>, world_data: &UnsafeCell<WorldData>) {
              $( 
                $t::create_system_param_data(system_id, system_param_ids, world_data);
              )*
           }
           #[allow(non_snake_case)]
           fn run(&mut self, system_params: &[SystemParamId], world_data: &UnsafeCell<WorldData>){
               fn call_inner<$($t,)*>(
                   mut f: impl FnMut($($t,)*),
                   $( $t : $t,)*
               ){
                  f($( $t,)*)
               }
               let mut system_param_index = 0;
               $(let $t = unsafe{$t::retrieve(&mut system_param_index, system_params, world_data)};)*
               call_inner(&mut self.f, $($t,)* );
           }
       }
    };
}

#[rustfmt::skip]
all_tuples!(
    impl_system_for_params,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

pub trait IntoSystem<Input> {
    type System: System;

    fn into_system(self) -> Self::System;
}

impl<F: FnMut()> IntoSystem<()> for F {
    type System = FunctionSystem<(), Self>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            f: self,
            marker: Default::default(),
        }
    }
}

macro_rules! impl_into_system_for_functionsystem {
    ( $($t:ident), * ) => {
       impl<F: FnMut($($t,)*), $($t : SystemParam,)*> IntoSystem<($($t,)*)> for F
           where
             for<'a, 'b> &'a mut F:
             FnMut($($t,)*) + FnMut($(<$t as SystemParam>::Item<'b>,)*)
        {
           type System = FunctionSystem<( $($t,)* ), Self>;

           fn into_system(self) -> Self::System {
               FunctionSystem{
                   f : self,
                   marker : Default::default(),
               }
           }
        }
    }
}

#[rustfmt::skip]
all_tuples!(
    impl_into_system_for_functionsystem,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

#[cfg(test)]
mod test {
    use crate::ecs::{system::ResMut, world::World};

    use super::Res;

    fn test_system1(prm: Res<i32>, prm2: ResMut<usize>) {
        println!("testsystem1 res: {}, {}", prm.value, prm2.value);
        assert_eq!(2324, *prm.value);
        assert_eq!(4350, *prm2.value);
        *prm2.value += 999999999;
        assert_eq!(4350 + 999999999, *prm2.value);
    }

    #[test]
    fn it_works() {
        let mut world = World::new();
        let num1: i32 = 2324;
        let num2: usize = 4350;
        world.add_system(test_system1);
        world.add_resource(num1);
        world.add_resource(num2);
    }
}
