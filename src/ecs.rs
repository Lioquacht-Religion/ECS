pub mod component;
pub mod entity;
pub mod query;
pub mod storages;
pub mod system;
pub mod world;

use std::{
    alloc::Layout,
    any::{Any, TypeId},
    collections::HashSet,
};

use crate::utils::{any_map::AnyMap, gen_vec::GenVec};

trait Component {}

struct ComponentInfo {
    layout: Layout,
    type_id: TypeId,
    name: String,
}

pub struct Entity {
    id: u32,
    genaration: u32,
}
pub struct Archetype {
    layout: Layout,
    components: HashSet<TypeId>,
}

struct System {}

pub struct World {
    entities: AnyMap,
    systems: AnyMap,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: AnyMap::new(),
            systems: AnyMap::new(),
        }
    }

    pub fn add_entity<T: Any>(&mut self, entity: T) {
        let ent_vec = self.entities.get_mut::<GenVec<T>>();
        if let Some(ent_vec) = ent_vec {
            ent_vec.insert(entity);
        } else {
            let mut gen_vec: GenVec<T> = GenVec::new();
            gen_vec.insert(entity);
            self.entities.insert(gen_vec);
        }
    }

    pub fn get_mut_entity_iter<T: Any>(&mut self) -> impl Iterator<Item = &mut T> {
        let ent_vec = self.entities.get_mut::<GenVec<T>>();
        ent_vec.unwrap().iter_mut()
    }

    pub fn add_system2<T: Any, F: Any + FnMut(T)>(&mut self, system: F) {
        self.systems.insert(system);
    }

    pub fn add_system<T: Any>(&mut self, system: Box<dyn FnMut(&mut T)>) {
        self.systems.insert(system);
    }

    pub fn execute_system<T: Any>(&mut self) {
        let entity_iter = self.entities.get_mut::<GenVec<T>>().unwrap().iter_mut();
        let system: &mut Box<dyn FnMut(&mut T)> =
            self.systems.get_mut::<Box<dyn FnMut(&mut T)>>().unwrap();
        for entity in entity_iter {
            system(entity);
        }
    }
}

struct Comp1(i32);
impl Component for Comp1 {}

struct Comp2(i32, i64, i64);
impl Component for Comp2 {}

fn test_system1(input: &mut (i32, i32, String)) {
    let (num1, num2, w) = input;
    println!("{}, {}, {}", num1, num2, w);
}

#[cfg(test)]
mod test {

    use super::{test_system1, Entity, World};

    #[test]
    fn test() {
        let mut world = World::new();
        let ent1: (i32, i32, String) = (2324, 24523, String::from("bla"));

        world.add_entity(ent1);

        world.add_system(Box::new(test_system1));

        for entity in world.get_mut_entity_iter::<(i32, i32, String)>() {
            test_system1(entity);
            let (num1, num2, word) = entity;
            num1.abs();

            word.push('u');
        }

        world.execute_system::<(i32, i32, String)>();
    }
}
