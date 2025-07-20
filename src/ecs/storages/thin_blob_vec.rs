//thin_blob_vec.rs

//TODO: dealloc and realloc allocation at old pointer
//TODO: do not drop other inner boxed values of moved boxed value
//TODO: store drop methods of stored values too for correct deallocation
//TODO: macro for more efficient tuple inserting through use of destructering

use std::{alloc::Layout, marker::PhantomData, ptr::NonNull};

use crate::{
    ecs::component::{Component, ComponentId, ComponentInfo},
    utils::tuple_iters::TupleIterator,
};

#[derive(Debug, Hash, Eq)]
pub(crate) struct CompElemPtr {
    pub(crate) comp_id: ComponentId,
    pub(crate) ptr: NonNull<u8>,
}

impl PartialEq for CompElemPtr {
    fn eq(&self, other: &Self) -> bool {
        self.comp_id.eq(&other.comp_id)
    }
}

impl PartialOrd for CompElemPtr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.comp_id.partial_cmp(&other.comp_id)
    }
}

impl Ord for CompElemPtr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.comp_id.cmp(&other.comp_id)
    }
}

pub struct ThinBlobVec {
    pub data: NonNull<u8>,
    pub layout: Layout,
    pub drop_fn: Option<unsafe fn(*mut u8)>,
}

impl ThinBlobVec {
    pub fn new(layout: Layout, drop_fn: Option<unsafe fn(*mut u8)>) -> Self {
        Self {
            data: NonNull::dangling(),
            layout,
            drop_fn,
        }
    }

    pub fn new_typed<T>() -> Self {
        Self {
            data: NonNull::dangling(),
            layout: Layout::new::<T>(),
            drop_fn: Some(Self::drop_ptr::<T>),
        }
    }

    pub unsafe fn dealloc(&mut self, cap: usize, len: usize) {
        if cap == 0 {
            return;
        }

        //call drop on every type erased entry
        if let Some(drop_fn) = self.drop_fn {
            for i in 0..len {
                let elem_ptr = self.data.add(self.layout.size() * i);
                drop_fn(elem_ptr.as_ptr());
            }
        }

        //dealloc allocation
        let alloc_size = cap * self.layout.size();
        let cur_array_layout = Layout::from_size_align(alloc_size, self.layout.align())
            .expect("err dealloc layout creation!");
        std::alloc::dealloc(self.data.as_ptr(), cur_array_layout);
    }

    pub unsafe fn dealloc_typed<T>(&mut self, cap: usize, len: usize) {
        self.dealloc(cap, len);
    }

    pub unsafe fn drop_ptr<T>(ptr: *mut u8) {
        let typed_ptr: *mut T = ptr.cast::<T>();
        let _ = std::ptr::drop_in_place(typed_ptr);
    }

    /**
     * Grows dynamic array by doubling the supplied capacity.
     * Returns new capacity of the new allocation.
     */
    pub unsafe fn grow(&mut self, cap: usize) -> usize {
        let (new_ptr, new_cap) = if cap == 0 {
            let new_cap = 4;
            let cur_array_layout =
                Layout::from_size_align(self.layout.size() * new_cap, self.layout.align())
                    .expect("err realloc layout creation!");

            let new_ptr = unsafe { std::alloc::alloc(cur_array_layout) };
            (new_ptr, new_cap)
        } else {
            let new_cap = cap * 2;
            let new_alloc_size = self.layout.size() * new_cap;
            let cur_array_layout = Layout::from_size_align(cap, self.layout.align())
                .expect("err realloc layout creation!");
            let new_ptr = unsafe {
                std::alloc::realloc(
                    self.data.as_ptr() as *mut u8,
                    cur_array_layout,
                    new_alloc_size,
                )
            };
            (new_ptr, new_cap)
        };
        self.data = NonNull::new(new_ptr).unwrap();
        new_cap
    }

    /**
     * Call std::mem::forget on the pushed value after calling this function to prevent it from being dropped.
     *
     */
    pub unsafe fn push_untyped(&mut self, cap: usize, len: usize, value_ptr: NonNull<u8>) -> usize {
        let new_cap = if len == cap { self.grow(cap) } else { cap };

        let base_offset = self.layout.size() * len;
        let entry_ptr: *mut u8 = self.data.as_ptr().add(base_offset).cast();

        std::ptr::copy(value_ptr.as_ptr(), entry_ptr, self.layout.size());

        new_cap
    }

    /**
     * Call std::mem::forget on the pushed value after calling this function to prevent it from being dropped.
     *
     */
    pub(crate) unsafe fn push_ptr_vec_untyped(
        &mut self,
        cap: &mut usize,
        len: &mut usize,
        comp_infos: &[ComponentInfo],
        comp_offsets: &[usize],
        value_ptrs: &[CompElemPtr],
    ) {
        *cap = if len == cap { self.grow(*cap) } else { *cap };

        let base_offset = self.layout.size() * *len;
        let entry_ptr: *mut u8 = self.data.as_ptr().add(base_offset).cast();

        for (i, value_ptr) in value_ptrs.iter().enumerate() {
            let comp_info = &comp_infos[value_ptr.comp_id.0 as usize];
            let dst_comp_ptr: *mut u8 = entry_ptr.add(comp_offsets[i]);
            let layout_size = comp_info.layout.size();
            std::ptr::copy(value_ptr.ptr.as_ptr(), dst_comp_ptr, layout_size);
        }

        *len += 1;
    }

    pub unsafe fn push_typed<T>(&mut self, cap: usize, len: usize, mut value: T) -> usize {
        let new_cap = self.push_untyped(
            cap,
            len,
            NonNull::new_unchecked(&mut value as *mut T).cast(),
        );
        std::mem::forget(value);
        new_cap
    }

    pub unsafe fn get_ptr_untyped(&self, index: usize, layout: Layout) -> NonNull<u8> {
        self.data.add(index * layout.size())
    }

    pub unsafe fn get_typed<T>(&self, index: usize) -> &T {
        &*self
            .get_ptr_untyped(index, Layout::new::<T>())
            .cast()
            .as_ptr()
    }

    pub unsafe fn get_typed_lifetime<'vec, T>(&self, index: usize) -> &'vec T {
        &*self
            .get_ptr_untyped(index, Layout::new::<T>())
            .cast()
            .as_ptr()
    }

    pub unsafe fn get_mut_typed<T>(&mut self, index: usize) -> &mut T {
        &mut *self
            .get_ptr_untyped(index, Layout::new::<T>())
            .cast()
            .as_ptr()
    }

    pub unsafe fn get_mut_typed_lifetime<'vec, T>(&mut self, index: usize) -> &'vec mut T {
        &mut *self
            .get_ptr_untyped(index, Layout::new::<T>())
            .cast()
            .as_ptr()
    }

    pub unsafe fn get_mut_inner_typed_lifetime<'vec, T>(&mut self, index: usize, offset: usize) -> &'vec mut T {
        &mut *self
            .get_ptr_untyped(index, self.layout)
            .cast::<T>()
            .as_ptr()
            .add(offset)
    }

    pub unsafe fn get_inner_typed_lifetime<'vec, T>(&self, index: usize, offset: usize) -> &'vec T {
        &*self
            .get_ptr_untyped(index, self.layout)
            .cast::<T>()
            .as_ptr()
            .add(offset)
    }

    pub unsafe fn iter<T: 'static>(&mut self, len: usize) -> ThinBlobIter<'_, T> {
        ThinBlobIter::new(self, len)
    }

    pub unsafe fn tuple_iter<T: 'static>(&self) -> ThinBlobIterUnsafe<'_, T> {
        ThinBlobIterUnsafe::new(self)
    }

    pub unsafe fn tuple_iter_mut<T: 'static>(&mut self) -> ThinBlobIterMutUnsafe<'_, T> {
        ThinBlobIterMutUnsafe::new(self)
    }

    pub unsafe fn tuple_inner_type_iter<T: 'static>(&self, offset: usize) 
        -> ThinBlobInnerTypeIterUnsafe<'_, T> {
        ThinBlobInnerTypeIterUnsafe::new(self, offset)
    }

    pub unsafe fn tuple_inner_type_iter_mut<T: 'static>(&mut self, offset: usize) 
        -> ThinBlobInnerTypeIterMutUnsafe<'_, T> {
        ThinBlobInnerTypeIterMutUnsafe::new(self, offset)
    }

}

pub struct ThinBlobIterUnsafe<'vec, T: 'static> {
    vec: &'vec ThinBlobVec,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobIterUnsafe<'vec, T> {
    pub fn new(blob: &'vec ThinBlobVec) -> Self {
        ThinBlobIterUnsafe {
            vec: blob,
            marker: PhantomData,
        }
    }
}

impl<'vec, T: Component + 'static> TupleIterator for ThinBlobIterUnsafe<'vec, T> {
    type Item = &'vec T;
    fn next(&mut self, index: usize) -> Self::Item {
        unsafe { self.vec.get_typed_lifetime(index) }
    }
}

pub struct ThinBlobIterMutUnsafe<'vec, T: 'static> {
    vec: &'vec mut ThinBlobVec,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobIterMutUnsafe<'vec, T> {
    pub fn new(blob: &'vec mut ThinBlobVec) -> Self {
        ThinBlobIterMutUnsafe {
            vec: blob,
            marker: PhantomData,
        }
    }
}

impl<'vec, T: Component + 'static> TupleIterator for ThinBlobIterMutUnsafe<'vec, T> {
    type Item = &'vec mut T;
    fn next(&mut self, index: usize) -> Self::Item {
        unsafe { self.vec.get_mut_typed_lifetime(index) }
    }
}

pub struct ThinBlobInnerTypeIterUnsafe<'vec, T: 'static> {
    vec: &'vec ThinBlobVec,
    offset: usize,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobInnerTypeIterUnsafe<'vec, T> {
    pub fn new(blob: &'vec ThinBlobVec, offset: usize) -> Self {
        ThinBlobInnerTypeIterUnsafe {
            vec: blob,
            offset,
            marker: PhantomData,
        }
    }
}

impl<'vec, T: Component + 'static> TupleIterator for ThinBlobInnerTypeIterUnsafe<'vec, T> {
    type Item = &'vec T;
    fn next(&mut self, index: usize) -> Self::Item {
        unsafe { self.vec.get_inner_typed_lifetime(index, self.offset) }
    }
}

pub struct ThinBlobInnerTypeIterMutUnsafe<'vec, T: 'static> {
    vec: &'vec mut ThinBlobVec,
    offset: usize,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobInnerTypeIterMutUnsafe<'vec, T> {
    pub fn new(blob: &'vec mut ThinBlobVec, offset: usize) -> Self {
        ThinBlobInnerTypeIterMutUnsafe {
            vec: blob,
            offset, 
            marker: PhantomData,
        }
    }
}

impl<'vec, T: Component + 'static> TupleIterator for ThinBlobInnerTypeIterMutUnsafe<'vec, T> {
    type Item = &'vec mut T;
    fn next(&mut self, index: usize) -> Self::Item {
        unsafe { self.vec.get_mut_inner_typed_lifetime(index, self.offset) }
    }
}

pub struct ThinBlobIter<'vec, T: 'static> {
    vec: &'vec mut ThinBlobVec,
    len: usize,
    index: usize,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobIter<'vec, T> {
    pub fn new(blob: &'vec mut ThinBlobVec, len: usize) -> Self {
        ThinBlobIter {
            vec: blob,
            len,
            index: 0,
            marker: PhantomData,
        }
    }
}

impl<'vec, T: 'static> Iterator for ThinBlobIter<'vec, T> {
    type Item = &'vec T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            None
        } else {
            let next = unsafe { self.vec.get_typed_lifetime::<T>(self.index) };
            self.index += 1;
            Some(next)
        }
    }
}

#[cfg(test)]
mod test {
    use super::ThinBlobVec;

    #[derive(Debug)]
    struct comp1(usize, usize, u8, u8);
    struct comp2(usize, u8, u8, u16);
    struct comp3(usize, Box<comp2>);

    #[test]
    fn test_thin_vec() {
        let mut bv = ThinBlobVec::new_typed::<comp1>();
        unsafe {
            let cap = bv.push_typed(0, 0, comp1(23, 435, 2, 5));
            let cap = bv.push_typed(cap, 1, comp1(23, 435, 2, 5));
            let cap = bv.push_typed(cap, 2, comp1(23, 435, 2, 5));
            let cap = bv.push_typed(cap, 3, comp1(23, 435, 2, 5));
            let cap = bv.push_typed(cap, 4, comp1(23, 435, 2, 5));
            let cap = bv.push_typed(cap, 5, comp1(23, 435, 2, 5));
            let v: &mut comp1 = bv.get_mut_typed(0);
            let v: &mut comp1 = bv.get_mut_typed(4);

            for c in bv.iter::<comp1>(6) {
                println!("{:?}", c);
            }

            bv.dealloc_typed::<comp1>(cap, 6);

            drop(bv);
        }
    }
}
