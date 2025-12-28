//thin_blob_vec.rs

//TODO: fix UB, add check for not allocation for layouts of size zero

use std::{alloc::Layout, marker::PhantomData, ptr::NonNull};

use crate::{
    ecs::component::{Component, ComponentId, ComponentInfo},
    utils::tuple_iters::TupleIterator,
};

use super::table_aos::TypeMetaData;

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
    pub data_ptr: NonNull<u8>,
    pub elem_layout: Layout,
    pub drop_fn: Option<unsafe fn(*mut u8)>,
}

impl ThinBlobVec {
    pub(crate) fn new(layout: Layout, drop_fn: Option<unsafe fn(*mut u8)>) -> Self {
        Self {
            data_ptr: NonNull::dangling(),
            elem_layout: layout,
            drop_fn,
        }
    }

    pub(crate) fn new_typed<T>() -> Self {
        Self {
            data_ptr: NonNull::dangling(),
            elem_layout: Layout::new::<T>(),
            drop_fn: Some(Self::drop_ptr::<T>),
        }
    }

    pub(crate) unsafe fn call_drop_on_elem(&mut self, index: usize) -> NonNull<u8> {
        unsafe {
            let elem_ptr = self.data_ptr.add(self.elem_layout.size() * index);
            if let Some(drop_fn) = self.drop_fn {
                let elem_ptr = self.data_ptr.add(self.elem_layout.size() * index);
                drop_fn(elem_ptr.as_ptr());
                elem_ptr
            } else {
                elem_ptr
            }
        }
    }

    pub(crate) unsafe fn remove_and_replace_with_last(&mut self, len: usize, to: usize) {
        //TODO: i do not think this works correctly,
        // check it again, especially if this works for both soa and aos
        assert!(to < len);

        unsafe {
            let to_ptr = self.call_drop_on_elem(to);
            if to == len - 1 {
                let from = self.data_ptr.add(self.elem_layout.size() * (len - 1));
                std::ptr::copy(from.as_ptr(), to_ptr.as_ptr(), self.elem_layout.size());
            }
        }
    }

    pub(crate) unsafe fn dealloc(&mut self, cap: usize, len: usize) {
        if cap == 0 || self.elem_layout.size() == 0 {
            return;
        }

        //call drop on every type erased entry
        if let Some(drop_fn) = self.drop_fn {
            for i in 0..len {
                unsafe {
                    let elem_ptr = self.data_ptr.add(self.elem_layout.size() * i);
                    drop_fn(elem_ptr.as_ptr());
                }
            }
        }

        //dealloc allocation
        let alloc_size = cap * self.elem_layout.size();
        let cur_array_layout = Layout::from_size_align(alloc_size, self.elem_layout.align())
            .expect("err dealloc layout creation!");
        unsafe {
            std::alloc::dealloc(self.data_ptr.as_ptr(), cur_array_layout);
        }
    }

    pub(crate) unsafe fn dealloc_typed<T>(&mut self, cap: usize, len: usize) {
        unsafe {
            self.dealloc(cap, len);
        }
    }

    pub(crate) unsafe fn drop_ptr<T>(ptr: *mut u8) {
        let typed_ptr: *mut T = ptr.cast::<T>();
        let _ = unsafe { std::ptr::drop_in_place(typed_ptr) };
    }

    /// Grows dynamic array by doubling the supplied capacity.
    /// Returns new capacity of the new allocation.
    ///
    /// #SAFETY:
    /// Do not call grow for layouts of size zero.
    /// Allocating memory of size zero is undefined behaviour
    pub(crate) unsafe fn grow(&mut self, cap: usize) -> usize {
        let (new_ptr, new_cap) = if cap == 0 {
            let new_cap = 4;
            let cur_array_layout =
                Layout::from_size_align(self.elem_layout.size() * new_cap, self.elem_layout.align())
                    .expect("err realloc layout creation!");

            let new_ptr = unsafe { std::alloc::alloc(cur_array_layout) };
            (new_ptr, new_cap)
        } else {
            let new_cap = cap * 2;
            let new_alloc_size = self.elem_layout.size() * new_cap;
            let cur_array_layout =
                Layout::from_size_align(self.elem_layout.size() * cap, self.elem_layout.align())
                    .expect("err realloc layout creation!");
            let new_ptr = unsafe {
                std::alloc::realloc(
                    self.data_ptr.as_ptr() as *mut u8,
                    cur_array_layout,
                    new_alloc_size,
                )
            };
            (new_ptr, new_cap)
        };
        self.data_ptr = NonNull::new(new_ptr).unwrap();
        new_cap
    }

    /// Pushes an untyped element onto the calling ThinBlobVec.
    /// Returns new capacity of this ThinBlobVec after pushing the new element onto it.
    ///
    /// #SAFETY:
    /// Call std::mem::forget on the pushed value
    /// after calling this function to prevent it from being dropped.
    pub(crate) unsafe fn push_untyped(
        &mut self,
        cap: usize,
        len: usize,
        value_ptr: NonNull<u8>,
    ) -> usize {
        if self.elem_layout.size() != 0 {
            unsafe {
                let new_cap = if len == cap { self.grow(cap) } else { cap };

                let base_offset = self.elem_layout.size() * len;
                let entry_ptr: *mut u8 = self.data_ptr.as_ptr().add(base_offset).cast();

                std::ptr::copy(value_ptr.as_ptr(), entry_ptr, self.elem_layout.size());

                return new_cap;
            }
        }
        // for zero-sized-types return capacity zero
        0
    }

    /// #SAFETY:
    /// Call std::mem::forget on the pushed value after calling this function to prevent it from being dropped.
    pub(crate) unsafe fn push_ptr_vec_untyped(
        &mut self,
        cap: &mut usize,
        len: &mut usize,
        comp_infos: &[ComponentInfo],
        comp_offsets: &[TypeMetaData],
        value_ptrs: &[CompElemPtr],
    ) {
        unsafe {
            self.push_ptr_vec_untyped_with_offset(
                cap,
                len,
                comp_infos,
                comp_offsets,
                value_ptrs,
                0,
            );
        }
    }

    ///#SAFETY
    /// Call std::mem::forget on the pushed value after calling this function to prevent it from being dropped.
    pub(crate) unsafe fn push_ptr_vec_untyped_with_offset(
        &mut self,
        cap: &mut usize,
        len: &mut usize,
        comp_infos: &[ComponentInfo],
        comp_offsets: &[TypeMetaData],
        value_ptrs: &[CompElemPtr],
        value_ptrs_offset: usize,
    ) {
        // do not allocate memory for zero-sized-types, e.g. layout size is zero
        if self.elem_layout.size() > 0 {
            unsafe {
                *cap = if len == cap { self.grow(*cap) } else { *cap };

                let base_offset = self.elem_layout.size() * *len;
                let entry_ptr: *mut u8 = self.data_ptr.as_ptr().add(base_offset).cast();

                for (i, value_ptr) in value_ptrs.iter().enumerate() {
                    let comp_info = &comp_infos[value_ptr.comp_id.0 as usize];
                    let dst_comp_ptr: *mut u8 = entry_ptr.add(comp_offsets[i].ptr_offset);
                    let layout_size = comp_info.layout.size();
                    // do not copy zero sized types to different location
                    if true
                    //layout_size > 0
                    {
                        let value_ptr_src = value_ptr.ptr.add(value_ptrs_offset);
                        std::ptr::copy(value_ptr_src.as_ptr(), dst_comp_ptr, layout_size);
                    }
                }

                *len += 1;
            }
        }
    }

    pub(crate) unsafe fn push_typed<T>(&mut self, cap: usize, len: usize, mut value: T) -> usize {
        let new_cap = unsafe {
            self.push_untyped(
                cap,
                len,
                NonNull::new_unchecked(&mut value as *mut T).cast(),
            )
        };

        std::mem::forget(value);
        new_cap
    }

    pub(crate) unsafe fn get_ptr_untyped(&self, index: usize, layout: Layout) -> NonNull<u8> {
        //TODO: add ZST safety
        if layout.size() != 0 {
            unsafe { self.data_ptr.add(index * layout.size()) }
        } else {
            self.data_ptr
        }
    }

    pub(crate) unsafe fn get_typed<T>(&self, index: usize) -> &T {
        unsafe {
            &*self
                .get_ptr_untyped(index, Layout::new::<T>())
                .cast()
                .as_ptr()
        }
    }

    pub(crate) unsafe fn get_typed_lifetime<'vec, T>(&self, index: usize) -> &'vec T {
        unsafe {
            &*self
                .get_ptr_untyped(index, Layout::new::<T>())
                .cast()
                .as_ptr()
        }
    }

    pub(crate) unsafe fn get_mut_typed<T>(&mut self, index: usize) -> &mut T {
        unsafe {
            &mut *self
                .get_ptr_untyped(index, Layout::new::<T>())
                .cast()
                .as_ptr()
        }
    }

    pub(crate) unsafe fn get_mut_typed_lifetime<'vec, T>(&mut self, index: usize) -> &'vec mut T {
        unsafe {
            &mut *self
                .get_ptr_untyped(index, Layout::new::<T>())
                .cast()
                .as_ptr()
        }
    }

    pub(crate) unsafe fn get_mut_inner_typed_lifetime<'vec, T>(
        &mut self,
        index: usize,
        offset: usize,
    ) -> &'vec mut T {
        unsafe {
            &mut *self
                .get_ptr_untyped(index, self.elem_layout)
                .cast::<T>()
                .byte_offset(offset as isize)
                .as_ptr()
        }
    }

    pub(crate) unsafe fn get_inner_typed_lifetime<'vec, T>(
        &self,
        index: usize,
        offset: usize,
    ) -> &'vec T {
        unsafe {
            &*self
                .get_ptr_untyped(index, self.elem_layout)
                .cast::<T>()
                .byte_offset(offset as isize)
                .as_ptr()
        }
    }

    pub(crate) unsafe fn iter<T: 'static>(&mut self, len: usize) -> ThinBlobIter<'_, T> {
        ThinBlobIter::new(self, len)
    }

    pub(crate) unsafe fn tuple_iter<T: 'static>(&self) -> ThinBlobIterUnsafe<'_, T> {
        ThinBlobIterUnsafe::new(self)
    }

    pub(crate) unsafe fn tuple_iter_mut<T: 'static>(&mut self) -> ThinBlobIterMutUnsafe<'_, T> {
        ThinBlobIterMutUnsafe::new(self)
    }

    pub(crate) unsafe fn tuple_inner_type_iter<T: 'static>(
        &self,
        offset: usize,
    ) -> ThinBlobInnerTypeIterUnsafe<'_, T> {
        ThinBlobInnerTypeIterUnsafe::new(self, offset)
    }

    pub(crate) unsafe fn tuple_inner_type_iter_mut<T: 'static>(
        &mut self,
        offset: usize,
    ) -> ThinBlobInnerTypeIterMutUnsafe<'_, T> {
        ThinBlobInnerTypeIterMutUnsafe::new(self, offset)
    }
}

pub struct ThinBlobIterUnsafe<'vec, T: 'static> {
    vec: &'vec ThinBlobVec,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobIterUnsafe<'vec, T> {
    pub(crate) fn new(blob: &'vec ThinBlobVec) -> Self {
        ThinBlobIterUnsafe {
            vec: blob,
            marker: PhantomData,
        }
    }
}

impl<'vec, T: Component + 'static> TupleIterator for ThinBlobIterUnsafe<'vec, T> {
    type Item = &'vec T;
    unsafe fn next(&mut self, index: usize) -> Self::Item {
        unsafe { self.vec.get_typed_lifetime(index) }
    }
}

pub struct ThinBlobIterMutUnsafe<'vec, T: 'static> {
    vec: &'vec mut ThinBlobVec,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobIterMutUnsafe<'vec, T> {
    pub(crate) fn new(blob: &'vec mut ThinBlobVec) -> Self {
        ThinBlobIterMutUnsafe {
            vec: blob,
            marker: PhantomData,
        }
    }
}

impl<'vec, T: Component + 'static> TupleIterator for ThinBlobIterMutUnsafe<'vec, T> {
    type Item = &'vec mut T;
    unsafe fn next(&mut self, index: usize) -> Self::Item {
        unsafe { self.vec.get_mut_typed_lifetime(index) }
    }
}

pub struct ThinBlobInnerTypeIterUnsafe<'vec, T: 'static> {
    vec: &'vec ThinBlobVec,
    offset: usize,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobInnerTypeIterUnsafe<'vec, T> {
    pub(crate) fn new(blob: &'vec ThinBlobVec, offset: usize) -> Self {
        ThinBlobInnerTypeIterUnsafe {
            vec: blob,
            offset,
            marker: PhantomData,
        }
    }
}

impl<'vec, T: Component + 'static> TupleIterator for ThinBlobInnerTypeIterUnsafe<'vec, T> {
    type Item = &'vec T;
    unsafe fn next(&mut self, index: usize) -> Self::Item {
        unsafe { self.vec.get_inner_typed_lifetime(index, self.offset) }
    }
}

pub struct ThinBlobInnerTypeIterMutUnsafe<'vec, T: 'static> {
    vec: &'vec mut ThinBlobVec,
    offset: usize,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobInnerTypeIterMutUnsafe<'vec, T> {
    pub(crate) fn new(blob: &'vec mut ThinBlobVec, offset: usize) -> Self {
        ThinBlobInnerTypeIterMutUnsafe {
            vec: blob,
            offset,
            marker: PhantomData,
        }
    }
}

impl<'vec, T: Component + 'static> TupleIterator for ThinBlobInnerTypeIterMutUnsafe<'vec, T> {
    type Item = &'vec mut T;
    unsafe fn next(&mut self, index: usize) -> Self::Item {
        unsafe { self.vec.get_mut_inner_typed_lifetime(index, self.offset) }
    }
}

pub(crate) struct ThinBlobIter<'vec, T: 'static> {
    vec: &'vec mut ThinBlobVec,
    len: usize,
    index: usize,
    marker: PhantomData<T>,
}

impl<'vec, T: 'static> ThinBlobIter<'vec, T> {
    pub(crate) fn new(blob: &'vec mut ThinBlobVec, len: usize) -> Self {
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
    struct Comp1(usize, usize, u8, u8, Box<Comp2>);
    #[derive(Debug)]
    struct Comp2(usize, u8, u16);

    impl Default for Comp1 {
        fn default() -> Self {
            Comp1(23, 435, 2, 5, Box::new(Comp2(64, 99, 5000)))
        }
    }

    #[test]
    fn test_thin_vec() {
        let mut bv = ThinBlobVec::new_typed::<Comp1>();
        unsafe {
            let cap = bv.push_typed(0, 0, Comp1::default());
            let cap = bv.push_typed(cap, 1, Comp1::default());
            let cap = bv.push_typed(cap, 2, Comp1::default());
            let cap = bv.push_typed(cap, 3, Comp1::default());
            let cap = bv.push_typed(cap, 4, Comp1::default());
            let cap = bv.push_typed(cap, 5, Comp1::default());
            let _v: &mut Comp1 = bv.get_mut_typed(0);
            let _v: &mut Comp1 = bv.get_mut_typed(4);

            for c in bv.iter::<Comp1>(6) {
                println!("{:?}", c);
                assert_eq!(
                    format!("{:?}", c),
                    format!("{:?}", Comp1(23, 435, 2, 5, Box::new(Comp2(64, 99, 5000))))
                );
            }

            bv.dealloc_typed::<Comp1>(cap, 6);

            drop(bv);
        }
    }
}
