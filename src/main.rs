// main.rs file for testing ECS package directly

use ecs::ecs::prelude::*;

#[allow(unused)]
struct Pos(i32);
impl Component for Pos {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Pos2(i32, i64);
impl Component for Pos2 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

#[allow(unused)]
struct Pos3(i32, i32, i32);
impl Component for Pos3 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

struct Pos4(i32, Box<Pos3>);
impl Component for Pos4 {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

#[allow(unused)]
struct PosAoS(i32);
impl Component for PosAoS {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

#[allow(unused)]
struct Pos2AoS(i32, i64);
impl Component for Pos2AoS {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

#[allow(unused)]
struct Pos3AoS(i32, i32, i32);
impl Component for Pos3AoS {
    const STORAGE: StorageTypes = StorageTypes::TableAoS;
}

#[allow(unused)]
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

#[inline(never)]
fn test_system1(
    mut commands: Commands,
    prm: Res<i32>,
    prm2: Res<usize>,
    mut query: Query<(&Comp1, &mut Comp2), Or<(Without<Pos4>, Without<Comp1>)>>,
    mut query2: Query<(EntityKey, &Pos, &mut Pos4, &Pos3), Or<(Without<Pos4>, Without<Comp1>)>>,
) {
    println!("testsystem1 res: {}, {}", prm.value, prm2.value);

    for (comp1, comp2) in query.iter() {
        //println!("comp1: {}", comp1.0);
        //println!("comp2: {}", comp2.0);
        comp2.0 = comp1.1 / 3245345;
        //println!("comp2: {}", comp2.0);
    }

    for (ek, _pos, pos4, _pos3) in query2.iter() {
        //println!("pos4 : {}", pos4.0);
        pos4.0 = 23234;
        pos4.0 -= 2344;
        //println!("pos4 : {}", pos4.0);

        //println!("pos4.1 box pointer: {}", pos4.1 .0);
        pos4.1.0 = 23234;
        pos4.1.0 -= 2344;
        //println!("pos4.1 box pointer: {}", pos4.1 .0);

        let _key = commands.spawn((Comp1AoS(999999, 29029), Comp2(999999, 29029)));
        let _key = commands.spawn((Comp1(999999, 29029), Comp2AoS(999999, 29029)));
        let _key = commands.spawn(Comp1(999999, 29029));

        commands.despawn(ek);
    }
}

#[inline(never)]
fn test_system2(
    mut query: Query<
        (&mut Comp1, &mut Comp2), //, With<Pos4>
    >,
    mut query_aos: Query<(&mut Comp1AoS, &mut Comp2AoS), Or<(With<Pos4AoS>, With<Comp1AoS>)>>,
) {
    let start1 = std::time::Instant::now();
    for (comp1, comp2) in query.iter() {
        comp1.0 /= 21;
        comp1.1 /= 437;
        comp2.0 /= 21;
        comp2.1 /= 437;

        /*
        println!(
            "soa iter: {i}; enitity key: {:?}; comp1: {}",
            entity, comp1.0
        );
        */
    }

    let start2 = std::time::Instant::now();
    for (comp1, comp2) in query_aos.iter() {
        comp1.0 /= 21;
        comp1.1 /= 437;
        comp2.0 /= 21;
        comp2.1 /= 437;

        comp1.0 /= 392049;
        comp1.1 /= 392049;
        comp2.0 /= 392049;
        comp2.1 /= 392049;

        /*
        println!(
            "aos iter: {i}; enitity key: {:?}; comp1: {}",
            entity, comp1.0
        );
        */
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

fn test_system3() {}

const CAPACITY: usize = 100_000;

fn init_es_insert(es: &mut World) {
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

fn test_system14() {}
fn test_system15() {}
fn test_system16() {}
fn test_system17() {}
fn test_system18() {}
fn test_system19() {}

fn test_system20() {}

fn test_system21() {}
fn test_system22() {}
fn test_system23() {}
fn test_system24() {}

#[inline(never)]
fn test_table_query_iter() {
    let mut world = World::new();
    let num1: i32 = 2324;
    let num2: usize = 2324;

    world.add_systems(test_system20);
    world.add_systems((test_system1, test_system2).chain());

    world.add_systems(test_system15.after(test_system14));
    world.add_systems(test_system3.after(test_system2));

    world.add_systems(test_system15.before(test_system18));

    world.add_systems(
        (
            test_system14,
            test_system15,
            test_system16,
            test_system17,
            test_system18,
            test_system19,
        )
            .chain(),
    );

    world.add_systems((test_system21, test_system22, test_system23, test_system24).chain());

    world.add_resource(num1);
    world.add_resource(num2);

    init_es_insert(&mut world);

    world.init_systems();
    world.run();
    /*
    world.run();
    world.run();
    world.run();
    world.run();
    */
}

fn main() {
    test_table_query_iter();
    normal_loop_test();
}

#[inline(never)]
fn normal_loop_test() {
    let mut ents = Vec::with_capacity(CAPACITY);
    let mut ents2 = Vec::with_capacity(CAPACITY);
    for i in 0..CAPACITY {
        ents.push((Comp1(i, 34), Comp2(i, 34)));
        ents2.push((
            Pos(33434),
            Comp1(i, 34),
            Pos4(12, Box::new(Pos3(1, 1, 1))),
            Comp2(i, 34),
            Pos2(232, 2423),
        ));
    }
    normal_loop(&mut ents, &mut ents2);
}

#[inline(never)]
fn normal_loop(ents: &mut Vec<(Comp1, Comp2)>, ents2: &mut Vec<(Pos, Comp1, Pos4, Comp2, Pos2)>) {
    let start3 = std::time::Instant::now();
    for c in ents.iter_mut() {
        c.0.0 /= 392049;
        c.0.1 /= 392049;
        c.1.0 /= 392049;
        c.1.1 /= 392049;
    }
    for c in ents2.iter_mut() {
        c.1.0 /= 392049;
        c.1.1 /= 392049;
        c.4.0 /= 392049;
        c.4.1 /= 392049;
    }
    println!(
        "normal loop time aos: {} micros; {} nanos",
        start3.elapsed().as_micros(),
        start3.elapsed().as_nanos()
    );
}
