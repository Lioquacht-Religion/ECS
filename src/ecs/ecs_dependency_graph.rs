// ecs_dependeny_graph.rs

use std::{collections::{HashMap, HashSet}, };

use crate::{
    ecs::{
        component::{ArchetypeId, ComponentId}, prelude::EntityStorage, query::{query_filter, QueryParamMetaData, QueryState, RefKind}, resource::ResourceId, system::SystemId
    },
    utils::{ecs_id::{impl_ecs_id, EcsId}, sorted_vec::SortedVec},
};

pub enum EcsNode {
    Component(ComponentId),
    Archetype(ArchetypeId),
    Query(QueryId),
    Resource(ResourceId),
    System(SystemId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcsEdge {
    Owned,
    Excl,
    Shared,
    None,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct QueryId(u32);
impl_ecs_id!(QueryId);

type EcsEdges = HashMap<u32, EcsEdge>;

pub struct SystemNode {
    #[allow(unused)]
    pub(crate) system_id: SystemId,
    pub(crate) component_edges: EcsEdges,
    pub(crate) resource_edges: EcsEdges,
    pub(crate) query_edges: EcsEdges,
}

impl SystemNode {
    pub fn new(system_id: SystemId) -> Self {
        Self {
            system_id,
            resource_edges: HashMap::new(),
            component_edges: HashMap::new(),
            query_edges: HashMap::new(),
        }
    }
}

pub struct ResourceNode {
    #[allow(unused)]
    pub(crate) resource_id: ResourceId,
    pub(crate) system_edges: EcsEdges,
}

impl ResourceNode {
    pub fn new(resource_id: ResourceId) -> Self {
        Self {
            resource_id,
            system_edges: HashMap::new(),
        }
    }
}

pub struct ComponentNode {
    #[allow(unused)]
    pub(crate) component_id: ComponentId,
    pub(crate) system_edges: EcsEdges,
}

impl ComponentNode {
    pub fn new(component_id: ComponentId) -> Self {
        Self {
            component_id,
            system_edges: HashMap::new(),
        }
    }
}

pub struct ArchetypeNode {
    #[allow(unused)]
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) component_edges: EcsEdges,
}

impl ArchetypeNode {
    pub fn new(archetype_id: ArchetypeId) -> Self {
        Self {
            archetype_id,
            component_edges: HashMap::new(),
        }
    }
}

pub struct QueryNode {
    #[allow(unused)]
    pub(crate) query_id: QueryId,
    #[allow(unused)]
    pub(crate) component_edges: EcsEdges,
    pub(crate) archetype_edges: EcsEdges,
}

impl QueryNode {
    pub fn new(query_id: QueryId) -> Self {
        Self {
            query_id,
            component_edges: HashMap::new(),
            archetype_edges: HashMap::new(),
        }
    }
}

pub struct EcsDependencyGraph {
    pub systems: Vec<SystemNode>,
    pub resources: Vec<ResourceNode>,
    pub components: Vec<ComponentNode>,
    pub archetypes: Vec<ArchetypeNode>,
    pub queries: Vec<QueryNode>,
    pub system_keys: HashMap<SystemId, u32>,
    pub resource_keys: HashMap<ResourceId, u32>,
    pub component_keys: HashMap<ComponentId, u32>,
    pub archetype_keys: HashMap<ArchetypeId, u32>,
    pub query_keys: HashMap<QueryId, u32>,
}

impl EcsDependencyGraph {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            resources: Vec::new(),
            components: Vec::new(),
            archetypes: Vec::new(),
            queries: Vec::new(),
            system_keys: HashMap::new(),
            resource_keys: HashMap::new(),
            archetype_keys: HashMap::new(),
            component_keys: HashMap::new(),
            query_keys: HashMap::new(),
        }
    }

    pub fn insert_system(&mut self, system_id: SystemId) -> u32 {
        if let Some(key) = self.system_keys.get(&system_id) {
            return *key;
        }
        let key: u32 = self.systems.len().try_into().unwrap();
        self.systems.push(SystemNode::new(system_id));
        self.system_keys.insert(system_id, key);
        key
    }
    ///NOTE: Do not insert the same resource id multiple times
    /// into the resource edges of a system.
    /// A system can only contain one of each resource as a system param.
    /// Otherwise this method will panic.
    pub fn insert_system_resource(
        &mut self,
        system_id: SystemId,
        resource: ResourceId,
        ecs_edge: EcsEdge,
    ) -> (u32, u32) {
        let system_key = if let Some(key) = self.system_keys.get(&system_id) {
            *key
        } else {
            self.insert_system(system_id)
        };
        let resource_key = self.insert_resource(resource);
        let resource_id = resource_key as usize;
        let res = &mut self.resources[resource_id];
        res.system_edges.insert(system_key, ecs_edge);
        let system_node = &mut self.systems[system_key as usize];
        if let Some(_res_edge_key) = system_node.resource_edges.get(&resource_key) {
            panic!("A system cannot have multiple params of the same resource type.")
        }
        let _ = system_node.resource_edges.insert(resource_key, ecs_edge);

        (system_key, resource_key)
    }
    pub fn insert_resource(&mut self, resource_id: ResourceId) -> u32 {
        if let Some(key) = self.resource_keys.get(&resource_id) {
            return *key;
        }
        let key: u32 = self.resources.len().try_into().unwrap();
        self.resources.push(ResourceNode::new(resource_id));
        self.resource_keys.insert(resource_id, key);
        key
    }
    pub fn insert_component(&mut self, component_id: ComponentId) -> u32 {
        if let Some(key) = self.component_keys.get(&component_id) {
            return *key;
        }
        let key = self.components.len().try_into().unwrap();
        self.components.push(ComponentNode::new(component_id));
        self.component_keys.insert(component_id, key);
        key
    }
    pub fn insert_archetype(&mut self, archetype_id: ArchetypeId) -> u32 {
        if let Some(key) = self.archetype_keys.get(&archetype_id) {
            return *key;
        }
        let key: u32 = self.archetypes.len().try_into().unwrap();
        self.archetypes.push(ArchetypeNode::new(archetype_id));
        self.archetype_keys.insert(archetype_id, key);
        key
    }
    pub fn insert_query(&mut self, query_id: QueryId) -> u32 {
        if let Some(key) = self.query_keys.get(&query_id) {
            return *key;
        }
        let key = self.queries.len().try_into().unwrap();
        self.queries.push(QueryNode::new(query_id));
        self.query_keys.insert(query_id, key);
        key
    }

    pub(crate) fn insert_archetype_components(
        &mut self,
        query_stati: &mut Vec<QueryState>,
        archetype_id: ArchetypeId,
        comp_ids: &[ComponentId],
    ) {
        let arch_key = self.insert_archetype(archetype_id);
        for cid in comp_ids.iter() {
            let comp_key = self.insert_component(*cid);
            let arch: &mut ArchetypeNode = &mut self.archetypes[arch_key as usize];
            arch.component_edges.insert(comp_key, EcsEdge::None);
        }

        // update archetypes of query nodes
        let arch: &mut ArchetypeNode = &mut self.archetypes[arch_key as usize];
        let mut query_ids : HashSet<QueryId> = HashSet::new();
        arch.component_edges.iter().for_each(|(c_row_id, _edge)|{
            self.components[*c_row_id as usize].system_edges.iter()
                .for_each(|(s_row_id, _edge)|{
                    self.systems[*s_row_id as usize].query_edges.iter()
                        .map(|(q_row_id, _edge)|{
                            self.queries[*q_row_id as usize].query_id.clone()
                        })
                    .for_each(|qid| {
                        query_ids.insert(qid);
                    });
                })
        });
        let arch_comp_ids_set : HashSet<ComponentId> = comp_ids.iter().map(|cid| *cid).collect();
        let arch_comp_ids : SortedVec<ComponentId> = comp_ids.iter().map(|cid| *cid).collect::<Vec<ComponentId>>().into();
        for qid in query_ids.iter(){
            let qnode = &mut self.queries[qid.id_usize()];
            let query_comp_row_ids : HashSet<u32> = qnode.component_edges
                .keys().map(|c_row_id|{ *c_row_id }).collect();
            //TODO: need to take optional query params into account
            let query_state = &mut query_stati[qid.id_usize()];

            if EntityStorage::is_subset_of(
                &query_state.query_param_meta_data, 
                &arch_comp_ids
            ){
                let not_filtered_out = query_filter::comp_ids_compatible_with_filter(
                    &arch_comp_ids_set,
                    &query_state.filter,
                );
                // need to take query filters into account
                if not_filtered_out {
                    println!("not filtered out");
                    dbg!(&arch_comp_ids);
                    dbg!(&query_comp_row_ids);
                    dbg!(&query_state.filter);
                    query_state.arch_ids.insert(archetype_id);
                    qnode.archetype_edges.insert(arch_key, EcsEdge::None);
                }
            }
        }
    }

    ///NOTE: Do not insert the same component ids multiple times
    /// into the componenent edges of a system.
    /// A system can currently only contain one of each component as a system param.
    /// Otherwise this method will panic.
    pub fn insert_system_components(
        &mut self,
        system_id: SystemId,
        meta_datas: &[QueryParamMetaData],
    ) {
        let system_key = self.insert_system(system_id);
        for meta_data in meta_datas.iter() {
            let comp_key = self.insert_component(meta_data.comp_id);
            let edge = match meta_data.ref_kind {
                RefKind::Exclusive => EcsEdge::Excl,
                RefKind::Shared => EcsEdge::Shared,
            };
            let comp: &mut ComponentNode = &mut self.components[comp_key as usize];
            comp.system_edges.insert(system_key, edge);
            let system_node = &mut self.systems[system_key as usize];
            if let Some(_comp_edge_key) = system_node.component_edges.get(&comp_key) {
                panic!("A system cannot have multiple params of the same component type.")
            }
            let _ = system_node.component_edges.insert(comp_key, edge);
        }
    }
    pub fn insert_query_archetypes(&mut self, query_id: QueryId, archetype_ids: &[ArchetypeId]) {
        let query_key = self.insert_query(query_id);
        for aid in archetype_ids.iter() {
            let arch_key = self.insert_archetype(*aid);
            let query = &mut self.queries[query_key as usize];
            query.archetype_edges.insert(arch_key, EcsEdge::None);
        }
    }
    pub fn insert_system_query(&mut self, system_id: SystemId, query_id: QueryId) {
        let system_key = self.insert_system(system_id);
        let query_key = self.insert_query(query_id);
        let system = &mut self.systems[system_key as usize];
        system.query_edges.insert(query_key, EcsEdge::None);
    }
}

impl Default for EcsDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use crate::ecs::prelude::*;

    #[allow(unused)]
    struct Res1(usize);
    #[allow(unused)]
    struct Res2(String);

    #[allow(unused)]
    struct Comp1(usize);
    impl Component for Comp1 {}
    #[allow(unused)]
    struct Comp2(String);
    impl Component for Comp2 {}
    #[allow(unused)]
    struct Comp3(String);
    impl Component for Comp3 {}

    fn invalid_res_sys_params_system(_res1_1: Res<Res1>, _res2: ResMut<Res2>, _res1_2: Res<Res1>) {
        panic!("This should not be reached by system execution.")
    }

    fn invalid_query_sys_params_system(
        _query1: Query<(&Comp1, &mut Comp2)>,
        _query2: Query<(&mut Comp1, &mut Comp3)>,
    ) {
        panic!("This should not be reached by system execution.")
    }

    fn invalid_query_mult_same_comp_type_sys_params_system(_query1: Query<(&Comp1, &mut Comp1)>) {
        panic!("This should not be reached by system execution.")
    }

    fn init_world() -> World {
        let mut world = World::new();
        world.add_resource(Res1(909));
        world.add_resource(Res2(String::from("Bob Bobster")));
        world.add_entity((
            Comp1(324),
            Comp2("edw".to_string()),
            Comp3("ewwevcwre".to_string()),
        ));
        world.add_entity((
            Comp1(324),
            Comp2("edw".to_string()),
            Comp3("ewwevcwre".to_string()),
        ));
        world.add_entity((Comp1(324), Comp2("edw".to_string())));
        world.add_entity((Comp1(324), Comp3("ewwevcwre".to_string())));
        world.add_entity((Comp1(324), Comp3("ewwevcwre".to_string())));

        world
    }

    fn test_query_same_component_sets_with_excluding_filters_system(
        mut comp1_2_query: Query<(&mut Comp1, &Comp2), With<Comp3>>,
        mut comp1_3_query: Query<(&mut Comp1, &Comp3), With<Comp2>>,
    ) {
        //TODO: this should be possible
        assert_eq!(2, comp1_2_query.iter().count());
        assert_eq!(2, comp1_3_query.iter().count());
    }

    #[test]
    #[should_panic(expected = "A system cannot have multiple params of the same resource type.")]
    fn test_multiple_same_resource_types_in_sysparams() {
        let mut world = init_world();
        world.add_systems(invalid_res_sys_params_system);
        world.init_and_run();
    }

    #[test]
    #[should_panic(expected = "A system cannot have multiple params of the same component type.")]
    fn test_multiple_same_components_types_in_query_sysparams() {
        let mut world = init_world();
        world.add_systems(invalid_query_sys_params_system);
        world.init_and_run();
    }

    #[test]
    #[should_panic(expected = "A system cannot have multiple params of the same component type.")]
    fn test_single_query_sysparam_cant_contain_multiple_of_same_comp_type() {
        let mut world = init_world();
        world.add_systems(invalid_query_mult_same_comp_type_sys_params_system);
        world.init_and_run();
    }

    #[test]
    fn test_query_same_component_sets_with_excluding_filters() {
        let mut world = init_world();
        world.add_systems(test_query_same_component_sets_with_excluding_filters_system);
        world.init_and_run();
    }
}
