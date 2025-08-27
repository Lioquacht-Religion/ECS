// entity.rs

use std::sync::atomic::{self, AtomicU32};

use super::component::ArchetypeId;

struct Entry {
    entity: Option<Entity>,
    generation: u32,
}

pub(crate) struct Entities {
    vec: Vec<Entry>,
    empty_indices: Vec<u32>,
    free_indices_barrier: AtomicU32,
    empty_indices_barrier: AtomicU32,
}

impl Entities {
    pub(crate) fn new() -> Self {
        Self {
            vec: Vec::new(),
            empty_indices: Vec::new(),
            free_indices_barrier: AtomicU32::new(0),
            empty_indices_barrier: AtomicU32::new(0),
        }
    }

    pub(crate) fn get(&self, key: EntityKey) -> Option<&Entity> {
        if let Some(Entry {
            entity: Some(entity),
            generation,
        }) = self.vec.get(key.get_id() as usize)
        {
            if *generation == key.get_generation() {
                return Some(entity);
            }
        }
        None
    }

    pub(crate) fn get_mut(&mut self, key: EntityKey) -> Option<&mut Entity> {
        if let Some(Entry {
            entity: Some(entity),
            generation,
        }) = self.vec.get_mut(key.get_id() as usize)
        {
            if *generation == key.get_generation() {
                return Some(entity);
            }
        }
        None
    }

    pub(crate) fn insert(&mut self, entity: Entity) -> EntityKey {
        if let Some(empty_index) = self.empty_indices.pop() {
            let entry = &mut self.vec[empty_index as usize];
            entry.entity = Some(entity);
            entry.generation += 1;
            EntityKey::new(empty_index, entry.generation)
        } else {
            let next_id = self
                .vec
                .len()
                .try_into()
                .expect("Only ids in the range of an u32 are allowed!");
            self.vec.push(Entry {
                entity: Some(entity),
                generation: 0,
            });
            EntityKey::new(next_id, 0)
        }
    }

    pub(crate) fn remove(&mut self, key: EntityKey) -> Option<Entity> {
        match self.vec.get_mut(key.get_id() as usize) {
            Some(entry) if key.get_generation() == entry.generation => {
                let entity = entry.entity.take();
                entry.generation += 1;
                self.empty_indices.push(key.get_id());
                entity
            }
            _ => None,
        }
    }

    pub(crate) fn reserve(&self) -> EntityKey {
        let empty_barrier = self.empty_indices_barrier.load(atomic::Ordering::Relaxed) as usize;
        if empty_barrier < self.empty_indices.len() {
            let empty_barrier = self
                .empty_indices_barrier
                .fetch_add(1, atomic::Ordering::Relaxed) as usize;
            if empty_barrier < self.empty_indices.len() {
                let empty_barrier = self.empty_indices.len() - empty_barrier;
                let id = self.empty_indices[empty_barrier];
                let gen = *&self.vec[id as usize].generation;
                return EntityKey::new(id, gen);
            }
        }
        let free_barrier: usize = self
            .free_indices_barrier
            .fetch_add(1, atomic::Ordering::Relaxed) as usize;
        let id = free_barrier.try_into().expect("Above u32 max!");
        EntityKey::new(id, 0)
    }

    pub(crate) fn insert_with_reserved_key(
        &mut self,
        reserved_key: EntityKey,
        entity_to_insert: Entity,
    ) {
        match self.vec.get_mut(reserved_key.get_id() as usize) {
            Some(Entry { entity, generation })
                if entity.is_none() && *generation == reserved_key.generation =>
            {
                entity.replace(entity_to_insert);
            }
            _ => {}
        }
    }

    pub(crate) fn reset_barriers(&self) {
        self.empty_indices_barrier
            .store(0, atomic::Ordering::Relaxed);
        let len: u32 = self.vec.len().try_into().expect("Above u32 max!");
        self.free_indices_barrier
            .store(len, atomic::Ordering::Relaxed);
    }

    pub(crate) fn update_with_barriers(&mut self) {
        let empty_barrier = self.empty_indices_barrier.load(atomic::Ordering::Relaxed) as usize;
        let empty_barrier = empty_barrier.clamp(0, self.empty_indices.len());
        for _i in 0..empty_barrier {
            self.empty_indices.pop();
        }

        let free_barrier = self.free_indices_barrier.load(atomic::Ordering::Relaxed) as usize;
        let add_entries_count = free_barrier - self.vec.len();

        for _i in 0..add_entries_count {
            self.vec.push(Entry {
                entity: None,
                generation: 0,
            });
        }
        self.reset_barriers();
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct EntityId(u32);

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct EntityKey {
    pub(crate) id: u32,
    pub(crate) generation: u32,
}

impl EntityKey {
    pub(crate) fn new(id: u32, generation: u32) -> Self {
        Self { id, generation }
    }

    pub(crate) fn get_id(&self) -> u32 {
        self.id
    }

    pub(crate) fn get_generation(&self) -> u32 {
        self.generation
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct Entity {
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) row_id: u32,
}
