// commands.rs

use std::{cell::UnsafeCell, sync::Mutex};

use crate::{
    ecs::{
        entity::Entities,
        system::{SystemId, SystemParamId},
    },
    utils::tuple_types::TupleTypesExt,
};

use super::{entity::EntityKey, system::SystemParam, world::WorldData};

type CommandQueue = Vec<Box<dyn Command>>;
type CommandQueueCell = Box<UnsafeCell<CommandQueue>>;
type CommandQueueVec = Vec<CommandQueueCell>;

pub(crate) struct CommandQueuesStorage {
    pub(crate) command_queues_unused: Mutex<CommandQueueVec>,
    pub(crate) command_queues_inuse: Mutex<CommandQueueVec>,
}

impl CommandQueuesStorage {
    pub(crate) fn new() -> Self {
        Self {
            command_queues_unused: Mutex::new(Vec::new()),
            command_queues_inuse: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn get_unused(&self) -> CommandQueueCell {
        if let Some(mut queue) = self.command_queues_unused.lock().unwrap().pop() {
            queue.get_mut().clear();
            queue
        } else {
            Box::new(UnsafeCell::new(Vec::new()))
        }
    }

    pub(crate) fn put_inuse(&self) -> *mut CommandQueue {
        let unused_queue = self.get_unused();
        let command_queue_ptr = unused_queue.get();
        self.command_queues_inuse.lock().unwrap().push(unused_queue);
        command_queue_ptr
    }
}

pub trait Command<Out = ()> {
    fn exec(self: Box<Self>, world_data: &mut WorldData) -> Out;
}

pub struct Commands<'w, 's> {
    entities: &'w Entities,
    command_queue: &'s mut Vec<Box<dyn Command>>,
}

unsafe impl<'w, 's> Send for Commands<'w, 's> {}
unsafe impl<'w, 's> Sync for Commands<'w, 's> {}

impl<'w, 's> SystemParam for Commands<'w, 's> {
    type Item<'new> = Self;
    unsafe fn retrieve<'r>(
        system_param_index: &mut usize,
        _system_param_ids: &[SystemParamId],
        world_data: *mut WorldData,
    ) -> Self::Item<'r> {
        *system_param_index += 1;
        //TODO: access too command queue is not thread safe here
        unsafe {
            let command_queue_ptr = (*world_data).commands_queues.put_inuse();

            //SAFETY: Because vec is stored behind box pointer on the heap,
            // it's address should be stable if moved.
            Commands::new((*world_data).get_entities(), &mut *command_queue_ptr)
        }
    }
    fn create_system_param_data(
        _system_id: SystemId,
        system_param_ids: &mut Vec<SystemParamId>,
        _world_data: &mut WorldData,
    ) {
        system_param_ids.push(SystemParamId::NotRelevant);
    }
}

pub(crate) struct SpawnCommand<T: TupleTypesExt> {
    reserved_key: EntityKey,
    entity_value: T,
}

impl<T: TupleTypesExt> Command for SpawnCommand<T> {
    fn exec(self: Box<Self>, world_data: &mut WorldData) -> () {
        world_data.add_entity_with_reserved_key(self.reserved_key, self.entity_value);
    }
}

pub(crate) struct DespawnCommand {
    entity_key: EntityKey,
}

impl Command for DespawnCommand {
    fn exec(self: Box<Self>, world_data: &mut WorldData) -> () {
        world_data.remove_entity(self.entity_key);
    }
}

impl<'w, 's> Commands<'w, 's> {
    pub(crate) fn new(
        entities: &'w Entities,
        command_queue: &'s mut Vec<Box<dyn Command>>,
    ) -> Self {
        Self {
            entities,
            command_queue,
        }
    }

    pub fn spawn<T: TupleTypesExt>(&mut self, entity_value: T) -> EntityKey {
        let reserved_key = self.entities.reserve();
        self.command_queue.push(Box::new(SpawnCommand {
            reserved_key,
            entity_value,
        }));
        reserved_key
    }

    pub fn despawn(&mut self, entity_key: EntityKey) {
        self.command_queue
            .push(Box::new(DespawnCommand { entity_key }));
    }
}

#[cfg(test)]
mod test {
    use crate::ecs::{prelude::*, system::ResMut};

    struct Comp1SoA(u8, u16, u8, Box<(u8, u8, String)>, u8, String);
    impl Component for Comp1SoA {
        const STORAGE: StorageTypes = StorageTypes::TableSoA;
    }
    impl Default for Comp1SoA {
        fn default() -> Self {
            Self(
                43,
                555,
                250,
                Box::new((23, 66, "first_str".to_string())),
                32,
                "second_str".to_string(),
            )
        }
    }
    struct Comp2SoA(u8, u16, u8, Box<(u8, u8, String)>, u8, String);
    impl Component for Comp2SoA {
        const STORAGE: StorageTypes = StorageTypes::TableSoA;
    }
    impl Default for Comp2SoA {
        fn default() -> Self {
            Self(
                43,
                555,
                250,
                Box::new((23, 66, "first_str".to_string())),
                32,
                "second_str".to_string(),
            )
        }
    }

    struct Comp1AoS(u8, u16, u8, Box<(u8, u8, String)>, u8, String);
    impl Component for Comp1AoS {
        const STORAGE: StorageTypes = StorageTypes::TableAoS;
    }
    impl Default for Comp1AoS {
        fn default() -> Self {
            Self(
                43,
                555,
                250,
                Box::new((23, 66, "first_str".to_string())),
                32,
                "second_str".to_string(),
            )
        }
    }
    struct Comp2AoS(u8, u16, u8, Box<(u8, u8, String)>, u8, String);
    impl Component for Comp2AoS {
        const STORAGE: StorageTypes = StorageTypes::TableAoS;
    }
    impl Default for Comp2AoS {
        fn default() -> Self {
            Self(
                43,
                555,
                250,
                Box::new((23, 66, "first_str".to_string())),
                32,
                "second_str".to_string(),
            )
        }
    }

    struct EntityCount(isize);

    impl EntityCount {
        fn increase(&mut self) {
            self.0 += 1;
        }
        fn decrease(&mut self) {
            self.0 -= 1;
        }
    }

    fn test_system_spawn(
        mut commands: Commands,
        count: ResMut<EntityCount>,
        mut query_soa: Query<(&Comp1SoA, &Comp2SoA)>,
        mut query_aos: Query<(&Comp1AoS, &Comp2AoS)>,
    ) {
        let _entity_key = commands.spawn((Comp1SoA::default(), Comp2SoA::default()));
        count.value.increase();
        let _entity_key = commands.spawn(Comp1SoA::default());
        let _entity_key = commands.spawn(Comp2SoA::default());
        let _entity_key = commands.spawn((Comp1AoS::default(), Comp2AoS::default()));
        let _entity_key = commands.spawn(Comp1AoS::default());
        let _entity_key = commands.spawn(Comp2AoS::default());

        for (_c1, _c2) in query_soa.iter() {
            let _entity_key = commands.spawn((Comp1SoA::default(), Comp2SoA::default()));
            count.value.increase();
            let _entity_key = commands.spawn(Comp1SoA::default());
            let _entity_key = commands.spawn(Comp2SoA::default());
        }
        for (_c1, _c2) in query_aos.iter() {
            let _entity_key = commands.spawn((Comp1AoS::default(), Comp2AoS::default()));
            let _entity_key = commands.spawn(Comp1AoS::default());
            let _entity_key = commands.spawn(Comp2AoS::default());
        }
    }

    fn test_system_count_after_commands(
        count: ResMut<EntityCount>,
        mut query_soa: Query<(&Comp1SoA, &Comp2SoA)>,
    ) {
        //assert_eq!
        println!(
            "count compare: should: {}, is: {}",
            count.0,
            query_soa.iter().count()
        );
    }

    fn test_system_despawn_soa(
        mut commands: Commands,
        count: ResMut<EntityCount>,
        mut query_soa: Query<(EntityKey, &Comp1SoA, &Comp2SoA)>,
    ) {
        for (ek, _c1, _c2) in query_soa.iter() {
            commands.despawn(ek);
            count.value.decrease();
        }
    }

    fn test_system_despawn_aos(
        mut commands: Commands,
        mut query_aos: Query<(EntityKey, &Comp1AoS, &Comp2AoS)>,
    ) {
        for (ek, _c1, _c2) in query_aos.iter() {
            commands.despawn(ek);
        }
    }

    fn add_entities_to_world(world: &mut World) {
        world.add_entity((Comp1AoS::default(), Comp2AoS::default()));
        world.add_entity((Comp1AoS::default(), Comp2AoS::default()));
        world.add_entity((Comp1SoA::default(), Comp2SoA::default()));
        world.add_entity((Comp1SoA::default(), Comp2SoA::default()));
        world.add_resource(EntityCount(2));
    }

    #[test]
    fn command_spawn_test() {
        let mut world = World::new();
        world.add_systems((test_system_spawn, test_system_count_after_commands).chain());
        add_entities_to_world(&mut world);
        world.init_and_run();
        world.run();
        world.run();
        world.run();
        world.run();
        world.run();
        world.run();
    }

    #[test]
    fn command_despawn_soa_test() {
        let mut world = World::new();
        world.add_systems(
            (
                test_system_spawn,
                test_system_despawn_soa, //TODO:, test_system_count_after_commands
            )
                .chain(),
        );
        add_entities_to_world(&mut world);
        world.init_and_run();
        world.run();
    }

    #[test]
    fn command_despawn_aos_test() {
        let mut world = World::new();
        world.add_systems((test_system_spawn, test_system_despawn_aos).chain());
        add_entities_to_world(&mut world);
        world.init_and_run();
        world.run();
    }
}
