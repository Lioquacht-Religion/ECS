// main.rs file for testing ECS package directly

use ecs::{
    ecs::{system::Res, world::World},
    utils::tuple_types::TupleTypesExt,
};

fn main() {
    let t = (0, 0, 0, 0, 0);

    t.self_layouts();

    let t1 = (0, 0);
    println!("layouts: {:?}", t1.self_layouts());
    let t2 = (0, 0, ("errf", "wsvfer"), 0, 0, (23, 232));
    println!("layouts: {:?}", t2.self_layouts());

    t1.self_type_ids();

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
    println!("testsystem3 tuple res: {}, {}, {}, {}", prm.0.value, prm.1.value, prm2.value.0, prm2.value.1);
}
