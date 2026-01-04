// component.rs

use std::{
    alloc::Layout, any::TypeId, borrow::Cow, collections::HashMap, hash::Hash, mem::needs_drop,
    ptr::drop_in_place, u32, usize,
};

use crate::{
    ecs::{entity::EntityKey, world::WorldData},
    utils::{
        ecs_id::{impl_ecs_id, EcsId},
        sorted_vec::SortedVec,
    },
};

pub type Map<K, V> = HashMap<K, V>;

pub trait Component: 'static {
    const STORAGE: StorageTypes = StorageTypes::TableSoA;
    fn on_add() -> Option<for<'a> fn(world_data: &mut WorldData, entity: EntityKey)> {
        None
    }
    fn on_remove() -> Option<for<'a> fn(world_data: &mut WorldData, entity: EntityKey)> {
        None
    }
}

#[derive(Debug)]
pub struct ComponentInfo {
    #[allow(unused)]
    pub(crate) name: Cow<'static, str>,
    #[allow(unused)]
    pub(crate) comp_id: ComponentId,
    pub(crate) type_id: TypeId,
    pub(crate) layout: Layout,
    pub(crate) drop: Option<unsafe fn(*mut u8)>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ComponentId(pub(crate) u32);

impl_ecs_id!(ComponentId);

pub struct Archetype {
    #[allow(unused)]
    pub(crate) archetype_id: ArchetypeId,
    //pub(crate) comp_type_ids: Vec<TypeId>, TODO: is this needed?
    pub(crate) soa_comp_ids: SortedVec<ComponentId>,
    pub(crate) aos_comp_ids: SortedVec<ComponentId>,
}

impl Archetype {
    pub fn new(
        archetype_id: ArchetypeId,
        soa_comp_ids: SortedVec<ComponentId>,
        aos_comp_ids: SortedVec<ComponentId>,
    ) -> Self {
        Self {
            archetype_id,
            soa_comp_ids,
            aos_comp_ids,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ArchetypeId(pub u32);

impl_ecs_id!(ArchetypeId);

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ArchetypeHash(u32);

impl From<u32> for ArchetypeId {
    fn from(value: u32) -> Self {
        ArchetypeId(value)
    }
}

impl From<ArchetypeId> for u32 {
    fn from(value: ArchetypeId) -> Self {
        value.0
    }
}

impl ComponentInfo {
    unsafe fn drop_ptr<T>(ptr: *mut u8) {
        let typed_ptr: *mut T = ptr.cast::<T>();
        let _ = unsafe { drop_in_place(typed_ptr) };
    }

    pub fn new<T: 'static>(comp_id: u32) -> Self {
        Self {
            name: Cow::Borrowed(core::any::type_name::<T>()),
            comp_id: ComponentId(comp_id),
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T>),
        }
    }
}

pub enum StorageTypes {
    TableAoS,
    TableSoA,
    SparseSet,
}

#[cfg(test)]
mod test {
    use crate::{
        ecs::{entity::EntityKey, prelude::{StorageTypes, With}, query::Query, world::{World, WorldData}},
        utils::tuple_types::TupleTypesExt,
    };

    use super::Component;
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }
    impl Vec3 {
        fn new(x: f32, y: f32, z: f32) -> Self {
            Self { x, y, z }
        }
    }

    struct Player {
        particles: Vec<EntityKey>,
    }
    impl Player {
        fn new() -> Self {
            Self {
                particles: Vec::new(),
            }
        }
    }
    impl Component for Player {
        fn on_add() -> Option<for<'a> fn(world_data: &mut WorldData, entity: EntityKey)> {
            Some(on_add_player_comp)
        }
    }
    struct Particle {
        parent: EntityKey,
    }
    impl Component for Particle {}

    struct Pos3 {
        pos: Vec3,
    }
    impl Pos3 {
        fn new(x: f32, y: f32, z: f32) -> Self {
            Self {
                pos: Vec3::new(x, y, z),
            }
        }
    }
    impl Component for Pos3 {
        const STORAGE: StorageTypes = StorageTypes::TableAoS;
    }

    struct Velocity {
        acc: Vec3,
    }
    impl Velocity {
        fn new(x: f32, y: f32, z: f32) -> Self {
            Self {
                acc: Vec3::new(x, y, z),
            }
        }
    }
    impl Component for Velocity {
        const STORAGE: StorageTypes = StorageTypes::TableAoS;
    }

    fn on_add_player_comp(world: &mut WorldData, entity: EntityKey) {
        if let Some(_player) = world.get_single_component::<Player>(entity) {
            let mut vec = Vec::with_capacity(10);
            for _i in 0..10 {
                let particle_entity_key = world.add_entity((
                    Particle { parent: entity },
                    Pos3::new(0., 0., 0.),
                    Velocity::new(0., 0., 0.),
                ));
                vec.push(particle_entity_key);
            }
            let player = world.get_single_component_mut::<Player>(entity).unwrap();
            player.particles.append(&mut vec);
        }
    }

    fn test_player_count_wout_filter(
        mut player_query: Query<&Player>, 
    ) {
        assert_eq!(2, player_query.iter().count());
    }

    fn test_player_count(
        mut player_query: Query<(&mut Pos3, &Velocity), With<Player>>, 
    ) {
        assert_eq!(2, player_query.iter().count());
    }

    fn test_particle_count(
        mut player_query: Query<(&mut Pos3, &Velocity), With<Player>>, 
        mut particle_query: Query<(&mut Pos3, &Velocity), With<Particle>>
    ) {
        assert_eq!(2, player_query.iter().count());
        for (p, v) in particle_query.iter() {
            p.pos.x += v.acc.x;
            p.pos.y += v.acc.y;
            p.pos.z += v.acc.z;
        }
        assert_eq!(20, particle_query.iter().count());
    }

    #[test]
    fn test_component_hooks() {
        let mut world: World = World::new();
        world.add_entity((
            Player::new(),
            Pos3::new(0., 0., 0.),
            Velocity::new(0.02, 0., 0.),
        ));
        world.add_entity((
            Player::new(),
            Pos3::new(0., 0., 0.),
            Velocity::new(0.02, 0., 0.),
        ));
        world.add_system(test_player_count);
        world.add_system(test_player_count_wout_filter);
        world.add_system(test_particle_count);
        world.init_and_run();
    }

    #[test]
    fn test_tuple_ext_methods() {
        let t = Pos3::new(0., 0., 0.);
        let mut vec = Vec::new();
        t.self_type_ids_rec(&mut vec);
    }
}
