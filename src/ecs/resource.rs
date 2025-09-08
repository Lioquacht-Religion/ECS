// resource.rs

use std::any::TypeId;

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ResourceId(TypeId);

impl ResourceId {
    pub fn new(type_id: TypeId) -> Self{
        Self(type_id)
    } 
}

//trait Resource{}

//pub struct ResourceInfo{}
