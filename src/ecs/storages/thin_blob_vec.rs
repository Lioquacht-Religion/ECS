//thin_blob_vec.rs

//TODO: dealloc and realloc allocation at old pointer
//TODO: do not drop other inner boxed values of moved boxed value
//TODO: store drop methods of stored values too for correct deallocation
//TODO: macro for more efficient tuple inserting through use of destructering

use std::{
    alloc::Layout,
    ptr::NonNull,
};

pub struct ThinBlobVec {
    pub data : NonNull<u8>,
    pub layout : Layout,
}

impl ThinBlobVec{

    pub fn new(layout: Layout) -> Self {
        Self {
            data: NonNull::dangling(),
            layout
        }
    }

    pub unsafe fn dealloc(&mut self, cap: usize, drop_fn: Option<unsafe fn(*mut u8)>){
        if cap == 0 {
            return;
        }

        //call drop on every type erased entry
        let alloc_size = cap*self.layout.size();

        if let Some(drop_fn) = drop_fn{
            for i in 0..cap{
              let elem_ptr = self.data.add(self.layout.size()*i);
              drop_fn(elem_ptr.as_ptr());
            }
        }

        //dealloc allocation
        let cur_array_layout = Layout::from_size_align(alloc_size, self.layout.align())
            .expect("err dealloc layout creation!");
        std::alloc::dealloc(self.data.as_ptr(), cur_array_layout);
    }


    pub unsafe fn dealloc_typed<T>(&mut self, cap: usize){
        self.dealloc(cap, Some(Self::drop_ptr::<T>));
    }

    unsafe fn drop_ptr<T>(ptr: *mut u8) {
        let typed_ptr: *mut T = ptr.cast::<T>();
        let _ = std::ptr::drop_in_place(typed_ptr);
    }


    /**
     * Grows dynamic array by doubling the supplied capacity.
     * Returns new capacity of the new allocation.
     */
    pub unsafe fn grow(&mut self, cap: usize) -> usize{
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
    pub unsafe fn push_untyped(&mut self, cap: usize, len: usize, value_ptr: NonNull<u8>) -> usize{
        let new_cap = if len == cap {
            self.grow(cap)
        }
        else{cap};

        let base_offset = self.layout.size() * len;
        let entry_ptr: *mut u8 = self.data.as_ptr().add(base_offset).cast();

        std::ptr::copy(value_ptr.as_ptr(), entry_ptr, self.layout.size());

        new_cap
    }

    pub unsafe fn push_typed<T>(&mut self, cap: usize, len: usize, mut value: T) -> usize{
        let new_cap = self.push_untyped(cap, len, NonNull::new_unchecked(&mut value as *mut T).cast());
        std::mem::forget(value);
        new_cap
    }

    pub unsafe fn get_ptr_untyped(
        &mut self, index: usize, layout: Layout
    ) -> NonNull<u8>{
        self.data.add(index*layout.size())
    }

    pub unsafe fn get_typed<T>(
        &mut self, index: usize 
    ) -> &T{
        & *self.get_ptr_untyped(index, Layout::new::<T>()).cast().as_ptr()
    }

    pub unsafe fn get_mut_typed<T>(
        &mut self, index: usize 
    ) -> &mut T{
        &mut *self.get_ptr_untyped(index, Layout::new::<T>()).cast().as_ptr()
    }
}



#[cfg(test)]
mod test {
    use std::alloc::Layout;

    use super::ThinBlobVec;

    struct comp1(usize, usize, u8, u8);
    struct comp2(usize, u8, u8, u16);
    struct comp3(usize, Box<comp2>);

    #[test]
    fn it_works() {
        let mut bv = ThinBlobVec::new(Layout::new::<comp1>());
        unsafe{
           bv.push_typed(0, 0, comp1(23, 435, 2, 5));
           bv.push_typed(4, 1, comp1(23, 435, 2, 5));
           bv.push_typed(4, 2, comp1(23, 435, 2, 5));
           let cap = bv.push_typed(4, 3, comp1(23, 435, 2, 5));
           bv.push_typed(cap, 4, comp1(23, 435, 2, 5));
           let cap = bv.push_typed(0, 5, comp1(23, 435, 2, 5));
           let v : &mut comp1 = bv.get_mut_typed(0);
           let v : &mut comp1 = bv.get_mut_typed(4);

           bv.dealloc_typed::<comp1>(cap);
        }

    }
}
