use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::mem::MaybeUninit;


#[macro_export]
macro_rules! max_array {
    () => {
        MaxArray::new()
    };
    ($item:expr $(, $another_item:expr)* $(,)?) => {
        {
            let mut max_array = MaxArray::new();
            max_array.push($item);
            $(
                max_array.push($another_item);
            )*
            max_array
        }
    };
}


/// Array with an upper bounded size.
pub struct MaxArray<T, const MAX_N: usize> {
    array: [MaybeUninit<T>; MAX_N],
    length: usize,
}
impl<T, const MAX_N: usize> MaxArray<T, MAX_N> {
    pub const fn new() -> Self {
        let array = [const { MaybeUninit::uninit() }; MAX_N];
        Self {
            array,
            length: 0,
        }
    }

    pub const fn as_slice(&self) -> &[T] {
        let array_mu_ptr = self.array.as_ptr();
        let array_ptr = array_mu_ptr as *const T;
        unsafe { std::slice::from_raw_parts(array_ptr, self.length) }
    }

    pub const fn as_slice_mut(&mut self) -> &mut [T] {
        let array_mu_ptr = self.array.as_mut_ptr();
        let array_ptr = array_mu_ptr as *mut T;
        unsafe { std::slice::from_raw_parts_mut(array_ptr, self.length) }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn try_push(&mut self, new_item: T) -> bool {
        if self.length >= self.array.len() {
            false
        } else {
            self.array[self.length] = MaybeUninit::new(new_item);
            self.length += 1;
            true
        }
    }

    pub fn push(&mut self, new_item: T) {
        if !self.try_push(new_item) {
            panic!("MaxArray is full");
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            None
        } else {
            let last_elem_mu = std::mem::replace(&mut self.array[self.length-1], MaybeUninit::uninit());
            self.length -= 1;
            let last_elem = unsafe { last_elem_mu.assume_init() };
            Some(last_elem)
        }
    }
}
impl<T, const MAX_N: usize> AsRef<[T]> for MaxArray<T, MAX_N> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T, const MAX_N: usize> AsMut<[T]> for MaxArray<T, MAX_N> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_slice_mut()
    }
}
impl<T, const MAX_N: usize> Drop for MaxArray<T, MAX_N> {
    fn drop(&mut self) {
        // replace each extant item with an uninit item,
        // then assume the extant item is initialized (because it is), whereupon it is dropped
        for item in &mut self.array[..self.length] {
            let current_item = std::mem::replace(item, MaybeUninit::uninit());
            unsafe { current_item.assume_init() };
        }
    }
}
impl<T: Clone, const MAX_N: usize> Clone for MaxArray<T, MAX_N> {
    fn clone(&self) -> Self {
        let mut ret = Self::new();
        for item in self.as_slice() {
            ret.push(item.clone());
        }
        ret
    }
}
impl<T: Debug, const MAX_N: usize> Debug for MaxArray<T, MAX_N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lister = f.debug_list();
        for item in self.as_slice() {
            lister.entry(item);
        }
        lister.finish()
    }
}
impl<T, const MAX_N: usize> Default for MaxArray<T, MAX_N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Hash, const MAX_N: usize> Hash for MaxArray<T, MAX_N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for item in self.as_slice() {
            item.hash(state);
        }
    }
}
impl<T: PartialEq, const MAX_N: usize> PartialEq for MaxArray<T, MAX_N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: PartialOrd, const MAX_N: usize> PartialOrd for MaxArray<T, MAX_N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}
impl<T: Eq, const MAX_N: usize> Eq for MaxArray<T, MAX_N> {
}
impl<T: Ord, const MAX_N: usize> Ord for MaxArray<T, MAX_N> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}
