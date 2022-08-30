#![doc = include_str!("../README.md")]
#![feature(decl_macro)]
#![feature(drain_filter)]
#![feature(associated_type_defaults)]

use std::any::TypeId;
#[cfg(feature = "registry")]
use std::borrow::Cow;
use std::mem::{align_of, size_of};

/// Generic tuple structs with a guaranteed `C` repr.
///
/// They have the same [RustTypeName] as tuples, because when dealing with reflection you often want
/// these instead of real tuples (real tuples have undefined layout).
pub mod c_tuple;
/// Proc macro derives for [HasTypeName] and [HasStructure]. Requires the `derive` feature
#[cfg(feature = "derive")]
pub mod derive;
mod type_name;
mod has_structure;
mod structure;
mod primitive;
#[cfg(feature = "registry")]
mod registry;
mod subtype;
mod size_align;
mod intrinsic;

pub use type_name::*;
pub use has_structure::*;
pub use structure::*;
pub use primitive::*;
#[cfg(feature = "registry")]
pub use registry::*;
pub use subtype::*;
pub use size_align::*;
pub use intrinsic::*;

/// Name, structure, type id, and layout info for a rust type.
/// May not be an actual rust type, but a defined in another language or external dependency.
///
/// Rust types are considered equal if both types have the same type id or name.
/// To check for subtyping, use [RustType::is_rough_subtype_of] or [RustType::is_structural_subtype_of].
#[derive(Debug, Clone)]
pub struct RustType {
    /// Corresponds to [TypeId::of]. Types in external libraries may not have ids.
    pub type_id: Option<TypeId>,
    /// Corresponds to [std::any::type_name], but more detailed and specified.
    pub type_name: RustTypeName,
    /// Corresponds to [std::size_of]
    pub size: usize,
    /// Corresponds to [std::align_of]
    pub align: usize,
    /// Structure
    pub structure: TypeStructure
}

impl RustType {
    /// Returns the type containing metadata of `T`.
    ///
    /// Tries to add this type data to the singleton registry if the crate feature `registry` is enabled,
    /// otherwise this is equivalent to [RustType::of_dont_register]
    pub fn of<T: HasStructure>() -> Self where T::Static: Sized {
        let rust_type = RustType::of_dont_register::<T>();
        #[cfg(feature = "registry")]
        Self::register(Cow::Borrowed(&rust_type), Some(IntrinsicRustType::of::<T>()));
        rust_type
    }

    /// Returns the type containing metadata of `T`, and doesn't try to add to the singleton registry
    pub fn of_dont_register<T: HasStructure>() -> Self {
        RustType {
            type_id: None,
            type_name: T::type_name(),
            size: size_of::<T>(),
            align: align_of::<T>(),
            structure: T::structure()
        }
    }

    /// Returns the unknown type
    pub fn unknown() -> Self {
        RustType {
            type_id: None,
            type_name: RustTypeName::unknown(),
            size: 0,
            align: 0,
            structure: TypeStructure::Opaque
        }
    }

    /// Returns the bottom type (subtype of everything including itself)
    pub fn bottom() -> Self {
        RustType {
            type_id: None,
            type_name: RustTypeName::bottom(),
            size: 0,
            align: 0,
            structure: TypeStructure::Opaque
        }
    }

    /// Returns a type with the intrinsic type id and layout, the type name, and opaque structure
    pub fn from_intrinsic(type_name: RustTypeName, data: IntrinsicRustType) -> Self {
        RustType {
            type_id: Some(data.type_id),
            type_name,
            size: data.size,
            align: data.align,
            structure: TypeStructure::Opaque
        }
    }

    /// Displays the type name
    #[must_use = "this does not display the type name, it returns an object that can be displayed"]
    pub fn display<'a, 'b>(&'a self, dnis: &'b DuplicateNamesInScope) -> RustTypeNameDisplay<'a, 'b> {
        self.type_name.display(dnis)
    }
}

/// Considered equal if both types have the same type id or name
impl PartialEq for RustType {
    fn eq(&self, other: &RustType) -> bool {
        if self.type_id.is_some() && other.type_id.is_some() {
            self.type_id == other.type_id
        } else {
            self.type_name == other.type_name
        }
    }
}

impl Eq for RustType {}
