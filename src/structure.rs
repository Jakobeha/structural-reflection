use std::any::TypeId;
use std::fmt::{Display, Formatter};
use std::iter::{empty, repeat, repeat_n};
use crate::{PrimitiveType, RustPointerKind, RustType, RustTypeName};
use auto_enums::auto_enum;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeStructure {
    /// Structure is unknown or cannot be interpreted structurally
    Opaque,
    /// Structure is a tuple, array, slice, struct, or enum variant of the given elements
    /// (array or slice means elements must all be the same "real" type, though they may be different subtypes)
    OpaqueTuple { elements: Vec<RustType> },
    /// Structure is a struct or enum variant with the given fields
    OpaqueFields { fields: Vec<TypeStructureBodyField> },
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
    /// Pointer
    Pointer {
        /// Pointer kind, as mutable pointers are not subtypes of immutable pointers and raw pointers are not subtypes of references
        ptr_kind: RustPointerKind,
        /// Size of pointer including metadata. For thin pointers this is `size_of::<*const ()>()`
        ptr_size: usize,
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// - If `TypeStructureBody::Unit`, returns `TypeStructure::Opaque`
    /// - If `TypeStructureBody::Tuple`, returns `TypeStructure::OpaqueTuple`
    /// - If `TypeStructureBody::Fields`, returns `TypeStructure::OpaqueFields`
    pub fn opaque(body: TypeStructureBody) -> Self {
        match body {
            TypeStructureBody::None => TypeStructure::Opaque,
            TypeStructureBody::Tuple(elements) => TypeStructure::OpaqueTuple { elements },
            TypeStructureBody::Fields(fields) => TypeStructure::OpaqueFields { fields }
        }
    }

    /// If this is an array, returns the element type and length
    pub fn array_elem_type_and_length(&self) -> Option<(&RustType, usize)> {
        match self {
            TypeStructure::Array { elem, length } => Some((elem, *length)),
            _ => None
        }
    }

    /// If this is a slice, returns the element type
    pub fn slice_elem_type(&self) -> Option<&RustType> {
        match self {
            TypeStructure::Slice { elem } => Some(elem),
            _ => None
        }
    }

    /// If this is an array or slice, returns the element type
    pub fn array_or_slice_elem_type(&self) -> Option<&RustType> {
        match self {
            TypeStructure::Array { elem, length: _ } => Some(elem),
            TypeStructure::Slice { elem } => Some(elem),
            _ => None
        }
    }

    /// If this is a c-tuple, returns the element types
    pub fn tuple_elem_types(&self) -> Option<&Vec<RustType>> {
        match self {
            TypeStructure::CTuple { elements } => Some(elements),
            _ => None
        }
    }

    /// If this is a tuple struct, returns the element types
    pub fn tuple_struct_item_types(&self) -> Option<&Vec<RustType>> {
        match self {
            TypeStructure::CReprStruct { body: TypeStructureBody::Tuple(tuple_items) } => Some(tuple_items),
            _ => None
        }
    }

    /// If this is an opaque tuple, array, slice, c-tuple, tuple struct, or enum with one tuple variant, returns the element types.
    ///
    /// If this is a slice, will return infinite items
    #[auto_enum]
    pub fn general_tuple_item_types(&self) -> Option<impl Iterator<Item=&RustType>> {
        #[auto_enum(Iterator)]
        let result = match self {
            TypeStructure::OpaqueTuple { elements } => elements.iter(),
            TypeStructure::CTuple { elements } => elements.iter(),
            TypeStructure::CReprStruct { body: TypeStructureBody::Tuple(tuple_items) } => tuple_items.iter(),
            #[nested]
            TypeStructure::CReprEnum { variants } => match variants.len() {
                #[nested]
                1 => match &variants[0].body {
                    TypeStructureBody::Tuple(tuple_items) => tuple_items.iter(),
                    _ => None?
                }
                _ => None?
            }
            TypeStructure::Array { elem, length } => repeat_n(elem.as_ref(), *length),
            TypeStructure::Slice { elem } => repeat(elem.as_ref()),
            _ => None?
        };
        Some(result)
    }

    /// If this is an opaque tuple, array, slice, c-tuple, tuple struct, or enum with one tuple variant, returns the element types.
    ///
    /// If this is a slice, will return a vec of length `slice_len`
    /// (use [TypeStructure::general_tuple_item_types] to return infinite iterator)
    #[auto_enum]
    pub fn general_tuple_item_types2(&self, slice_len: usize) -> Option<impl ExactSizeIterator<Item=&RustType>> {
        #[auto_enum(ExactSizeIterator)]
        let result = match self {
            TypeStructure::OpaqueTuple { elements } => elements.iter(),
            TypeStructure::CTuple { elements } => elements.iter(),
            TypeStructure::CReprStruct { body: TypeStructureBody::Tuple(tuple_items) } => tuple_items.iter(),
            #[nested]
            TypeStructure::CReprEnum { variants } => match variants.len() {
                #[nested]
                1 => match &variants[0].body {
                    TypeStructureBody::Tuple(tuple_items) => tuple_items.iter(),
                    _ => None?
                }
                _ => None?
            }
            TypeStructure::Array { elem, length } => repeat_n(elem.as_ref(), *length),
            TypeStructure::Slice { elem } => repeat_n(elem.as_ref(), slice_len),
            _ => None?
        };
        Some(result)
    }

    /// If this is a field struct, returns the field types
    pub fn field_struct_field_types(&self) -> Option<&Vec<TypeStructureBodyField>> {
        match self {
            TypeStructure::CReprStruct { body: TypeStructureBody::Fields(fields) } => Some(fields),
            _ => None
        }
    }

    /// If this is a struct or enum with exactly one field variant, returns the field types.
    pub fn general_field_compound_field_types(&self) -> Option<&Vec<TypeStructureBodyField>> {
        match self {
            TypeStructure::OpaqueFields { fields } => Some(fields),
            TypeStructure::CReprStruct { body: TypeStructureBody::Fields(fields) } => Some(fields),
            TypeStructure::CReprEnum { variants } if variants.len() == 1 => match &variants[0].body {
                TypeStructureBody::Fields(fields) => Some(fields),
                _ => None
            }
            _ => None
        }
    }

    /// If this is a tuple, struct, array, or enum with exactly one variant, returns the number of elements
    ///
    /// Note that for slice, [TypeStructure::general_compound_elem_types] will return an infinite iterator,
    /// but [TypeStructure::general_compound_elem_length] is `None`.
    pub fn general_compound_length(&self) -> Option<usize> {
        match self {
            TypeStructure::OpaqueTuple { elements } => Some(elements.len()),
            TypeStructure::OpaqueFields { fields } => Some(fields.len()),
            TypeStructure::CTuple { elements } => Some(elements.len()),
            TypeStructure::CReprStruct { body } => Some(body.general_compound_len()),
            TypeStructure::CReprEnum { variants } if variants.len() == 1 => Some(variants[0].body.general_compound_len()),
            TypeStructure::Array { elem: _, length } => Some(*length),
            _ => None
        }
    }

    /// If this is a tuple, struct, array, slice, or enum with exactly one variant, returns the element types.
    ///
    /// Note that for slice, [TypeStructure::general_compound_elem_types] will return an infinite iterator,
    /// but [TypeStructure::general_compound_elem_length] is `None`.
    #[auto_enum]
    pub fn general_compound_elem_types(&self) -> Option<impl Iterator<Item=&RustType>> {
        #[auto_enum(Iterator)]
        let result = match self {
            TypeStructure::OpaqueTuple { elements } => elements.iter(),
            TypeStructure::OpaqueFields { fields } => fields.iter().map(|field| &field.rust_type),
            TypeStructure::CTuple { elements } => elements.iter(),
            TypeStructure::CReprStruct { body } => body.general_compound_elem_types(),
            #[nested]
            TypeStructure::CReprEnum { variants } => match variants.len() {
                1 => variants[0].body.general_compound_elem_types(),
                _ => None?
            },
            TypeStructure::Array { elem, length } => repeat_n(elem.as_ref(), *length),
            TypeStructure::Slice { elem } => repeat(elem.as_ref()),
            _ => None?
        };
        Some(result)
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

    fn general_compound_len(&self) -> usize {
        match self {
            TypeStructureBody::None => 0,
            TypeStructureBody::Tuple(tuple_items) => tuple_items.len(),
            TypeStructureBody::Fields(fields) => fields.len()
        }
    }

    #[auto_enum(Iterator)]
    fn general_compound_elem_types(&self) -> impl Iterator<Item=&RustType> {
        match self {
            TypeStructureBody::Tuple(tuple_items) => tuple_items.iter(),
            TypeStructureBody::Fields(fields) => fields.iter().map(|field| &field.rust_type),
            TypeStructureBody::None => empty()
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
