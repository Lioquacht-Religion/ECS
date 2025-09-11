// graph.rs

use crate::utils::gen_vec::{GenVec, Key};

pub struct Graph<V, E = ()> {
    nodes: GenVec<Node<V, E>>,
}

pub struct Node<V, E = ()> {
    pub value: V,
    edges: Vec<Edge<E>>,
}

pub struct Edge<E = ()> {
    pub value: E,
    node_key: Key,
}

impl<V, E> Graph<V, E> {
    pub fn new() -> Self {
        Self {
            nodes: GenVec::new(),
        }
    }

    pub fn insert_node(&mut self, node: Node<V, E>) -> Key {
        self.nodes.insert(node)
    }

    pub fn add_edge_to_first_node(&mut self, edge_val: E, node_key: Key) {
        let node = self.nodes.get_mut(&node_key);
        if let Some(node) = node {
            node.edges.push(Edge::new(edge_val, node_key));
        }
    }

    pub fn add_edge_to_both_nodes(
        &mut self,
        node_key1: Key,
        edge_val1: E,
        node_key2: Key,
        edge_val2: E,
    ) {
        self.add_edge_to_first_node(edge_val1, node_key1);
        self.add_edge_to_first_node(edge_val2, node_key2);
    }

    pub fn get_node(&self, key: Key) -> Option<&Node<V, E>> {
        self.nodes.get(&key)
    }
    pub fn get_node_mut(&mut self, key: Key) -> Option<&mut Node<V, E>> {
        self.nodes.get_mut(&key)
    }

    pub fn iter<'g>(&'g self, key: Key) -> NodesIter<'g, V, E> {
        NodesIter::new(self, key)
    }
}

pub struct NodesIter<'g, V, E = ()> {
    graph: &'g Graph<V, E>,
    start_node: &'g Node<V, E>,
    cur_edge_index: usize,
}

impl<'g, V, E> NodesIter<'g, V, E> {
    pub fn new(graph: &'g Graph<V, E>, key: Key) -> Self {
        let start_node = graph
            .get_node(key)
            .expect("Graph does not contain node for supplied key.");
        Self {
            graph,
            start_node,
            cur_edge_index: 0,
        }
    }

    pub fn filter<P: FnMut(&Node<V, E>, &Edge<E>, &Node<V, E>) -> bool>(
        self,
        predicate: P,
    ) -> NodesFilter<'g, V, E, P> {
        NodesFilter::new(self.graph, self.start_node, predicate)
    }
}

impl<'g, V, E> Iterator for NodesIter<'g, V, E> {
    type Item = &'g Node<V, E>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_edge_index < self.start_node.get_edges().len() {
            let next_edge = self.start_node.get_edge(self.cur_edge_index);
            if let Some(next_node) = self.graph.get_node(next_edge.node_key) {
                self.cur_edge_index += 1;
                Some(next_node)
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct NodesFilter<'g, V, E, P: FnMut(&Node<V, E>, &Edge<E>, &Node<V, E>) -> bool> {
    graph: &'g Graph<V, E>,
    start_node: &'g Node<V, E>,
    cur_edge_index: usize,
    predicate: P,
}

impl<'g, V, E, P: FnMut(&Node<V, E>, &Edge<E>, &Node<V, E>) -> bool> NodesFilter<'g, V, E, P> {
    pub fn new(graph: &'g Graph<V, E>, start_node: &'g Node<V, E>, predicate: P) -> Self {
        Self {
            graph,
            start_node,
            cur_edge_index: 0,
            predicate,
        }
    }

    pub fn new_with_key(graph: &'g Graph<V, E>, start_node_key: Key, predicate: P) -> Self {
        let start_node = graph
            .get_node(start_node_key)
            .expect("Start node does not exist for supplied key!");
        Self {
            graph,
            start_node,
            cur_edge_index: 0,
            predicate,
        }
    }
}

impl<'g, V, E, P: FnMut(&Node<V, E>, &Edge<E>, &Node<V, E>) -> bool> Iterator
    for NodesFilter<'g, V, E, P>
{
    type Item = &'g Node<V, E>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.start_node.get_edges().len() > self.cur_edge_index {
                let next_edge = &self.start_node.get_edges()[self.cur_edge_index];
                let next_node = self
                    .graph
                    .get_node(next_edge.node_key)
                    .expect("Could not find next node with supplied key!");
                self.cur_edge_index += 1;
                if (self.predicate)(self.start_node, next_edge, next_node) {
                    return Some(next_node);
                }
            } else {
                return None;
            }
        }
    }
}

impl<V, E> Default for Graph<V, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E> Node<V, E> {
    pub fn new(value: V) -> Self {
        Self {
            value: value,
            edges: Vec::new(),
        }
    }
    pub fn get_mut_edge(&mut self, index: usize) -> &mut Edge<E> {
        &mut self.edges[index]
    }
    pub fn get_edge(&self, index: usize) -> &Edge<E> {
        &self.edges[index]
    }

    pub fn get_edges(&self) -> &Vec<Edge<E>> {
        &self.edges
    }
}

impl<E> Edge<E> {
    pub fn new(value: E, node_key: Key) -> Self {
        Self { value, node_key }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_graph1() {
        unimplemented!()
    }
}
