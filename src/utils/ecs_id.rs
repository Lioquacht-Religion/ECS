// ecs_id.rs

pub trait EcsId: From<usize> {
    fn id(&self) -> u32;
    fn id_usize(&self) -> usize {
        self.id() as usize
    }
}

macro_rules! impl_ecs_id {
    ( $id_type:ident) => {
        impl EcsId for $id_type {
            fn id(&self) -> u32 {
                self.0
            }
        }
        impl From<$id_type> for usize {
            fn from(value: $id_type) -> Self {
                value.id_usize()
            }
        }

        impl From<usize> for $id_type {
            fn from(value: usize) -> Self {
                let id: u32 = value
                    .try_into()
                    .expect("Archetype Ids have increased over their max possible u32 value!");
                $id_type(id)
            }
        }
    };
}

pub(crate) use impl_ecs_id;
