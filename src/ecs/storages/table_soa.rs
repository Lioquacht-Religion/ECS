// table_soa.rs

use crate::ecs::{component::ComponentId, Entity};

use super::{sparse_set::SparseSet, thin_blob_vec::ThinBlobVec};

pub struct TableSoA{
    entities: Vec<Entity>,
    columns: SparseSet<ComponentId, ThinBlobVec>,
    cap : usize,
}
