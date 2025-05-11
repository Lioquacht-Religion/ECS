use std::{
    alloc::{GlobalAlloc, Layout},
    any::{Any, TypeId},
    marker::PhantomData,
    ptr::NonNull,
};

/**
 *
 * This module contains a heterogen stack data structure.
 * Different data types can be pushed onto this stack
 * of continues memory, which dynamically resizes and reallocates
 * without any indirection through, e.g. dynamic dispatch.
 *
 *
 * This data structure is not finished or tested yet!
 *
 *
 */
pub struct HetStack {
    meta_data: Vec<EntryMetaData>,
    head: NonNull<u8>,
    data_layout: Layout,
    pub used_bytes: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key<T: Any> {
    pub id: u32,
    pub type_id: TypeId,
    _p: PhantomData<T>,
}

struct EntryMetaData {
    type_id: TypeId,
    total_offset: usize,
}

impl HetStack {
    pub fn new() -> Self {
        let layout = Layout::new::<()>();

        let head: *mut u8 = unsafe { std::alloc::alloc(layout) };

        Self {
            meta_data: Vec::new(),
            head: NonNull::new(head).unwrap(),
            data_layout: layout,
            used_bytes: 0,
            total_bytes: 0,
        }
    }

    pub fn init<T: Any>(value: T) -> Self {
        let layout = Layout::for_value(&value);
        let used_bytes = std::mem::size_of_val(&value);

        let head: *mut u8 = unsafe { std::alloc::alloc(layout) };

        let meta_data = EntryMetaData {
            type_id: TypeId::of::<T>(),
            total_offset: 0,
        };

        Self {
            meta_data: vec![meta_data],
            head: NonNull::new(head).unwrap(),
            data_layout: layout,
            used_bytes,
            total_bytes: used_bytes,
        }
    }

    fn grow_by_type<T: Any>(&mut self) -> EntryMetaData {
        let (new_layout, offset) = if self.total_bytes == 0 {
            (
                Layout::new::<T>(), //.pad_to_align()
                0,
            )
        } else {
            let layout_new_val = Layout::new::<T>();
            let (ext_layout, offset) = self.data_layout.extend(layout_new_val).unwrap();
            (
                ext_layout, //.pad_to_align()
                offset,
            )
        };

        //realloc
        let new_head = if self.total_bytes == 0 {
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            unsafe { std::alloc::realloc(self.head.as_ptr(), self.data_layout, new_layout.size()) }
        };

        self.head = match NonNull::new(new_head) {
            Some(p) => p,
            None => std::alloc::handle_alloc_error(new_layout),
        };
        self.data_layout = new_layout;
        self.total_bytes = new_layout.size();

        let new_meta_data = EntryMetaData {
            type_id: TypeId::of::<T>(),
            total_offset: offset,
        };

        new_meta_data
    }

    pub fn push<T: Any>(&mut self, value: T) -> Key<T> {
        let next_type_id = TypeId::of::<T>();
        let next_id = self.meta_data.len();
        let val_size = std::mem::size_of::<T>();

        if self.used_bytes + val_size > self.total_bytes {
            let entry_data = self.grow_by_type::<T>();
            self.meta_data.push(entry_data);
        }

        unsafe {
            std::ptr::write(self.head.as_ptr().add(self.used_bytes).cast::<T>(), value);
        }

        self.used_bytes = self.data_layout.size();

        Key {
            id: next_id as u32,
            type_id: next_type_id,
            _p: PhantomData,
        }
    }

    pub fn get_mut<T: Any>(&mut self, key: &Key<T>) -> Option<&mut T> {
        if let Some(entry_data) = &self.meta_data.get(*&key.id as usize) {
            if entry_data.type_id == key.type_id {
                unsafe {
                    let ptr = self.head.as_ptr().add(entry_data.total_offset).cast::<T>();
                    let reference = &mut *ptr;
                    return Some(reference);
                }
            }
            None
        } else {
            None
        }
    }
}

/*
#[cfg(test)]
mod test{
    use std::any::TypeId;

    use super::{HetStack, Key};

    struct Pos(i32, i32);

    #[test]
    fn it_works(){
        let mut stack : HetStack = HetStack::new();
        let num1_i32 : i32 = 3453;

        let key1 : Key<i32> = stack.push(num1_i32);
        let key2 : Key<Pos> = stack.push(Pos(124, 23452));

        assert!(key1.id == 0);
        assert!(key1.type_id == TypeId::of::<i32>());

        assert!(key2.id == 1);

        let refr1 : &i32 = stack.get_mut(&key1).unwrap();
        assert!(*refr1 == 3453);
        let refr2 : &Pos = stack.get_mut(&key2).unwrap();
        assert!(refr2.0 == 124);
        assert!(refr2.1 == 23452);

    }
}
*/
