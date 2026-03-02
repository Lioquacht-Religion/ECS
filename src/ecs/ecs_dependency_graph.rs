// ecs_dependeny_graph.rs

use std::collections::{HashMap, HashSet};

use crate::{
    ecs::{
        component::{ArchetypeId, ComponentId},
        query::RefKind,
        resource::ResourceId,
        system::SystemId,
    },
    utils::ecs_id::{EcsId, impl_ecs_id},
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
    pub(crate) query_id: QueryId,
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
        let resource_key = if let Some(key) = self.resource_keys.get(&resource) {
            *key
        } else {
            self.insert_resource(resource)
        };
        let resource_id = resource_key as usize;
        let res = &mut self.resources[resource_id];
        res.system_edges.insert(system_key, ecs_edge);
        let _ = &mut self.systems[system_key as usize]
            .resource_edges
            .insert(resource_key, ecs_edge);

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
    pub fn insert_archetype_components(
        &mut self,
        archetype_id: ArchetypeId,
        comp_ids: &[ComponentId],
    ) {
        let arch_key = self.insert_archetype(archetype_id);
        for cid in comp_ids.iter() {
            let comp_key = self.insert_component(*cid);
            let arch: &mut ArchetypeNode = &mut self.archetypes[arch_key as usize];
            arch.component_edges.insert(comp_key, EcsEdge::None);
            //TODO add archetype edges to components?
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
            let comp: &mut ComponentNode = &mut self.components[comp_key as usize];
            comp.system_edges.insert(system_key, edge);
            let _ = &mut self.systems[system_key as usize]
                .component_edges
                .insert(comp_key, edge);
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
