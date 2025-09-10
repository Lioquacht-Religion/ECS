// commands.rs

use std::cell::UnsafeCell;

use crate::{ecs::{entity::Entities, system::{SystemId, SystemParamId}}, utils::tuple_types::TupleTypesExt};

use super::{entity::EntityKey, system::SystemParam, world::WorldData};

type CommandQueue = Vec<Box<dyn Command>>;
type CommandQueueCell = Box<UnsafeCell<CommandQueue>>;
type CommandQueueVec = Vec<CommandQueueCell>;

pub(crate) struct CommandQueuesStorage {
    pub(crate) command_queues_unused: CommandQueueVec,
    pub(crate) command_queues_inuse: CommandQueueVec,
}

impl CommandQueuesStorage {
    pub(crate) fn new() -> Self {
        Self {
            command_queues_unused: Vec::new(),
            command_queues_inuse: Vec::new(),
        }
    }

    pub(crate) fn get_unused(&mut self) -> CommandQueueCell {
        if let Some(mut queue) = self.command_queues_unused.pop() {
            queue.get_mut().clear();
            queue
        } else {
            Box::new(UnsafeCell::new(Vec::new()))
        }
    }

    pub(crate) fn put_inuse(&mut self) -> *mut CommandQueue {
        let unused_queue = self.get_unused();
        let command_queue_ptr = unused_queue.get();
        self.command_queues_inuse.push(unused_queue);
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

impl<'w, 's> SystemParam for Commands<'w, 's> {
    type Item<'new> = Self;
    unsafe fn retrieve<'r>(_system_param_index: &mut usize, _system_param_ids: &[SystemParamId], world_data: &'r UnsafeCell<WorldData>) -> Self::Item<'r> {
        let world_data = &mut *world_data.get();
        let command_queue_ptr = world_data.commands_queues.put_inuse();

        //SAFETY: Because vec is stored behind box pointer on the heap,
        // it's address should be stable when moved.
        Commands::new(&world_data.entity_storage.entities, &mut *command_queue_ptr)
    }
    fn create_system_param_data(_system_id: SystemId, system_param_ids: &mut Vec<SystemParamId>, _world_data: &UnsafeCell<WorldData>) {
        system_param_ids.push(SystemParamId::NotRelevant);
    }
}

pub(crate) struct SpawnCommand<T: TupleTypesExt> {
    reserved_key: EntityKey,
    entity_value: T,
}

impl<T: TupleTypesExt> Command for SpawnCommand<T> {
    fn exec(self: Box<Self>, world_data: &mut WorldData) -> () {
        world_data
            .entity_storage
            .add_entity_with_reserved_key(self.reserved_key, self.entity_value);
    }
}

pub(crate) struct DespawnCommand {
    entity_key: EntityKey,
}

impl Command for DespawnCommand {
    fn exec(self: Box<Self>, world_data: &mut WorldData) -> () {
        world_data.entity_storage.remove_entity(self.entity_key);
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

    #[test]
    fn spawn_command_test() {
        todo!()
    }
}
