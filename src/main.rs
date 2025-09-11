// main.rs file for testing ECS package directly

use ecs::ecs::{
    commands::Commands,
    component::{Component, StorageTypes},
    entity::EntityKey,
    query::{
        query_filter::{Or, With, Without},
        Query,
    },
    storages::entity_storage::EntityStorage,
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

struct PosAoS(i32);
impl Component for PosAoS {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

struct Pos2AoS(i32, i64);
impl Component for Pos2AoS {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

struct Pos3AoS(i32, i32, i32);
impl Component for Pos3AoS {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

struct Pos4AoS(i32, Box<Pos3>);
impl Component for Pos4AoS {
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
impl Component for Comp1AoS {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

struct Comp2AoS(usize, usize);
impl Component for Comp2AoS {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

fn test_system1(
    prm: Res<i32>,
    prm2: Res<usize>,
    mut query: Query<(&Comp1, &mut Comp2), Or<(Without<Pos4>, Without<Comp1>)>>,
    mut query2: Query<(&Pos, &mut Pos4, &Pos3), Or<(Without<Pos4>, Without<Comp1>)>>,
) {
    println!("testsystem1 res: {}, {}", prm.value, prm2.value);

    for (comp1, comp2) in query.iter() {
        println!("comp1: {}", comp1.0);
        println!("comp2: {}", comp2.0);
        comp2.0 = 2;
        println!("comp2: {}", comp2.0);
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
    mut commands: Commands,
    mut query: Query<
        (EntityKey, &mut Comp1, &mut Comp2), //, With<Pos4>
    >,
    mut query_aos: Query<
        (EntityKey, &mut Comp1AoS, &mut Comp2AoS),
        Or<(With<Pos4AoS>, With<Comp1AoS>)>,
    >,
) {
    let start1 = std::time::Instant::now();
    for (i, (entity, comp1, comp2)) in query.iter().enumerate() {
        comp1.0 /= 21;
        comp1.1 /= 437;
        comp2.0 /= 21;
        comp2.1 /= 437;

        println!(
            "soa iter: {i}; enitity key: {:?}; comp1: {}",
            entity, comp1.0
        );
    }

    let start2 = std::time::Instant::now();
    for (i, (entity, comp1, comp2)) in query_aos.iter().enumerate() {
        /*comp1.0 /= 21;
        comp1.1 /= 437;
        comp2.0 /= 21;
        comp2.1 /= 437;*/

        let _key = commands.spawn((Comp1(999999, 29029), Comp2(999999, 29029)));
        let _key = commands.spawn((Comp1(999999, 29029), Comp2(999999, 29029)));
        let _key = commands.spawn(Comp1(999999, 29029));

        comp1.0 /= 392049;
        comp1.1 /= 392049;
        comp2.0 /= 392049;
        comp2.1 /= 392049;

        commands.despawn(entity);

        println!(
            "aos iter: {i}; enitity key: {:?}; comp1: {}",
            entity, comp1.0
        );
    }

    let el1 = start1.elapsed();
    let el2 = start2.elapsed();
    println!(
        "soa time: {} nanos; {} micros",
        el1.as_nanos(),
        el1.as_micros()
    );
    println!(
        "aos time: {} nanos; {}, micros",
        el2.as_nanos(),
        el2.as_micros()
    );
}

static CAPACITY: usize = 10;

fn init_es_insert(es: &mut EntityStorage) {
    let start1 = std::time::Instant::now();
    for i in 0..CAPACITY {
        es.add_entity((Comp1(i, 34), Comp2(i, 34)));
        es.add_entity((
            Pos(33434),
            Comp1(i, 34),
            Pos4(12, Box::new(Pos3(1, 1, 1))),
            Comp2(i, 34),
            Pos2(232, 2423),
        ));
    }
    println!("insert time soa: {} nanos", start1.elapsed().as_nanos());
    let start2 = std::time::Instant::now();
    for i in 0..CAPACITY {
        es.add_entity((Comp1AoS(i, 34), Comp2AoS(i, 34)));
        es.add_entity((
            PosAoS(34434),
            Comp1AoS(i, 34),
            Pos4AoS(12, Box::new(Pos3(1, 1, 1))),
            Comp2AoS(i, 34),
            Pos2(2434, 23),
        ));
    }
    println!("insert time aos: {} nanos", start2.elapsed().as_nanos());
    println!("single inserts time: {} nanos", start1.elapsed().as_nanos());

    let start1 = std::time::Instant::now();
    let mut ents_soa = Vec::with_capacity(CAPACITY);
    let mut ents_soa2 = Vec::with_capacity(CAPACITY);
    for i in 0..CAPACITY {
        ents_soa2.push((Comp1(i, 34), Comp2(i, 34)));
        ents_soa.push((
            Pos(33434),
            Comp1(i, 34),
            Pos4(12, Box::new(Pos3(1, 1, 1))),
            Comp2(i, 34),
            Pos2(232, 2423),
        ));
    }
    es.add_entities_batch(ents_soa);
    es.add_entities_batch(ents_soa2);
    println!(
        "batch insert time soa: {} nanos",
        start1.elapsed().as_nanos()
    );

    let start2 = std::time::Instant::now();
    let mut ents_aos = Vec::with_capacity(CAPACITY);
    let mut ents_aos2 = Vec::with_capacity(CAPACITY);
    for i in 0..CAPACITY {
        ents_aos.push((Comp1AoS(i, 34), Comp2AoS(i, 34)));
        ents_aos2.push((
            PosAoS(34434),
            Comp1AoS(i, 34),
            Pos4AoS(12, Box::new(Pos3(1, 1, 1))),
            Comp2AoS(i, 34),
            Pos2(2434, 23),
        ));
    }

    let start3 = std::time::Instant::now();
    for c in ents_aos.iter_mut() {
        c.0 .0 /= 392049;
        c.0 .1 /= 392049;
        c.1 .0 /= 392049;
        c.1 .1 /= 392049;
    }
    for c in ents_aos2.iter_mut() {
        c.1 .0 /= 392049;
        c.1 .1 /= 392049;
        c.4 .0 /= 392049;
        c.4 .1 /= 392049;
    }
    println!(
        "normal loop time aos: {} nanos",
        start3.elapsed().as_nanos()
    );

    es.add_entities_batch(ents_aos);
    es.add_entities_batch(ents_aos2);
    println!(
        "batch insert time aos: {} nanos",
        start2.elapsed().as_nanos()
    );
    println!(
        "batch insert time: {} nanos; {} micros",
        start1.elapsed().as_nanos(),
        start1.elapsed().as_micros()
    );

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
    world.add_system(test_system1);
    world.add_system(test_system2);
    unsafe { (&mut *world.data.get()).add_resource(num1) };
    unsafe { (&mut *world.data.get()).add_resource(num2) };

    let es = &mut world.data.get_mut().entity_storage;

    init_es_insert(es);

    world.run();
    world.run();
    world.run();
    world.run();
    world.run();
}

fn main() {
    test_table_soa_query_iter();
    test_table_soa_insert();
}
