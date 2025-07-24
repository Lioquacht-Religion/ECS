// main.rs file for testing ECS package directly

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
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Pos3(i32, i32, i32);
impl Component for Pos3 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Pos4(i32, Box<Pos3>);
impl Component for Pos4 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Comp1(usize, usize);
impl Component for Comp1 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Comp2(usize, usize);
impl Component for Comp2 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

fn test_system1(
    prm: Res<i32>,
    prm2: Res<usize>,
    mut query: Query<(&Comp1, &mut Comp2)>,
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

    assert_eq!(count, 3);
    assert_eq!(query.iter().count(), 3);

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

    assert_eq!(query2.iter().count(), 4);
}

fn init_es_insert(es: &mut EntityStorage) {
    es.add_entity((Comp1(12, 34), Comp2(12, 34)));
    es.add_entity((Comp1(12, 34), Comp2(12, 34)));
    es.add_entity((Comp2(12, 34), Comp1(12, 34)));

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
