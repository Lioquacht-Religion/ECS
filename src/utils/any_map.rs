use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub trait AnyMapTrait {
    fn get<T>(&self) -> Option<&T>;
    fn get_mut<T>(&mut self) -> Option<&mut T>;
    fn insert<T>(&mut self, value: T);
    fn remove<T>(&mut self) -> Option<T>;
}

pub struct AnyMap {
    data: HashMap<TypeId, Box<dyn Any>>,
}

impl AnyMap {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        match self.data.get(&type_id) {
            None => return None,
            Some(boxed_val) => {
                return boxed_val.downcast_ref::<T>();
            }
        }
    }

    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        match self.data.get_mut(&type_id) {
            None => return None,
            Some(boxed_val) => {
                return boxed_val.downcast_mut::<T>();
            }
        }
    }

    pub fn insert<T: Any>(&mut self, value: T) {
        let type_id = TypeId::of::<T>();
        let value: Box<dyn Any> = Box::new(value);
        self.data.insert(type_id, value);
    }

    pub fn remove<T: Any>(&mut self) -> Option<T> {
        let type_id = TypeId::of::<T>();
        match self.data.remove(&type_id) {
            None => None,
            Some(boxed_val) => Some(*boxed_val.downcast::<T>().unwrap()),
        }
    }

    pub fn len(&self) -> usize{
        self.data.len()
    }
}

#[cfg(test)]
mod test {

    #[derive(Debug, PartialEq, Eq)]
    struct Pos(i32, i32);

    #[test]
    fn test_1() {
        /*
        let mut map = AnyMap::new();
        let num: i32 = 0;
        let pos1: Pos = Pos(232, 3453);
        let vec: Vec<Pos> = vec![Pos(123, 456)];
        let mut gen_vec: GenVec<Pos> = GenVec::new();
        let key_1: Key = gen_vec.insert(Pos(123, 456));
        map.insert(num);
        map.insert(pos1);
        map.insert(vec);
        map.insert(gen_vec);

        assert_eq!(map.get(), Some(&0i32));
        assert_eq!(map.get(), Some(&Pos(232, 3453)));
        //assert_eq!(map.get(), Some(&vec![Pos(123, 456)]));

        let ref_elem: &GenVec<Pos> = map.get().unwrap();
        let ref_elem_pos = ref_elem.get(&key_1);
        assert_eq!(ref_elem_pos, Some(&Pos(123, 456)));

        //assert_eq!(map.remove(), Some(vec![Pos(123, 456)]));

        assert_eq!(map.get(), None::<&f32>);

        let tuple_1 = (Pos(234, 567), 4.678);
        map.insert(tuple_1);

        assert_eq!(map.get(), Some(&(Pos(234, 567), 4.678)));
        */
    }
}
