// main.rs file for testing ECS package directly

use std::time::Duration;

use ecs::ecs::{
    component::{Component, EntityStorage, StorageTypes},
    query::Query,
    system::Res,
    world::World,
};

struct Pos(i32);
impl Component for Pos {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Pos2(i32, i64);
impl Component for Pos2 {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

struct Pos3(i32, i32, i32);
impl Component for Pos3 {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

struct Pos4(i32, Box<Pos3>);
impl Component for Pos4 {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

struct Comp1(usize, usize);
impl Component for Comp1 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Comp2(usize, usize);
impl Component for Comp2 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Comp1AoS(usize, usize);
impl Component for Comp1AoS{
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

struct Comp2AoS(usize, usize);
impl Component for Comp2AoS{
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

fn test_system1(
    prm: Res<i32>,
    prm2: Res<usize>,
    mut query: Query<(&Comp1, &mut Comp2)>,
    mut query_aos: Query<(&Comp1AoS, &mut Comp2AoS)>,
    mut query2: Query<(&Pos, &mut Pos4, &Pos3)>,
) {
    println!("testsystem1 res: {}, {}", prm.value, prm2.value);

    let mut count = 0;
    for (comp1, comp2) in query.iter() {
        println!("comp1: {}", comp1.0);
        println!("comp2: {}", comp2.0);
        comp2.0 = 2;
        println!("comp2: {}", comp2.0);
        count += 1;
    }

    for (_pos, pos4, _pos3) in query2.iter() {
        println!("pos4 : {}", pos4.0);
        pos4.0 = 23234;
        pos4.0 -= 2344;
        println!("pos4 : {}", pos4.0);

        println!("pos4.1 box pointer: {}", pos4.1 .0);
        pos4.1 .0 = 23234;
        pos4.1 .0 -= 2344;
        println!("pos4.1 box pointer: {}", pos4.1 .0);
    }
}

fn test_system2(
    mut query: Query<(&mut Comp1, &mut Comp2)>,
    mut query_aos: Query<(&mut Comp1AoS, &mut Comp2AoS)>,
) {
    let start1 = std::time::Instant::now();
    for (comp1, comp2) in query.iter() {
        comp1.0 /= 21;
        comp1.1 /= 437;
        comp2.0 /= 21;
        comp2.1 /= 437;
    }

    let start2 = std::time::Instant::now();
    for (comp1, comp2) in query_aos.iter() {
        comp1.0 /= 21;
        comp1.1 /= 437;
        comp2.0 /= 21;
        comp2.1 /= 437;
    }

    println!("soa time: {} nanos", start1.elapsed().as_nanos());
    println!("aos time: {} nanos", start2.elapsed().as_nanos());


}



fn init_es_insert(es: &mut EntityStorage) {

    for i in 0..100000{
        es.add_entity((Comp1(i, 34), Comp2(i, 34)));
        es.add_entity((Comp1AoS(i, 34), Comp2AoS(i, 34)));
    }

/*
    es.add_entity((Comp1(12, 34), Comp2(12, 34)));
    es.add_entity((Comp1(12, 34), Comp2(12, 34)));
    es.add_entity((Comp2(12, 34), Comp1(12, 34)));

    es.add_entity((Comp2(12, 34), Comp1(12, 34), Pos4(12, Box::new(Pos3(1, 1, 1)))));
    es.add_entity((Comp2(12, 34), Comp1(12, 34), Pos(76)));
    es.add_entity((Comp2(12, 34), Comp1(12, 34), Pos4(12, Box::new(Pos3(1, 1, 1)))));
    es.add_entity((Comp2(12, 34), Comp1(12, 34), Pos4(12, Box::new(Pos3(1, 1, 1)))));
    es.add_entity((Comp2(12, 34), Comp1(12, 34), Pos(76)));
    es.add_entity((Comp2(12, 34), Comp1(12, 34), Pos(76)));
*/

    es.add_entity((Pos(12), Pos3(12, 34, 56)));
    es.add_entity((Pos3(12, 12, 34), Pos(56)));
    es.add_entity((Pos(12), Pos3(12, 34, 56), Pos2(213, 23)));
    es.add_entity((Pos2(213, 23), Pos(12)));
    es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
    es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
    es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
    es.add_entity((Pos(12), Pos3(12, 34, 56), Pos4(12, Box::new(Pos3(1, 1, 1)))));
}

fn test_table_soa_insert() {
    let mut es = EntityStorage::new();
    init_es_insert(&mut es);
}

fn test_table_soa_query_iter() {
    let mut world = World::new();
    let num1: i32 = 2324;
    let num2: usize = 2324;
    world.systems.add_system(test_system1);
    world.systems.add_system(test_system2);
    unsafe { (&mut *world.data.get()).add_resource(num1) };
    unsafe { (&mut *world.data.get()).add_resource(num2) };

    let es = &mut world.data.get_mut().entity_storage;

    init_es_insert(es);

    world.run();
}

fn main() {
    test_table_soa_query_iter();
    test_table_soa_insert();
}
