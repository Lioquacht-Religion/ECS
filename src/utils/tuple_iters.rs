// tuple_iters.rs

use crate::{
    all_tuples,
    ecs::{component::Component, entity::EntityKeyIterUnsafe},
};

pub trait TupleIterator {
    type Item;
    ///SAFETY: This function does not check if iterator is still in the valid range.
    /// Bounds check needs to be tracked from outside the function.
    unsafe fn next(&mut self, index: usize) -> Self::Item;
}

pub trait TupleConstructorSource: 'static {
    type IterType<'c, T: Component>: TupleIterator
    where
        Self: 'c;
    type IterMutType<'c, T: Component>: TupleIterator
    where
        Self: 'c;
    fn get_entity_key_iter<'c>(&'c mut self) -> EntityKeyIterUnsafe<'c>;
    unsafe fn get_iter<'c, T: Component>(&'c mut self) -> Self::IterType<'c, T>;
    unsafe fn get_iter_mut<'c, T: Component>(&'c mut self) -> Self::IterMutType<'c, T>;
}

pub trait TupleIterConstructor<S: TupleConstructorSource> {
    type Construct<'c>: TupleIterator;
    unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s>;
}

impl<T: Component, S: TupleConstructorSource> TupleIterConstructor<S> for &T {
    type Construct<'c> = S::IterType<'c, T>;
    unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s> {
        (&mut *source).get_iter()
    }
}

impl<T: Component, S: TupleConstructorSource> TupleIterConstructor<S> for &mut T {
    type Construct<'c> = S::IterMutType<'c, T>;
    unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s> {
        (&mut *source).get_iter_mut()
    }
}

macro_rules! impl_tuple_iter_constructor{
    ($($t:ident), *) => {
       impl<S: TupleConstructorSource, $($t : TupleIterConstructor<S>), *> TupleIterConstructor<S> for ($($t),*,){
            #[allow(unused_parens, non_snake_case)]
            type Construct<'c> = ($($t::Construct<'c>), *);
            unsafe fn construct<'s>(source: *mut S) -> Self::Construct<'s> {
                ($($t::construct(source)), *)
            }
        }
    }
}

#[rustfmt::skip]
all_tuples!(
    impl_tuple_iter_constructor,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

macro_rules! impl_tuple_iterator{
    ($($t:ident), *) => {
       impl<$($t : TupleIterator), *> TupleIterator for ($($t),*,){

            #[allow(unused_parens, non_snake_case)]
            type Item = ($($t::Item),* );
            #[allow(unconditional_recursion)]
            unsafe fn next(&mut self, index: usize) -> Self::Item {
                #[allow(unused_parens, non_snake_case)]
                let ($($t),*) = self;
                ($($t.next(index)),*)
            }
        }
    }
}

#[rustfmt::skip]
all_tuples!(
    impl_tuple_iterator,
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

#[cfg(test)]
mod test {
    #[test]
    fn test_tuple_iter() {}
}
