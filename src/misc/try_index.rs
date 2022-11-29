use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::Hash;
use derive_more::{Display, Error};

pub macro impl_from_try_index(<$index:ty, Output=$output:ty> for $ty:ty) {
    impl ::std::ops::Index<$index> for $ty {
        type Output = $output;

        fn index(&self, index: $index) -> &Self::Output {
            self.try_index(index).unwrap()
        }
    }
}

pub macro impl_from_try_index_and_mut($([$($a:lifetime),*])? <$index:ty, Output=$output:ty> for $ty:ty) {
impl$(<$($a),*>)? ::std::ops::Index<$($(&$a),*)? $index> for $ty {
    type Output = $output;

    fn index(&self, index: $($(&$a),*)? $index) -> &Self::Output {
        self.try_index(index).unwrap()
    }
}

impl$(<$($a),*>)? ::std::ops::IndexMut<$($(&$a),*)? $index> for $ty {
    fn index(&self, index: $($(&$a),*)? $index) -> &Self::Output {
        self.try_index_mut(index).unwrap()
    }
}
}

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[display(fmt = "not found: {}", index)]
pub struct NotFound<Idx> {
    pub index: Idx
}

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[display(fmt = "not found at {}: path={}", path_index, index_path)]
pub struct NotFoundIndexPath<Idx> {
    pub path_index: usize,
    pub index_path: Idx
}

pub trait TryIndex<Idx> {
    type Output;
    type Error = NotFound<Idx>;

    fn try_index(&self, index: Idx) -> Result<&Self::Output, Self::Error>;
}

pub trait TryIndexMut<Idx>: TryIndex<Idx> {
    fn try_index_mut(&mut self, index: Idx) -> Result<&mut Self::Output, Self::Error>;
}

impl<'a, T> TryIndex<usize> for &'a [T] {
    type Output = T;

    fn try_index(&self, index: usize) -> Result<&Self::Output, NotFound<usize>> {
        self.get(index).ok_or(NotFound { index })
    }
}

impl<'a, T> TryIndex<usize> for &'a mut [T] {
    type Output = T;

    fn try_index(&self, index: usize) -> Result<&Self::Output, NotFound<usize>> {
        self.get(index).ok_or(NotFound { index })
    }
}

impl<'a, T> TryIndexMut<usize> for &'a mut [T] {
    fn try_index_mut(&mut self, index: usize) -> Result<&mut Self::Output, NotFound<usize>> {
        self.get_mut(index).ok_or(NotFound { index })
    }
}

impl<T> TryIndex<usize> for Vec<T> {
    type Output = T;

    fn try_index(&self, index: usize) -> Result<&Self::Output, NotFound<usize>> {
        self.get(index).ok_or(NotFound { index })
    }
}

impl<T> TryIndexMut<usize> for Vec<T> {
    fn try_index_mut(&mut self, index: usize) -> Result<&mut Self::Output, NotFound<usize>> {
        self.get_mut(index).ok_or(NotFound { index })
    }
}

impl<T> TryIndex<usize> for VecDeque<T> {
    type Output = T;

    fn try_index(&self, index: usize) -> Result<&Self::Output, NotFound<usize>> {
        self.get(index).ok_or(NotFound { index })
    }
}

impl<T> TryIndexMut<usize> for VecDeque<T> {
    fn try_index_mut(&mut self, index: usize) -> Result<&mut Self::Output, NotFound<usize>> {
        self.get_mut(index).ok_or(NotFound { index })
    }
}

impl<K: Ord, V> TryIndex<K> for BTreeMap<K, V> {
    type Output = V;

    fn try_index(&self, index: K) -> Result<&Self::Output, NotFound<K>> {
        self.get(&index).ok_or(NotFound { index })
    }
}

impl<K: Ord, V> TryIndexMut<K> for BTreeMap<K, V> {
    fn try_index_mut(&mut self, index: K) -> Result<&mut Self::Output, NotFound<K>> {
        self.get_mut(&index).ok_or(NotFound { index })
    }
}

impl<'a, K: Eq + Hash, V> TryIndex<&'a K> for HashMap<K, V> {
    type Output = V;

    fn try_index(&self, index: &'a K) -> Result<&Self::Output, NotFound<&'a K>> {
        self.get(&index).ok_or(NotFound { index })
    }
}

impl<'a, K: Eq + Hash, V> TryIndexMut<&'a K> for HashMap<K, V> {
    fn try_index_mut(&mut self, index: &'a K) -> Result<&mut Self::Output, NotFound<&'a K>> {
        self.get_mut(&index).ok_or(NotFound { index })
    }
}