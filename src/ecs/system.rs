// system.rs

use std::{
    any::TypeId,
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use builder::{IntoSystemConfig, IntoSystemTuple, SystemConfig};

use crate::{
    all_tuples,
    ecs::{
        ecs_dependency_graph::{EcsEdge, QueryId},
        resource::ResourceId,
    },
    utils::ecs_id::{EcsId, impl_ecs_id},
};

use super::world::WorldData;

pub mod builder;

type StoredSystem = Box<dyn System>;

/// Storage for systems.
pub struct Systems {
    pub(crate) system_vec: Vec<StoredSystem>,
    func_system_map: HashMap<TypeId, SystemId>,
    pub(crate) constraints: HashMap<SystemId, Constraint>,
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

#[derive(Debug)]
pub(crate) struct Constraint {
    #[allow(unused)]
    pub(crate) system_id: SystemId,
    pub(crate) after: HashSet<SystemId>,
    pub(crate) before: HashSet<SystemId>,
}

impl Constraint {
    pub(crate) fn new(system_id: SystemId) -> Self {
        Self {
            system_id,
            after: HashSet::new(),
            before: HashSet::new(),
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

    pub fn add_system<Input, S: System + 'static, IS: IntoSystem<Input, System = S> + 'static>(
        &mut self,
        value: IS,
    ) -> SystemId {
        let config_id = TypeId::of::<IS>();
        self.add_system_inner(value, config_id)
    }

    pub fn add_system_inner<Input, S: System + 'static>(
        &mut self,
        value: impl IntoSystem<Input, System = S>,
        sys_config_id: TypeId,
    ) -> SystemId {
        let system = value.into_system();
        if let Some(system_id) = self.func_system_map.get(&sys_config_id) {
            return *system_id;
        }
        let next_id: SystemId = self.system_vec.len().into();
        self.system_vec.push(Box::new(system));
        self.func_system_map.insert(sys_config_id, next_id);
        next_id.into()
    }

    pub fn add_system_builder<
        I,
        ST: IntoSystemTuple<I>,
        IA,
        AS: IntoSystemTuple<IA>,
        IB,
        BS: IntoSystemTuple<IB>,
    >(
        &mut self,
        value: impl IntoSystemConfig<I, ST, IA, AS, IB, BS>,
    ) -> Vec<SystemId> {
        let SystemConfig {
            system_tuple,
            _marker,
            chain,
            after,
            before,
        } = value.build();
        let mut system_ids = Vec::new();
        system_tuple.add_systems_to_stor(self, &mut system_ids);

        let after: Vec<SystemId> = if let Some(after) = after {
            let mut system_ids = Vec::new();
            after.add_systems_to_stor(self, &mut system_ids);
            system_ids
        } else {
            Vec::new()
        };

        let before: Vec<SystemId> = if let Some(before) = before {
            let mut system_ids = Vec::new();
            before.add_systems_to_stor(self, &mut system_ids);
            system_ids
        } else {
            Vec::new()
        };

        for system_id in system_ids.iter() {
            self.add_system_constraints(*system_id, &after, &before);
        }

        if chain {
            if system_ids.len() > 1 {
                let sysid = system_ids[0];
                let before_sysid = system_ids[1];
                self.add_system_constraints(sysid, &[], &[before_sysid]);
                let mut ind = 1;
                for i in 1..(system_ids.len() - 1) {
                    let after_sysid = system_ids[i - 1];
                    let sysid = system_ids[i];
                    let before_sysid = system_ids[i + 1];
                    self.add_system_constraints(sysid, &[after_sysid], &[before_sysid]);
                    ind = i;
                }
                if system_ids.len() > ind {
                    let after_sysid = system_ids[ind - 1];
                    let sysid = system_ids[ind];
                    self.add_system_constraints(sysid, &[after_sysid], &[]);
                }
            }
        }

        system_ids
    }

    fn add_system_constraints(
        &mut self,
        system_id: SystemId,
        after: &[SystemId],
        before: &[SystemId],
    ) -> SystemId {
        fn insert_after(constraint: &mut Constraint, system_id: SystemId) {
            constraint.after.insert(system_id);
        }
        fn insert_before(constraint: &mut Constraint, system_id: SystemId) {
            constraint.before.insert(system_id);
        }

        self.insert_constraints(system_id, after, insert_after, insert_before);
        self.insert_constraints(system_id, before, insert_before, insert_after);
        system_id
    }

    fn insert_constraints(
        &mut self,
        system_id: SystemId,
        con_sys_ids: &[SystemId],
        mut insert_system_constraints: impl FnMut(&mut Constraint, SystemId),
        mut insert_constraint_constraints: impl FnMut(&mut Constraint, SystemId),
    ) {
        let constraint = self
            .constraints
            .entry(system_id)
            .or_insert(Constraint::new(system_id));
        for csys in con_sys_ids.iter() {
            insert_system_constraints(constraint, *csys);
        }
        for csys in con_sys_ids.iter() {
            let constraint = self
                .constraints
                .entry(*csys)
                .or_insert(Constraint::new(*csys));
            insert_constraint_constraints(constraint, system_id);
        }
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

/*
       impl<
       F: FnMut(T1, T2), T1 : SystemParam, T2: SystemParam>
       IntoSystem<(T1, T2)> for F
           where
             for<'a, 'b> &'a mut F:
             FnMut(T1, T2) + FnMut(<T1 as SystemParam>::Item<'b>, <T2 as SystemParam>::Item<'b>)
        {
           type System = FunctionSystem<(T1, T2), Self>;

           fn into_system(self) -> Self::System {
               FunctionSystem{
                   f : self,
                   marker : Default::default(),
               }
           }
        }
*/

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

    fn test_system2() {}
    fn test_system3() {}
    fn test_system4() {}
    fn test_system5() {}
    fn test_system6() {}
    fn test_system7() {}
    fn test_system8() {}

    #[test]
    fn it_works() {
        let mut world = World::new();
        let num1: i32 = 2324;
        let num2: usize = 4350;

        world.add_system_builder(test_system1);
        world.add_resource(num1);
        world.add_resource(num2);

        world.init_and_run();
    }
}
