// ecs_dependeny_graph.rs

use std::collections::HashMap;

use crate::{
    ecs::{component::{ArchetypeId, ComponentId}, resource::ResourceId, system::SystemId},
    utils::{ecs_id::{impl_ecs_id, EcsId}, gen_vec::Key, graph::{Edge, Graph, Node}},
};

pub enum EcsNode {
    Component(ComponentId),
    Archetype(ArchetypeId),
    Query(QueryId),
    Resource(ResourceId),
    System(SystemId),
}

#[derive(Debug, Clone)]
pub enum EcsEdge {
    Owned,
    Excl,
    Shared,
    None,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct QueryId(u32);
impl_ecs_id!(QueryId);

pub struct EcsDependencyGraph {
    graph: Graph<EcsNode, EcsEdge>,
    components: HashMap<ComponentId, Key>,
    archetypes: HashMap<ArchetypeId, Key>,
    queries: HashMap<QueryId, Key>,
    resources: HashMap<ResourceId, Key>,
    systems: HashMap<SystemId, Key>,
}

impl EcsDependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            components: HashMap::new(),
            archetypes: HashMap::new(),
            queries: HashMap::new(),
            resources: HashMap::new(),
            systems: HashMap::new(),
        }
    }

    pub fn insert_system(&mut self, system_id: SystemId) -> Key{
        let key = self.graph.insert_node(Node::new(EcsNode::System(system_id)));
        self.systems.insert(system_id, key);
        key
    }
    pub fn insert_system_resource(&mut self, system_id: SystemId, resource: ResourceId, ecs_edge_val: EcsEdge) -> Key{
        let system_key = *self.systems.get(&system_id)
            .expect("System with supplied system_id has not yet been added to dependency graph.");
        let resource_key = self.insert_resource(resource);
        self.graph.add_edge_to_both_nodes(
            system_key, ecs_edge_val.clone(), 
            resource_key, ecs_edge_val,
        );
        resource_key
    }

    pub fn insert_resource(&mut self, resource_id: ResourceId) -> Key{
        let key = self.graph.insert_node(Node::new(EcsNode::Resource(resource_id)));
        self.resources.insert(resource_id, key);
        key
    }
    pub fn insert_component(&mut self, component_id: ComponentId) -> Key{
        let key = self.graph.insert_node(Node::new(EcsNode::Component(component_id)));
        self.components.insert(component_id, key);
        key
    }
    pub fn insert_archetype(&mut self, archetype_id: ArchetypeId) -> Key{
        let key = self.graph.insert_node(Node::new(EcsNode::Archetype(archetype_id)));
        self.archetypes.insert(archetype_id, key);
        key
    }
}
