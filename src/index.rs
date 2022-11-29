use std::ops::Index;
use crate::{RustType, TypeStructure, TypeStructureBody};
use crate::misc::try_index::{impl_from_try_index_and_mut, NotFound, NotFoundIndexPath, TryIndex, TryIndexMut};

/// Used to access a nested field or tuple/array element.
///
/// Types which can be indexed:
/// - Opaque tuple or field compound
/// - Struct
/// - Enum *with one only variant*
/// - Tuple
/// - Array (returns `elem` iff `index < length`)
/// - Slice (returns `elem`)
///
/// Types which cannot be indexed:
/// - Opaque
/// - Primitive
/// - Pointer
/// - Enum with zero or multiple variants
/// - Any type if the index is out of range,
///   including struct/enum types with no body and array types if `index >= length`=

impl_from_try_index_and_mut!(['a] <[usize], Output=RustType> for RustType);
impl_from_try_index_and_mut!(<usize, Output=RustType> for RustType);
impl_from_try_index_and_mut!(<usize, Output=RustType> for TypeStructure);
impl_from_try_index_and_mut!(<usize, Output=RustType> for TypeStructureBody);

impl<'a> TryIndex<&'a [usize]> for RustType {
    type Output = RustType;
    type Error = NotFoundIndexPath<&'a [usize]>;

    fn try_index(&self, index_path: &'a [usize]) -> Result<&Self::Output, Self::Error> {
        let mut result = self;
        for (path_index, index) in index_path.iter().enumerated() {
            result = result.try_index(index).map_err(|_| NotFoundIndexPath { path_index, index_path })?;
        }
        Ok(result)
    }
}

impl<'a> TryIndexMut<&'a [usize]> for RustType {
    fn try_index_mut(&mut self, index_path: &'a [usize]) -> Result<&mut Self::Output, Self::Error> {
        let mut result = self;
        for (path_index, index) in index_path.iter().enumerated() {
            result = result.try_index_mut(index).map_err(|_| NotFoundIndexPath { path_index, index_path })?;
        }
        Ok(result)
    }
}

impl TryIndex<usize> for RustType {
    type Output = RustType;

    fn try_index(&self, index: usize) -> Result<&Self::Output, NotFound<usize>> {
        self.structure.try_index(index)
    }
}

impl TryIndex<usize> for TypeStructure {
    type Output = RustType;

    fn try_index(&self, index: usize) -> Result<&Self::Output, NotFound<usize>> {
        match self {
            TypeStructure::Opaque |
            TypeStructure::Primitive(_) |
            TypeStructure::Pointer { .. } => Err(NotFound { index }),
            TypeStructure::OpaqueTuple { elements } => elements.try_index(index),
            TypeStructure::OpaqueFields { fields } => fields.try_index(index).map(|field| &field.rust_type),
            TypeStructure::CReprStruct { body } => body.try_index(index),
            TypeStructure::CReprEnum { variants } => if variants.len() == 1 {
                variants[0].body.try_index(index)
            } else {
                Err(NotFound { index })
            }
            TypeStructure::CTuple { elements } => elements.try_index(index),
            TypeStructure::Array { elem, length } => if index < *length {
                Ok(elem.as_ref())
            } else {
                Err(NotFound { index })
            }
            TypeStructure::Slice { elem } => Ok(elem.as_ref())
        }
    }
}

impl TryIndex<usize> for TypeStructureBody {
    type Output = RustType;

    fn try_index(&self, index: usize) -> Result<&Self::Output, NotFound<usize>> {
        match self {
            TypeStructureBody::None => Err(NotFound { index }),
            TypeStructureBody::Tuple(elements) => elements.try_index(index),
            TypeStructureBody::Fields(fields) => fields.try_index(index).map(|field| &field.rust_type)
        }
    }
}

impl TryIndexMut<usize> for RustType {
    fn try_index_mut(&mut self, index: usize) -> Result<&mut Self::Output, NotFound<usize>> {
        self.structure.try_index_mut(index)
    }
}

impl TryIndexMut<usize> for TypeStructure {
    fn try_index_mut(&mut self, index: usize) -> Result<&mut Self::Output, NotFound<usize>> {
        match self {
            TypeStructure::Opaque |
            TypeStructure::Primitive(_) |
            TypeStructure::Pointer { .. } => Err(NotFound { index }),
            TypeStructure::OpaqueTuple { elements } => elements.try_index_mut(index),
            TypeStructure::OpaqueFields { fields } => fields.try_index_mut(index).map(|field| &field.rust_type),
            TypeStructure::CReprStruct { body } => body.try_index_mut(index),
            TypeStructure::CReprEnum { variants } => if variants.len() == 1 {
                variants[0].body.try_index_mut(index)
            } else {
                Err(NotFound { index })
            }
            TypeStructure::CTuple { elements } => elements.try_index_mut(index),
            TypeStructure::Array { elem, length } => if index < *length {
                Ok(elem.as_mut())
            } else {
                Err(NotFound { index })
            }
            TypeStructure::Slice { elem } => Ok(elem.as_mut())
        }
    }
}

impl TryIndexMut<usize> for TypeStructureBody {
    fn try_index_mut(&mut self, index: usize) -> Result<&mut Self::Output, NotFound<usize>> {
        match self {
            TypeStructureBody::None => Err(NotFound { index }),
            TypeStructureBody::Tuple(elements) => elements.try_index_mut(index),
            TypeStructureBody::Fields(fields) => fields.try_index_mut(index).map(|field| &field.rust_type)
        }
    }
}