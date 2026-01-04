// builder.rs

use std::marker::PhantomData;

use crate::all_tuples_wout_single;

use super::{IntoSystem, System, SystemId, Systems};

pub trait IntoSystemTuple<I> {
    fn add_systems_to_stor(self, sys_stor: &mut Systems, system_ids: &mut Vec<SystemId>);
}

impl IntoSystemTuple<SystemId> for SystemId {
    fn add_systems_to_stor(self, _sys_stor: &mut Systems, system_ids: &mut Vec<SystemId>) {
        system_ids.push(self);
    }
}

impl<I, S: System + 'static, IS: IntoSystem<I, System = S> + 'static> IntoSystemTuple<I> for IS {
    fn add_systems_to_stor(self, sys_stor: &mut Systems, system_ids: &mut Vec<SystemId>) {
        system_ids.push(sys_stor.add_system(self));
    }
}

impl IntoSystemTuple<()> for () {
    fn add_systems_to_stor(self, _sys_stor: &mut Systems, _system_ids: &mut Vec<SystemId>) {}
}

macro_rules! impl_into_system_tuples {
    (($($t1:ident), *), ($($t2:ident), *)) => {
        impl<$($t2, $t1: IntoSystemTuple<$t2>), *>
            IntoSystemTuple<($($t2), *)> for ($($t1), *)
        {
            fn add_systems_to_stor(
                self,
                sys_stor: &mut Systems,
                system_ids: &mut Vec<SystemId>
            ) {
                 #[allow(non_snake_case)]
                let ($($t1), *) = self;
                $($t1::add_systems_to_stor($t1, sys_stor, system_ids)); *
            }
        }
    }
}

#[rustfmt::skip]
all_tuples_wout_single!(
    impl_into_system_tuples,
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16), 
    (I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15, I16)
);

pub struct SystemConfig<
    I,
    ST: IntoSystemTuple<I>,
    IA,
    AS: IntoSystemTuple<IA>,
    IB,
    BS: IntoSystemTuple<IB>,
> {
    pub(crate) system_tuple: ST,
    pub(crate) _marker: PhantomData<(I, IA, IB)>,
    pub(crate) chain: bool,
    pub(crate) after: Option<AS>,
    pub(crate) before: Option<BS>,
}

pub trait IntoSystemConfig<
    I,
    ST: IntoSystemTuple<I>,
    IA,
    AS: IntoSystemTuple<IA>,
    IB,
    BS: IntoSystemTuple<IB>,
>
{
    fn build(self) -> SystemConfig<I, ST, IA, AS, IB, BS>;
    fn chain(self) -> SystemConfig<I, ST, IA, AS, IB, BS>;
    fn after<I2, ST2: IntoSystemTuple<I2>>(
        self,
        after_systems: ST2,
    ) -> impl IntoSystemConfig<I, ST, I2, ST2, IB, BS>;
    fn before<I2, ST2: IntoSystemTuple<I2>>(
        self,
        before_systems: ST2,
    ) -> impl IntoSystemConfig<I, ST, IA, AS, I2, ST2>;
}

impl<I, ST: IntoSystemTuple<I>> IntoSystemConfig<I, ST, (), (), (), ()> for ST {
    fn build(self) -> SystemConfig<I, ST, (), (), (), ()> {
        SystemConfig {
            system_tuple: self,
            _marker: PhantomData::default(),
            chain: false,
            after: None,
            before: None,
        }
    }
    fn chain(self) -> SystemConfig<I, ST, (), (), (), ()> {
        SystemConfig {
            system_tuple: self,
            _marker: PhantomData::default(),
            chain: true,
            after: None,
            before: None,
        }
    }
    fn after<I2, ST2: IntoSystemTuple<I2>>(
        self,
        after_systems: ST2,
    ) -> impl IntoSystemConfig<I, ST, I2, ST2, (), ()> {
        SystemConfig {
            system_tuple: self,
            _marker: PhantomData::default(),
            chain: false,
            after: Some(after_systems),
            before: None,
        }
    }
    fn before<I2, ST2: IntoSystemTuple<I2>>(
        self,
        before_systems: ST2,
    ) -> impl IntoSystemConfig<I, ST, (), (), I2, ST2> {
        SystemConfig {
            system_tuple: self,
            _marker: PhantomData::default(),
            chain: false,
            after: None,
            before: Some(before_systems),
        }
    }
}

impl<I, ST: IntoSystemTuple<I>, IA, AS: IntoSystemTuple<IA>, IB, BS: IntoSystemTuple<IB>>
    IntoSystemConfig<I, ST, IA, AS, IB, BS> for SystemConfig<I, ST, IA, AS, IB, BS>
{
    fn build(self) -> SystemConfig<I, ST, IA, AS, IB, BS> {
        Self {
            system_tuple: self.system_tuple,
            _marker: PhantomData::default(),
            chain: self.chain,
            after: self.after,
            before: self.before,
        }
    }
    fn chain(self) -> SystemConfig<I, ST, IA, AS, IB, BS> {
        Self {
            system_tuple: self.system_tuple,
            _marker: PhantomData::default(),
            chain: true,
            after: self.after,
            before: self.before,
        }
    }
    fn after<I2, ST2: IntoSystemTuple<I2>>(
        self,
        after_systems: ST2,
    ) -> impl IntoSystemConfig<I, ST, I2, ST2, IB, BS> {
        SystemConfig {
            system_tuple: self.system_tuple,
            _marker: PhantomData::default(),
            chain: self.chain,
            after: Some(after_systems),
            before: self.before,
        }
    }
    fn before<I2, ST2: IntoSystemTuple<I2>>(
        self,
        before_systems: ST2,
    ) -> impl IntoSystemConfig<I, ST, IA, AS, I2, ST2> {
        SystemConfig {
            system_tuple: self.system_tuple,
            _marker: PhantomData::default(),
            chain: self.chain,
            after: self.after,
            before: Some(before_systems),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ecs::{
        commands::Commands,
        component::Component,
        query::Query,
        system::{Res, ResMut},
        world::World,
    };

    use super::IntoSystemConfig;

    fn test_system1(prm: Res<i32>, prm2: ResMut<usize>) {
        println!("testsystem1 res: {}, {}", prm.value, prm2.value);
        assert_eq!(2324, *prm.value);
        assert_eq!(4350, *prm2.value);
        *prm2.value += 999999999;
        assert_eq!(4350 + 999999999, *prm2.value);
    }

    struct Comp1();
    impl Component for Comp1 {}

    fn test_system2() {}
    fn test_system3() {}
    fn test_system4() {}
    fn test_system5() {}
    fn test_system6() {}
    fn test_system7() {}
    fn test_system8(#[allow(unused)] command: Commands, #[allow(unused)] query: Query<&mut Comp1>) {
    }

    #[test]
    fn test_system_scheduler_builder() {
        let mut world = World::new();

        let b = (test_system1, test_system2, test_system3)
            .after((test_system4, test_system7))
            .before((test_system6, test_system8))
            .chain();

        world.add_systems(b);

        world.add_systems(
            test_system5
                .after((test_system2, test_system3, test_system4))
                .before((test_system6, test_system8)),
        );

        let num1: i32 = 2324;
        let num2: usize = 4350;
        world.add_resource(num1);
        world.add_resource(num2);

        println!("system constraints: {:?}", &world.systems.constraints);

        world.init_and_run();
    }

    #[test]
    #[should_panic(expected = "System scheduling loop detected!")]
    fn test_system_scheduler_builder_infinite_loop_check() {
        let mut world = World::new();

        let b = (
            test_system7, //causes cyclic dependency
            test_system1, 
            test_system2, 
            test_system3
        ).chain();

        world.add_systems(b);

        world.add_systems(
            test_system5
                .after((test_system2, test_system3, test_system4))
                .before((
                    test_system6,
                    test_system7, //causes cyclic dependency
                    test_system8,
                )),
        );

        let num1: i32 = 2324;
        let num2: usize = 4350;
        world.add_resource(num1);
        world.add_resource(num2);

        println!("system constraints: {:?}", &world.systems.constraints);

        world.init_and_run();
    }
}
