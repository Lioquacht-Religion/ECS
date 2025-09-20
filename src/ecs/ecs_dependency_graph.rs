// ecs_dependeny_graph.rs

use std::collections::HashMap;

use crate::{
    ecs::{
        component::{ArchetypeId, ComponentId},
        query::RefKind,
        resource::ResourceId,
        system::SystemId,
    },
    utils::{
        ecs_id::{EcsId, impl_ecs_id},
        gen_vec::Key,
        graph::{Graph, Node},
    },
};

pub enum EcsNode {
    Component(ComponentId),
    Archetype(ArchetypeId),
    Query(QueryId),
    Resource(ResourceId),
    System(SystemId),
}

#[derive(Debug, Clone, Copy)]
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

    pub fn insert_system(&mut self, system_id: SystemId) -> Key {
        if let Some(key) = self.systems.get(&system_id) {
            return *key;
        }
        let key = self
            .graph
            .insert_node(Node::new(EcsNode::System(system_id)));
        self.systems.insert(system_id, key);
        key
    }
    pub fn insert_system_resource(
        &mut self,
        system_id: SystemId,
        resource: ResourceId,
        ecs_edge_val: EcsEdge,
    ) -> Key {
        let system_key = if let Some(key) = self.systems.get(&system_id) {
            *key
        } else {
            self.insert_system(system_id)
        };
        let resource_key = self.insert_resource(resource);
        self.graph.add_edge_to_both_nodes(
            system_key,
            ecs_edge_val.clone(),
            resource_key,
            ecs_edge_val,
        );
        resource_key
    }
    pub fn insert_resource(&mut self, resource_id: ResourceId) -> Key {
        if let Some(key) = self.resources.get(&resource_id) {
            return *key;
        }
        let key = self
            .graph
            .insert_node(Node::new(EcsNode::Resource(resource_id)));
        self.resources.insert(resource_id, key);
        key
    }
    pub fn insert_component(&mut self, component_id: ComponentId) -> Key {
        if let Some(key) = self.components.get(&component_id) {
            return *key;
        }
        let key = self
            .graph
            .insert_node(Node::new(EcsNode::Component(component_id)));
        self.components.insert(component_id, key);
        key
    }
    pub fn insert_archetype(&mut self, archetype_id: ArchetypeId) -> Key {
        if let Some(key) = self.archetypes.get(&archetype_id) {
            return *key;
        }
        let key = self
            .graph
            .insert_node(Node::new(EcsNode::Archetype(archetype_id)));
        self.archetypes.insert(archetype_id, key);
        key
    }
    pub fn insert_query(&mut self, query_id: QueryId) -> Key {
        if let Some(key) = self.queries.get(&query_id) {
            return *key;
        }
        let key = self.graph.insert_node(Node::new(EcsNode::Query(query_id)));
        self.queries.insert(query_id, key);
        key
    }
    pub fn insert_archetype_components(
        &mut self,
        archetype_id: ArchetypeId,
        comp_ids: &[ComponentId],
    ) {
        let arch_key = self.insert_archetype(archetype_id);
        for cid in comp_ids.iter() {
            let comp_key = self.insert_component(*cid);
            self.graph
                .add_edge_to_both_nodes(arch_key, EcsEdge::None, comp_key, EcsEdge::None);
        }
    }
    pub fn insert_system_components(
        &mut self,
        system_id: SystemId,
        comp_ids: &[ComponentId],
        ref_kinds: &[RefKind],
    ) {
        let system_key = self.insert_system(system_id);
        for (i, cid) in comp_ids.iter().enumerate() {
            let comp_key = self.insert_component(*cid);
            let edge = match ref_kinds[i] {
                RefKind::Exclusive => EcsEdge::Excl,
                RefKind::Shared => EcsEdge::Shared,
            };
            self.graph
                .add_edge_to_both_nodes(system_key, edge, comp_key, edge);
        }
    }
    pub fn insert_query_archetypes(&mut self, query_id: QueryId, archetype_ids: &[ArchetypeId]) {
        let query_key = self.insert_query(query_id);
        for aid in archetype_ids.iter() {
            let arch_key = self.insert_archetype(*aid);
            self.graph
                .add_edge_to_both_nodes(query_key, EcsEdge::None, arch_key, EcsEdge::None);
        }
    }
    pub fn insert_system_query(&mut self, system_id: SystemId, query_id: QueryId) {
        let system_key = self.insert_system(system_id);
        let query_key = self.insert_query(query_id);
        self.graph
            .add_edge_to_both_nodes(system_key, EcsEdge::None, query_key, EcsEdge::None);
    }
}
