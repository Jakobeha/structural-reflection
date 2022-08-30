use std::any::TypeId;
use std::fmt::{Display, Formatter};
use crate::{PrimitiveType, RustPointerKind, RustType, RustTypeName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeStructure {
    /// Structure is unknown or cannot be interpreted structurally
    Opaque,
    /// Primitive type
    Primitive(PrimitiveType),
    /// `#[repr(C)]` enum
    CReprEnum { variants: Vec<TypeEnumVariant> },
    /// `#[repr(C)]` or `#[repr(transparent)]` struct
    CReprStruct { body: TypeStructureBody },
    /// Note: these are "technically" not actual tuples, as tuples in Rust have no defined repr.
    /// Thus in order to use them in Rust, you must either assume C-style repr or coerce to a C-repr struct.
    CTuple { elements: Vec<RustType> },
    /// Array with known length
    Array { elem: Box<RustType>, length: usize },
    /// Array with unknown length
    Slice { elem: Box<RustType> },
    /// Thin pointer (only thin pointers are supported)
    Pointer {
        /// Pointer kind, as mutable pointers are not subtypes of immutable pointers and raw pointers are not subtypes of references
        ptr_kind: RustPointerKind,
        refd_id: Option<TypeId>,
        /// Remember: we don't need refd structure because it doesn't affect the pointer size.
        refd_name: RustTypeName
    }
}

/// Enum variant
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeEnumVariant {
    pub variant_name: String,
    pub body: TypeStructureBody
}

/// Struct or enum variant body
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeStructureBody {
    /// This is a unit struct or variant without associated values
    None,
    /// This is a tuple struct or variant
    Tuple(Vec<RustType>),
    /// This is a field struct or variant
    Fields(Vec<TypeStructureBodyField>)
}

/// Is a unit, tuple, or field struct/variant?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeStructureBodyForm {
    None,
    Tuple,
    Fields
}

/// Field in a field struct or variant
#[derive(Debug, Clone, PartialEq)]
pub struct TypeStructureBodyField {
    pub name: String,
    pub rust_type: RustType
}

/// Is lhs a subtype of rhs?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsSubtypeOf {
    /// No
    No,
    /// We don't know (e.g. one of the types is opaque)
    Unknown,
    /// Yes
    Yes,
}

impl TypeStructure {
    /// If this is an array, returns the element type and length
    pub fn array_elem_type_and_length(&self) -> Option<(&RustType, usize)> {
        match self {
            TypeStructure::Array { elem, length } => Some((elem, *length)),
            _ => None
        }
    }

    /// If this is an array or slice, returns the element type
    pub fn array_slice_elem_type(&self) -> Option<&RustType> {
        match self {
            TypeStructure::Array { elem, length: _ } => Some(elem),
            TypeStructure::Slice { elem } => Some(elem),
            _ => None
        }
    }

    /// If this is a tuple, returns the element types
    pub fn tuple_elem_types(&self) -> Option<&Vec<RustType>> {
        match self {
            TypeStructure::CTuple { elements } => Some(elements),
            _ => None
        }
    }

    /// If this is a tuple struct, returns the element types
    pub fn tuple_struct_tuple_item_types(&self) -> Option<&Vec<RustType>> {
        match self {
            TypeStructure::CReprStruct { body: TypeStructureBody::Tuple(tuple_items) } => Some(tuple_items),
            _ => None
        }
    }

    /// If this is a tuple or tuple struct, returns the element types
    pub fn tuple_or_struct_tuple_item_types(&self) -> Option<&Vec<RustType>> {
        match self {
            TypeStructure::CTuple { elements } => Some(elements),
            TypeStructure::CReprStruct { body: TypeStructureBody::Tuple(tuple_items) } => Some(tuple_items),
            _ => None
        }
    }

    /// If this is a field struct, returns the field types
    pub fn field_struct_field_types(&self) -> Option<&Vec<TypeStructureBodyField>> {
        match self {
            TypeStructure::CReprStruct { body: TypeStructureBody::Fields(fields) } => Some(fields),
            _ => None
        }
    }
}

impl TypeStructureBody {
    pub fn form(&self) -> TypeStructureBodyForm {
        match self {
            TypeStructureBody::None => TypeStructureBodyForm::None,
            TypeStructureBody::Tuple(_) => TypeStructureBodyForm::Tuple,
            TypeStructureBody::Fields(_) => TypeStructureBodyForm::Fields
        }
    }
}

impl Display for TypeStructureBodyForm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeStructureBodyForm::None => write!(f, "none"),
            TypeStructureBodyForm::Tuple => write!(f, "tuple items"),
            TypeStructureBodyForm::Fields => write!(f, "fields")
        }
    }
}

impl Default for TypeStructureBody {
    fn default() -> Self {
        TypeStructureBody::None
    }
}

impl Default for TypeStructureBodyForm {
    fn default() -> Self {
        TypeStructureBodyForm::None
    }
}
