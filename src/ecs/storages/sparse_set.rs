//sparse_set.rs

use std::hash::Hash;

pub struct SparseSet<I, V>
where I : PartialEq + Eq + Clone + Hash
{
    dense: Vec<V>,
    sparse: Vec<I>,
}
