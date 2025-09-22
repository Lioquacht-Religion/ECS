// system.rs

use std::{
    any::TypeId,
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use crate::{
    all_tuples,
    ecs::{
        ecs_dependency_graph::{EcsEdge, QueryId},
        resource::ResourceId,
    },
    utils::ecs_id::{EcsId, impl_ecs_id},
};

use super::world::WorldData;

type StoredSystem = Box<dyn System>;

/// Storage for systems.
pub struct Systems {
    system_vec: Vec<StoredSystem>,
    func_system_map: HashMap<usize, SystemId>,
    constraints: HashMap<SystemId, Constraint>,
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

struct Constraint {
    system_id: SystemId,
    after: HashSet<SystemId>,
    before: HashSet<SystemId>,
}

pub struct SystemBuilder<I, IS: IntoSystem<I>, AS: SystemTuple, BS: SystemTuple> {
    into_system: IS,
    _input_marker: PhantomData<I>,
    after: Option<AS>,
    before: Option<BS>,
}

pub trait SystemTuple {
    fn get_system_ids(self, system_storage: &mut Systems, systems: &mut Vec<SystemId>);
}

impl SystemTuple for () {
    fn get_system_ids(self, _system_storage: &mut Systems, _systems: &mut Vec<SystemId>) {}
}

impl<IS: IntoSystem<()> + 'static> SystemTuple for IS {
    fn get_system_ids(self, system_storage: &mut Systems, systems: &mut Vec<SystemId>) {
        systems.push(system_storage.add_system(self));
    }
}

impl<IS1: IntoSystem<()> + 'static, IS2: IntoSystem<()> + 'static> SystemTuple for (IS1, IS2) {
    fn get_system_ids(self, system_storage: &mut Systems, systems: &mut Vec<SystemId>) {
        #[allow(non_snake_case)]
        let (IS1, IS2) = self;
        IS1::get_system_ids(IS1, system_storage, systems);
        IS2::get_system_ids(IS2, system_storage, systems);
    }
}

pub trait IntoSystemBuilder<I, IS>
where
    IS: IntoSystem<I>,
{
    type After: SystemTuple;
    type Before: SystemTuple;

    fn builder(self) -> SystemBuilder<I, IS, Self::After, Self::Before>;

    fn after<ST: SystemTuple>(self, systems: ST) -> impl IntoSystemBuilder<I, IS>
    where
        Self: Sized,
    {
        let SystemBuilder {
            into_system,
            _input_marker,
            after: _,
            before,
        } = self.builder();
        SystemBuilder {
            into_system,
            _input_marker,
            after: Some(systems),
            before,
        }
    }

    fn before<ST: SystemTuple>(self, systems: ST) -> impl IntoSystemBuilder<I, IS>
    where
        Self: Sized,
    {
        let SystemBuilder {
            into_system,
            _input_marker,
            after,
            before: _,
        } = self.builder();
        SystemBuilder {
            into_system,
            _input_marker,
            after,
            before: Some(systems),
        }
    }
}

impl<I, IS, ASI, BSI> IntoSystemBuilder<I, IS> for SystemBuilder<I, IS, ASI, BSI>
where
    IS: IntoSystem<I>,
    ASI: SystemTuple,
    BSI: SystemTuple,
{
    type After = ASI;
    type Before = BSI;
    fn builder(self) -> SystemBuilder<I, IS, Self::After, Self::Before> {
        self 
    }
}

impl<I, IS: IntoSystem<I>> IntoSystemBuilder<I, IS> for IS {
    type After = ();
    type Before = ();
    fn builder(self) -> SystemBuilder<I, IS, Self::After, Self::Before> {
        SystemBuilder {
            into_system: self,
            _input_marker: Default::default(),
            after: None,
            before: None,
        }
    }
}

impl Systems {
    pub fn new() -> Self {
        Systems {
            system_vec: Vec::new(),
            func_system_map: HashMap::new(),
            constraints: HashMap::new(),
            system_param_data: HashMap::new(),
        }
    }

    pub fn add_system<Input, S: System + 'static>(
        &mut self,
        value: impl IntoSystem<Input, System = S>,
    ) -> SystemId {
        let system = value.into_system();
        //TODO: handle none value
        let fn_ptr = system.get_fn_ptr().unwrap();
        if let Some(system_id) = self.func_system_map.get(&fn_ptr) {
            return *system_id;
        }
        let next_id: SystemId = self.system_vec.len().into();
        self.system_vec.push(Box::new(system));
        self.func_system_map.insert(fn_ptr, next_id);
        next_id.into()
    }

    pub fn add_system_builder<Input, S, IS, ISB: IntoSystemBuilder<Input, IS>>(
        &mut self,
        builder: ISB,
    ) -> SystemId
    where
        S: System + 'static,
        IS: IntoSystem<Input, System = S>,
    {
        //TODO:
        let SystemBuilder {
            into_system,
            _input_marker,
            after,
            before,
        } = builder.builder();
        self.add_system(into_system)
    }

    pub fn get_system(&mut self, system_id: SystemId) -> &mut dyn System {
        &mut *self.system_vec[system_id.id_usize()]
    }

    pub fn get_param_data(&mut self, system_id: SystemId) -> &[SystemParamId] {
        self.system_param_data
            .get(&system_id)
            .expect("Systems does not contain system params for system id.")
    }

    pub fn init_systems(&mut self, world_data: &mut UnsafeCell<WorldData>) {
        for (system_id, system) in self.system_vec.iter_mut().enumerate() {
            let system_id: SystemId = system_id.into();
            let mut system_param_ids = Vec::new();
            system.init(system_id, &mut system_param_ids, world_data);
            self.system_param_data.insert(system_id, system_param_ids);
        }
    }

    pub fn run_system(&mut self, system_id: SystemId, world_data: &UnsafeCell<WorldData>) {
        let _ = &mut self.system_vec[system_id.id_usize()]
            .run(self.system_param_data.get(&system_id).unwrap(), world_data);
    }

    pub fn run_systems(&mut self, world_data: &UnsafeCell<WorldData>) {
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
    fn init(
        &mut self,
        system_id: SystemId,
        system_param_ids: &mut Vec<SystemParamId>,
        world_data: &UnsafeCell<WorldData>,
    );
    fn run(&mut self, system_param_ids: &[SystemParamId], world_data: &UnsafeCell<WorldData>);
    fn get_fn_ptr(&self) -> Option<usize>;
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
        unsafe {
            Res {
                value: (&*world_data.get()).resources.get().unwrap(),
            }
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
        unsafe {
            ResMut {
                value: (&mut *world_data.get()).resources.get_mut().unwrap(),
            }
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
        unsafe { (*world_data.get()).resources.remove().unwrap() }
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

         unsafe fn retrieve<'r>(
             system_param_index: &mut usize,
             system_param_ids: &[SystemParamId],
             world_data: &'r UnsafeCell<WorldData>
         ) -> Self::Item<'r> {
             unsafe{
                 (
                   $(
                     $t::retrieve(system_param_index, system_param_ids, world_data),
                   )*
                 )
             }
          }

         fn create_system_param_data(
             system_id: SystemId,
             system_param_ids: &mut Vec<SystemParamId>,
             world_data: &UnsafeCell<WorldData>
         ){
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
    fn init(
        &mut self,
        _system_id: SystemId,
        _system_param_ids: &mut Vec<SystemParamId>,
        _world_data: &UnsafeCell<WorldData>,
    ) {
    }
    fn run(&mut self, _system_params: &[SystemParamId], _world_data: &UnsafeCell<WorldData>) {
        (self.f)();
    }
    fn get_fn_ptr(&self) -> Option<usize> {
        unsafe { Some(std::mem::transmute(&self.f)) }
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
           fn get_fn_ptr(&self) -> Option<usize> {
               unsafe { Some(std::mem::transmute(&self.f)) }
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

    use super::{IntoSystemBuilder, Res};

    fn test_system1(prm: Res<i32>, prm2: ResMut<usize>) {
        println!("testsystem1 res: {}, {}", prm.value, prm2.value);
        assert_eq!(2324, *prm.value);
        assert_eq!(4350, *prm2.value);
        *prm2.value += 999999999;
        assert_eq!(4350 + 999999999, *prm2.value);
    }

    fn test_system2() {}

    #[test]
    fn it_works() {
        let mut world = World::new();
        let num1: i32 = 2324;
        let num2: usize = 4350;
        let b = test_system2
            .after((test_system2, test_system2))
            //.before((test_system2, (test_system2, test_system2)));
            .before((test_system2, test_system2));
        world.add_system_builder(b);
        //world.add_system(test_system1);
        world.add_resource(num1);
        world.add_resource(num2);
    }
}
