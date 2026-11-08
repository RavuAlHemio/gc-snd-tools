use arrayvec::ArrayVec;


pub(crate) struct ArrayVecSlice<'a, T> {
    slice: &'a [T],
    capacity: usize,
}
impl<'a, T> ArrayVecSlice<'a, T> {
    pub fn new<const CAP: usize>(av: &'a ArrayVec<T, CAP>) -> Self {
        Self {
            slice: av.as_slice(),
            capacity: av.capacity(),
        }
    }

    pub const fn capacity(&self) -> usize { self.capacity }
}
impl<'a, T> AsRef<[T]> for ArrayVecSlice<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.slice
    }
}
