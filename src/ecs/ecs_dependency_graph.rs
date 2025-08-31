// ecs_dependeny_graph.rs

use std::collections::HashMap;

use crate::{
    ecs::{component::{ArchetypeId, ComponentId}, resource::ResourceId, system::SystemId},
    utils::{ecs_id::{impl_ecs_id, EcsId}, gen_vec::Key, graph::{Graph, Node}},
};

pub enum EcsNode {
    Component(ComponentId),
    Archetype(ArchetypeId),
    Query(QueryId),
    Resource(ResourceId),
    System(SystemId),
}

pub enum EcsEdge {
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

    pub fn insert_system(system_id: SystemId, resources: &[ResourceId], components: &[ComponentId]){
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
