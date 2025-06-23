// main.rs file for testing ECS package directly

use std::usize;

use ecs::{
    ecs::{component::Component, storages::thin_blob_vec::ThinBlobVec, system::Res, world::World},
    utils::tuple_types::TupleTypesExt,
};

struct UsizeWrapper(usize);
impl Component for UsizeWrapper {}

#[derive(Debug)]
struct comp1(usize, usize, u8, u8);
impl Component for comp1 {}
struct comp3(usize, Box<comp1>);
impl Component for comp3 {}

fn main() {
    println!("test thin vec");
    test_thin_vec();
    println!("test table soa");
    test_table_soa();
}

use ecs::ecs::component::EntityStorage;

struct Pos(i32);
impl Component for Pos {}

struct Pos2(i32, i32);
impl Component for Pos2 {}

struct Pos3(i32, i32, i32);
impl Component for Pos3 {}

fn test_table_soa() {
    let mut es = EntityStorage::new();
    es.add_entity((Pos(12), Pos3(12, 34, 56)));
    es.add_entity((Pos3(12, 12, 34), Pos(56)));

    es.add_entity((Pos(12), Pos3(12, 34, 56), Pos2(213, 23)));
    es.add_entity((
        Pos(12),
        Pos3(12, 34, 56),
        comp3(12, Box::new(comp1(1, 1, 1, 1))),
    ));
}

fn test_thin_vec() {
    let mut bv = ThinBlobVec::new_typed::<comp1>();
    unsafe {
        let cap = bv.push_typed(0, 0, comp1(23, 435, 2, 5));
        let cap = bv.push_typed(cap, 1, comp1(23, 435, 2, 5));
        let cap = bv.push_typed(cap, 2, comp1(23, 435, 2, 5));
        let cap = bv.push_typed(cap, 3, comp1(23, 435, 2, 5));
        let cap = bv.push_typed(cap, 4, comp1(23, 435, 2, 5));
        let cap = bv.push_typed(cap, 5, comp1(23, 435, 2, 5));
        let _v: &mut comp1 = bv.get_mut_typed(0);
        let _v: &mut comp1 = bv.get_mut_typed(4);

        for c in bv.iter::<comp1>(10) {
            println!("{:?}", c);
        }

        println!("before alloc");

        bv.dealloc_typed::<comp1>(cap, 6);

        for c in bv.iter::<comp1>(10) {
            println!("{:?}", c);
        }

        drop(bv);
    }
}

fn main2() {
    let t = (UsizeWrapper(0), UsizeWrapper(0), UsizeWrapper(0));
    t.self_layouts();

    println!("layouts: {:?}", t.self_layouts());
    let t2 = (0, 0, ("errf", "wsvfer"), 0, 0, (23, 232));
    //println!("layouts: {:?}", t2.self_layouts());

    let t2: u32 = 0;
    let t3: () = ();

    t3.self_layouts();

    test(t2);

    //t2.self_type_ids();

    it_works_systems();
}

fn test(t: u32) -> u32 {
    let t2 = 3;
    t + t2
}

fn it_works_systems() {
    let mut world = World::new();
    let num1: i32 = 2324;
    let num2: usize = 57867;
    let num3: u64 = 2342454635;
    let num4: (usize, usize) = (432, 765);
    world.systems.add_system(test_system1);
    world.systems.add_system(test_system2);
    world.systems.add_system(test_system3);

    unsafe { (&mut *world.data.get()).add_resource(num1) };
    unsafe { (&mut *world.data.get()).add_resource(num2) };

    unsafe { (&mut *world.data.get()).add_resource(num3) };
    unsafe { (&mut *world.data.get()).add_resource(num4) };

    world.run();
}

fn test_system2() {
    println!("no params for this system");
}

fn test_system1(prm: Res<i32>, prm2: Res<usize>) {
    println!("testsystem1 res: {}, {}", prm.value, prm2.value);
}

fn test_system3(prm: (Res<i32>, Res<u64>), prm2: Res<(usize, usize)>) {
    println!(
        "testsystem3 tuple res: {}, {}, {}, {}",
        prm.0.value, prm.1.value, prm2.value.0, prm2.value.1
    );
}
