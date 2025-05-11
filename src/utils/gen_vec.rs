#[derive(Debug)]
pub struct GenVec<T> {
    vec: Vec<Entry<T>>,
    next_free_id: Option<u32>,
    len: usize,
}

#[derive(Debug)]
enum Entry<T> {
    Free {
        next_free_id: Option<u32>,
        generation: u32,
    },
    Occupied {
        value: T,
        generation: u32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Key {
    id: u32,
    generation: u32,
}

impl Key {
    pub fn get_id(&self) -> u32 {
        self.id
    }
    pub fn get_gen(&self) -> u32 {
        self.generation
    }
}

impl<T> GenVec<T> {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            next_free_id: None,
            len: 0,
        }
    }

    pub fn insert(&mut self, value: T) -> Key {
        self.len += 1;
        if let Some(cur_next_free_id) = self.next_free_id {
            if let Entry::Free {
                next_free_id,
                generation,
            } = self.vec[cur_next_free_id as usize]
            {
                let next_gen = generation + 1;
                self.vec[cur_next_free_id as usize] = Entry::Occupied {
                    value,
                    generation: next_gen,
                };
                match next_free_id {
                    Some(next_free_id) => self.next_free_id = Some(next_free_id),
                    None => self.next_free_id = None,
                }
                return Key {
                    id: cur_next_free_id,
                    generation: next_gen,
                };
            }
            unreachable!()
        } else {
            let id = self.vec.len() as u32;
            self.vec.push(Entry::Occupied {
                value,
                generation: 0,
            });
            Key { id, generation: 0 }
        }
    }

    pub fn get(&self, key: &Key) -> Option<&T> {
        match &self.vec[key.id as usize] {
            Entry::Free {
                next_free_id: _,
                generation: _,
            } => None,
            Entry::Occupied { value, generation } => {
                if *generation == key.generation {
                    Some(value)
                } else {
                    None
                }
            }
        }
    }

    pub fn get_mut(&mut self, key: &Key) -> Option<&mut T> {
        match &mut self.vec[key.id as usize] {
            Entry::Free {
                next_free_id: _,
                generation: _,
            } => None,
            Entry::Occupied { value, generation } => {
                if *generation == key.generation {
                    Some(value)
                } else {
                    None
                }
            }
        }
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let entry = &mut self.vec[key.id as usize];

        match entry {
            Entry::Free {
                next_free_id: _,
                generation: _,
            } => return None,
            Entry::Occupied {
                value: _,
                generation,
            } => {
                if *generation == key.generation {
                    // initialize free entry
                    let mut rem_entry = Entry::Free {
                        next_free_id: self.next_free_id.take(),
                        generation: *generation + 1,
                    };
                    // swap current entry with free entry to move cur entry out of vec
                    std::mem::swap(entry, &mut rem_entry);
                    if let Entry::Occupied {
                        value,
                        generation: _,
                    } = rem_entry
                    {
                        self.len -= 1;
                        self.next_free_id = Some(key.get_id());
                        return Some(value);
                    }
                    return None;
                } else {
                    return None;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.vec.iter().filter_map(|e| {
            if let Entry::Occupied {
                value,
                generation: _,
            } = &e
            {
                return Some(value);
            }
            None
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.vec.iter_mut().filter_map(|e| {
            if let Entry::Occupied {
                value,
                generation: _,
            } = e
            {
                return Some(value);
            }
            None
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::gen_vec::GenVec;

    struct Pos(i32, i32);

    #[test]
    fn it_works() {
        let mut gen_vec: GenVec<Pos> = GenVec::new();

        let key_1 = gen_vec.insert(Pos(3, 5));
        let key_2 = gen_vec.insert(Pos(3, 5));
        gen_vec.insert(Pos(3, 5));
        let key_4 = gen_vec.insert(Pos(3, 5));

        assert_eq!(key_1.get_id(), 0);
        assert_eq!(key_1.get_gen(), 0);
        assert_eq!(key_2.get_id(), 1);
        assert_eq!(key_2.get_gen(), 0);
        assert_eq!(key_4.get_id(), 3);
        assert_eq!(key_4.get_gen(), 0);

        assert_eq!(gen_vec.vec.len(), 4);

        let p1 = gen_vec.remove(key_1);
        assert!(p1.is_some());

        let p4 = gen_vec.remove(key_4);
        assert!(p4.is_some());

        let key_5 = gen_vec.insert(Pos(3, 5));

        assert_eq!(key_5.get_id(), 3);
        assert_eq!(key_5.get_gen(), 2);

        let p5 = gen_vec.get_mut(&key_5);
        assert!(p5.is_some());

        let key_6 = gen_vec.insert(Pos(3, 5));

        assert_eq!(key_6.get_id(), 0);
        assert_eq!(key_6.get_gen(), 2);
    }
}
