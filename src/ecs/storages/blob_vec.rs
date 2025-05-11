//blob_vec.rs
//

//TODO: dealloc and realloc allocation at old pointer
//TODO: do not drop other inner boxed values of moved boxed value
//TODO: store drop methods of stored values too for correct deallocation
//TODO: macro for more efficient tuple inserting through use of destructering

use std::{
    alloc::Layout,
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    ptr::NonNull,
};

use crate::utils::tuple_types::TupleTypesExt;

#[derive(Clone, Debug)]
pub struct TypeMetaData {
    type_id: TypeId,
    layout: Layout,
    offset: usize,
    drop_fn: Option<for<'a> unsafe fn(*mut ())>,
}

pub struct BlobArray {
    pub type_meta_data_map: HashMap<TypeId, TypeMetaData>,
    pub type_meta_data: Vec<TypeMetaData>,
    pub layout: Layout, // component combined type
    ptr: NonNull<u8>,
    cap: usize,
    len: usize,
    free_indexes: Vec<u32>,
}

impl Drop for BlobArray {
    fn drop(&mut self) {
        //call drop on every type erased entry
        //TODO:

        //dealloc allocation
        let cur_array_layout = Layout::from_size_align(self.cap, self.layout.align())
            .expect("err dealloc layout creation!");
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr(), cur_array_layout);
        }
    }
}

impl BlobArray {
    pub fn from_tuple_type<T: TupleTypesExt + 'static>() -> Self {
        Self::new(T::type_ids(), T::type_layouts())
    }

    pub fn new(types: Vec<TypeId>, comp_layouts: Vec<Layout>) -> Self {
        let mut type_meta_data_map = HashMap::with_capacity(types.len());
        let mut type_meta_data = Vec::with_capacity(types.len());

        let mut lo_iter = comp_layouts.iter().enumerate();
        let (_, layout) = lo_iter.next().unwrap();
        let mut layout = *layout;
        //TODO: maybe remove this extra elem fo offset zero? this would always be the same
        let tmd = TypeMetaData {
            type_id: types[0],
            layout,
            offset: 0,
            drop_fn: None,
        };
        type_meta_data_map.insert(types[0], tmd.clone());
        type_meta_data.push(tmd);

        for (i, lo) in lo_iter {
            let (ext_layout, offset) = layout.extend(*lo).unwrap();
            layout = ext_layout;
            let tmd = TypeMetaData {
                type_id: types[i],
                layout: *lo,
                offset,
                drop_fn: None,
            };
            type_meta_data_map.insert(types[i], tmd.clone());
            type_meta_data.push(tmd);
        }
        layout = layout.pad_to_align();

        if layout.size() == 0 {
            panic!("Zero Sized Types are not supported!")
        }

        Self {
            type_meta_data_map,
            type_meta_data,
            layout,
            ptr: NonNull::dangling(),
            cap: 0,
            len: 0,
            free_indexes: Vec::new(),
        }
    }

    // usable for structs/tuples that are not repr C
    pub unsafe fn insert_tuple_unchecked2<T: TupleTypesExt + 'static>(
        &mut self,
        mut tuple_entry: T,
    ) {
        if self.len == self.cap {
            self.grow();
        }

        let tup_el_ptrs = tuple_entry.self_get_elem_ptrs();

        let base_offset = self.layout.size() * self.len;
        let entry_ptr: *mut u8 = self.ptr.as_ptr().add(base_offset).cast();

        for (i, tmd) in self.type_meta_data.iter().enumerate() {
            // cast to u8 needed, zero sized types will not be copied such as ()
            let src: *mut u8 = tup_el_ptrs[i].cast();
            let dst = entry_ptr.add(tmd.offset);
            std::ptr::copy(src, dst, tmd.layout.size());
        }

        self.len += 1;
        std::mem::forget(tuple_entry);
    }

    // should only be used with type that are reprc
    pub unsafe fn insert_tuple_unchecked<T: 'static>(&mut self, tuple_entry: T) {
        if self.len == self.cap {
            self.grow();
        }

        let offset = self.layout.size() * self.len;
        let entry_ptr: *mut T = self.ptr.as_ptr().add(offset).cast();
        std::ptr::write(entry_ptr, tuple_entry);

        self.len += 1;
    }

    pub unsafe fn insert_move(&mut self, mut components: Vec<Box<dyn Any>>) {
        let mut c_ptrs: Vec<*mut ()> = Vec::with_capacity(components.len());

        while let Some(comp) = components.pop() {
            let ptr: *mut () = Box::into_raw(comp) as *mut ();
            c_ptrs.push(ptr);
        }

        let mut iter = c_ptrs.iter().rev();
        unsafe {
            self.insert(&mut iter);
        }

        c_ptrs.iter().for_each(|ptr| {
            let ptr = *ptr as *mut dyn Any;
            //TODO: fix possible deallocation and use after frees of inner pointers
            let _box_val = unsafe { Box::from_raw(ptr) };
        });
    }

    pub unsafe fn insert<'a>(&mut self, components: &mut impl Iterator<Item = &'a *mut ()>) {
        if self.len == self.cap {
            self.grow();
        }

        let base_offset = self.layout.size() * self.len;
        for (ind, c_ptr) in components.into_iter().enumerate() {
            unsafe {
                let c_ptr: *mut u8 = *c_ptr as *mut u8;
                let tmd = &self.type_meta_data[ind];
                let c_layout = tmd.layout;
                let offset = base_offset + tmd.offset;
                std::ptr::copy(c_ptr, self.ptr.as_ptr().add(offset), c_layout.size());
            }
        }
        self.len += 1;
    }

    unsafe fn grow(&mut self) {
        let new_ptr = if self.cap == 0 {
            let new_cap = 4;
            let cur_array_layout =
                Layout::from_size_align(self.layout.size() * new_cap, self.layout.align())
                    .expect("err realloc layout creation!");

            println!("cur array layout cap = 0: {:?}", &cur_array_layout);
            let new_ptr = unsafe { std::alloc::alloc(cur_array_layout) };
            self.cap = new_cap;
            new_ptr
        } else {
            let new_cap = self.cap * 2;
            let new_alloc_size = self.layout.size() * new_cap;
            let cur_array_layout = Layout::from_size_align(self.cap, self.layout.align())
                .expect("err realloc layout creation!");
            println!("cur array layout: {:?}", &cur_array_layout);
            let new_ptr = unsafe {
                std::alloc::realloc(
                    self.ptr.as_ptr() as *mut u8,
                    cur_array_layout,
                    new_alloc_size,
                )
            };
            self.cap = new_cap;
            new_ptr
        };
        self.ptr = NonNull::new(new_ptr).unwrap();
    }

    pub fn get_mut<T: Any>(&mut self, index: usize) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        if let Some(tmd) = self.type_meta_data_map.get(&type_id) {
            println!("get tmd offset: {}", tmd.offset);
            let total_offset = self.layout.size() * index + tmd.offset;
            unsafe {
                let ptr = self.ptr.as_ptr().add(total_offset).cast::<T>();
                let reference = &mut *ptr;
                return Some(reference);
            }
        } else {
            None
        }
    }

    pub fn iter_mut<T: TupleTypesExt>(&mut self) -> BlobIterator<T> {
        BlobIterator::new(self)
    }
}

pub struct BlobIterator<'a, T> {
    blob: &'a mut BlobArray,
    entry_ind: usize,
    comp_offsets: Vec<usize>,
    _ph: PhantomData<T>,
}

impl<'a, T: TupleTypesExt> BlobIterator<'a, T>
where
    T: TupleTypesExt,
{
    fn new(blob_array: &'a mut BlobArray) -> Self {
        let types = T::type_ids();
        let offsets: Vec<usize> = types
            .iter()
            .map(|tid| blob_array.type_meta_data_map.get(tid).unwrap().offset)
            .collect();

        Self {
            blob: blob_array,
            entry_ind: 0,
            comp_offsets: offsets,
            _ph: PhantomData,
        }
    }
}

impl<'a, T1: 'a, T2: 'a> Iterator for BlobIterator<'a, (T1, T2)> {
    type Item = (&'a mut T1, &'a mut T2);
    fn next(&mut self) -> Option<Self::Item> {
        if self.entry_ind >= self.blob.len {
            return None;
        }
        let base_offset = self.blob.layout.size() * self.entry_ind;
        unsafe {
            let entr_ptr = self.blob.ptr.add(base_offset);
            let T1: &mut T1 = &mut *entr_ptr.add(self.comp_offsets[0]).as_ptr().cast();
            let T2: &mut T2 = &mut *entr_ptr.add(self.comp_offsets[0 + 1]).as_ptr().cast();
            self.entry_ind += 1;
            return Some((T1, T2));
        }
    }
}

macro_rules! impl_blob_iterator {
    () => {};
}

#[cfg(test)]
mod test {
    use std::{
        alloc::Layout,
        any::{Any, TypeId},
    };

    use super::BlobArray;

    struct comp1(usize, usize, u8, u8);
    struct comp2(usize, u8, u8, u16);
    struct comp3(usize, Box<comp2>);

    #[test]
    fn it_works() {
        let c1 = comp1(1234, 5678, 123, 23);
        let c2 = comp2(2234, 2, 3, 23);
        let c3 = comp3(3234, Box::new(comp2(1234, 2, 3, 23)));
        let c12 = comp1(4234, 5678, 123, 23);
        let c22 = comp2(5234, 2, 3, 23);
        let c32 = comp3(6234, Box::new(comp2(89234, 2, 3, 23)));

        let types: Vec<TypeId> = vec![
            TypeId::of::<comp1>(),
            TypeId::of::<comp2>(),
            TypeId::of::<comp3>(),
        ];
        let comp_layouts: Vec<Layout> = vec![
            Layout::new::<comp1>(),
            Layout::new::<comp2>(),
            Layout::new::<comp3>(),
        ];
        let mut ba = BlobArray::new(types, comp_layouts);

        let comps: Vec<Box<dyn Any>> = vec![Box::new(c1), Box::new(c2), Box::new(c3)];
        let comps2: Vec<Box<dyn Any>> = vec![Box::new(c12), Box::new(c22), Box::new(c32)];
        unsafe {
            ba.insert_move(comps);
            assert!(ba.cap == 4);
            assert!(ba.len == 1);
            ba.insert_move(comps2);
            assert!(ba.cap == 4);
            assert!(ba.len == 2);

            assert!(ba.type_meta_data.len() == 3);
            assert!(ba.type_meta_data_map.len() == 3);

            assert!(ba.get_mut::<comp1>(0).unwrap().0 == 1234);
            assert!(ba.get_mut::<comp2>(0).unwrap().0 == 2234);
            ba.get_mut::<comp2>(0).unwrap();
            ba.get_mut::<comp3>(0).unwrap();
            ba.get_mut::<comp1>(1).unwrap();
            ba.get_mut::<comp2>(1).unwrap();
            assert!(ba.get_mut::<comp3>(1).unwrap().1 .0 == 89234);

            for (c1, c3) in ba.iter_mut::<(comp1, comp3)>() {
                c1.1;
                c3.0;
            }
        }
    }
}
