// main.rs file for testing ECS package directly

use std::time::{Duration, Instant};

use ecs::ecs::prelude::*;

#[allow(unused)]
struct Pos(i32);
impl Component for Pos {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
}

#[allow(unused)]
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
    mut query: Query<(&Comp1, &mut Comp2), Without<Pos4>>,
    mut query2: Query<(EntityKey, &Pos, &mut Pos4, &Pos2)>,
) {
    for (comp1, comp2) in query.iter() {
        comp2.0 = comp1.1 / 3245345 * prm.abs() as usize;
        comp2.1 = *prm2 / 7137;
    }

    for (ek, _pos, pos4, _pos3) in query2.iter() {
        pos4.0 = 23234;
        pos4.0 -= 2344;
        pos4.1.0 = 23234;
        pos4.1.0 -= 2344;

        let _key = commands.spawn((Comp1AoS(999999, 29029), Comp2(999999, 29029)));
        let _key = commands.spawn((Comp1(999999, 29029), Comp2AoS(999999, 29029)));
        let _key = commands.spawn(Comp1(999999, 29029));

        commands.despawn(ek);
    }
}

#[inline(never)]
fn test_system2(
    query_soa: Query<
        (&mut Comp1, &mut Comp2), //, With<Pos4>
    >,
    query_aos: Query<
        (&mut Comp1AoS, &mut Comp2AoS), //, Or<(With<Pos4AoS>, With<Comp1AoS>)>
    >,
) {
    let el1 = test_soa(query_soa);
    let el2 = test_aos(query_aos);

    /*
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
    */
}

#[inline(never)]
fn test_soa(mut query_soa: Query<(&mut Comp1, &mut Comp2)>) {
    for (comp1, comp2) in query_soa.iter() {
        do_some_work((comp1, comp2));
        /*
        println!(
            "soa iter: {i}; enitity key: {:?}; comp1: {}",
            entity, comp1.0
        );
        */
    }
}

#[inline(never)]
fn test_aos(mut query_aos: Query<(&mut Comp1AoS, &mut Comp2AoS)>) {
    for (comp1, comp2) in query_aos.iter() {
        do_some_work_aos((comp1, comp2));
        /*
        println!(
            "aos iter: {i}; enitity key: {:?}; comp1: {}",
            entity, comp1.0
        );
        */
    }
}

fn test_system3() {}

const CAPACITY: usize = 100_000;
const ITERATIONS: usize = 60;

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

    //world.add_systems(test_system20);
    //world.add_systems((test_system1).before((test_aos, test_soa)));
    world.add_systems((test_aos, test_soa).after((test_system21, test_system1)));
    //world.add_systems(test_aos);
    //world.add_systems(test_soa);

    /*
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
    */
    //world.add_systems((test_system21, test_system22, test_system23, test_system24).chain());

    world.add_resource(num1);
    world.add_resource(num2);

    init_es_insert(&mut world);

    world.init_systems();

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        world.run();
    }
    let end = start.elapsed();
    println!(
        "total run duration: {} nanos; {} millis; {} secs",
        end.as_nanos(),
        end.as_millis(),
        end.as_secs()
    );
}

fn main() {
    test_table_query_iter();
    normal_loop_test();
    normal_loop_test_soa();
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

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        normal_loop(&mut ents, &mut ents2);
    }
    let end = start.elapsed();
    println!(
        "normal loop aos total run duration: {} nanos; {} millis; {} secs",
        end.as_nanos(),
        end.as_millis(),
        end.as_secs()
    );
}

#[inline(never)]
fn normal_loop_test_soa() {
    let mut ents1 = Vec::with_capacity(CAPACITY);
    let mut ents2 = Vec::with_capacity(CAPACITY);
    let mut ents3 = Vec::with_capacity(CAPACITY);
    let mut ents4 = Vec::with_capacity(CAPACITY);
    for i in 0..CAPACITY {
        ents1.push(Comp1(i, 34));
        ents2.push(Comp2(i, 34));
        ents3.push(Comp1(i, 34));
        ents4.push(Comp2(i, 34));
    }
    let mut ents1 = (ents1, ents2);
    let mut ents2 = (ents3, ents4);
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        normal_loop_soa(&mut ents1, &mut ents2);
    }
    let end = start.elapsed();
    println!(
        "normal loop soa total run duration: {} nanos; {} millis; {} secs",
        end.as_nanos(),
        end.as_millis(),
        end.as_secs()
    );
}

#[inline(never)]
fn normal_loop_soa(ents: &mut (Vec<Comp1>, Vec<Comp2>), ents2: &mut (Vec<Comp1>, Vec<Comp2>)) {
    let (c1, c2) = ents;
    let (mut c1, mut c2) = (c1.iter_mut(), c2.iter_mut());
    while let (Some(c1), Some(c2)) = (c1.next(), c2.next()) {
        let c = (c1, c2);
        do_some_work(c);
    }
    let (c1, c2) = ents2;
    let (mut c1, mut c2) = (c1.iter_mut(), c2.iter_mut());
    while let (Some(c1), Some(c2)) = (c1.next(), c2.next()) {
        let c = (c1, c2);
        do_some_work(c);
    }
}

#[inline(never)]
fn normal_loop(ents: &mut Vec<(Comp1, Comp2)>, ents2: &mut Vec<(Pos, Comp1, Pos4, Comp2, Pos2)>) {
    let start3 = std::time::Instant::now();
    for c in ents.iter_mut() {
        do_some_work((&mut c.0, &mut c.1));
    }
    for c in ents2.iter_mut() {
        do_some_work((&mut c.1, &mut c.3));
    }
    let e3 = start3.elapsed();
    let sum: usize = ents
        .iter()
        .fold(0, |a, e| a + e.0.0 + e.0.1 + e.1.0 + e.1.1)
        + ents2
            .iter()
            .fold(0, |a, e| a + e.1.0 + e.1.1 + e.3.0 + e.3.1);
    /*println!("normal loop aos sum : {sum}");
    println!(
        "normal loop time aos: {} micros; {} nanos",
        e3.as_micros(),
        e3.as_nanos()
    );*/
}
fn do_some_work_aos(c: (&mut Comp1AoS, &mut Comp2AoS)) {
    c.0.0 /= 21;
    c.0.1 /= 437;
    c.1.0 /= 21;
    c.1.1 /= 437;

    c.0.0 /= 392049;
    c.0.1 /= 392049;
    c.1.0 /= 392049;
    c.1.1 /= 392049;

    c.0.0 *= c.1.1;
    c.0.1 *= c.1.0;
    c.1.0 += c.0.1;
    c.1.1 += c.0.0;
}
fn do_some_work(mut c: (&mut Comp1, &mut Comp2)) {
    do_some_work1(&mut c);
    do_some_work2(&mut c);
    do_some_work3(&mut c);
}
fn do_some_work1(c: &mut (&mut Comp1, &mut Comp2)) {
    c.0.0 /= 21;
    c.0.1 /= 437;
    c.1.0 /= 21;
    c.1.1 /= 437;
}

fn do_some_work2(c: &mut (&mut Comp1, &mut Comp2)) {
    c.0.0 /= 392049;
    c.0.1 /= 392049;
    c.1.0 /= 392049;
    c.1.1 /= 392049;
}

fn do_some_work3(c: &mut (&mut Comp1, &mut Comp2)) {
    c.0.0 *= c.1.1;
    c.0.1 *= c.1.0;
    c.1.0 += c.0.1;
    c.1.1 += c.0.0;
}
