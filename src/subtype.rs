use std::iter::zip;
use crate::{RustPointerKind, RustType, TypeStructureBody};
use crate::structure::{IsSubtypeOf, TypeStructure};

impl RustType {
    /// Returns true if a value of this type can be casted to the other type,
    /// where the casting rules are as follows:
    ///
    /// - If both types have a type_id, then they must be equal
    /// - Otherwise, check that one type is a structural subtype of another (see [TypeStructure::is_structural_subtype_of])
    ///
    /// If you want to compare rust types for actual structural subtyping, use [RustType::is_structural_subtype_of].
    pub fn is_rough_subtype_of(&self, other: &RustType) -> IsSubtypeOf {
        if self.type_name.is_bottom() {
            IsSubtypeOf::Yes
        } else if other.type_name.is_bottom() {
            IsSubtypeOf::No
        } else if self.type_id.is_some() && other.type_id.is_some() {
            IsSubtypeOf::known(self.type_id == other.type_id)
        } else {
            self.is_structural_subtype_of(other)
        }
    }

    /// Compares structures for subtyping. See [TypeStructure::is_structural_subtype_of] for details.
    pub fn is_structural_subtype_of(&self, other: &RustType) -> IsSubtypeOf {
        self.structure.is_structural_subtype_of(&other.structure)
    }

    /// Attempts to unify with the other type, consuming it in the process.
    ///
    /// This works just like [TypeStructure::unify], except if this type has a type_id it will not
    /// change, and if this is unknown, it will become the other type, including its name, type_id,
    /// and layout (unifying the structure wouldn't change those things).
    pub fn unify(&mut self, other: RustType) {
        if self.type_name.is_unknown() {
            *self = other;
        } else if self.type_id.is_none() {
            self.structure.unify(other.structure);
        }
    }
}

impl TypeStructure {
    /// Returns true if a value of this type can be casted to the other type. That is:
    ///
    /// - If either type is opaque, returns [IsSubtypeOf::Unknown].
    /// - If both types are tuples or tuple structures: the lengths must be equal and each corresponding element must be a subtype.
    /// - If both types are field structures: `other` may have extra fields and will still be a subtype. Shared fields must be subtypes
    /// - If both types are enums: `other` may *be missing variants* will still be a subtype. Shared variants must be subtypes
    /// - If both types are pointers: the inner type ids must be equal if both are provided, else the names must be equal; *and* the pointer kind must be a subtype:
    ///   - Immutable references are subtypes of mutable references.
    ///   - Immutable raw pointers are subtypes of mutable raw pointers.
    ///   - Raw pointers are subtypes of references, except mutable raw pointer is not a subtype of immutable reference.
    /// - If both types are arrays or slices: the element must be a subtype. If both types are arrays the length must be equal. If `self` is a slice `other` can be an array, but not vice versa.
    pub fn is_structural_subtype_of(&self, other: &TypeStructure) -> IsSubtypeOf {
        match (self, other) {
            (TypeStructure::Opaque, _) | (_, TypeStructure::Opaque) => IsSubtypeOf::Unknown,
            (TypeStructure::Primitive(primitive), TypeStructure::Primitive(other_primitive)) => {
                IsSubtypeOf::known(primitive == other_primitive)
            },
            (TypeStructure::CReprEnum { variants }, TypeStructure::CReprEnum { variants: other_variants }) => {
                variants.iter().map(|variant| {
                    match other_variants.iter().find(|other_variant| variant.variant_name == other_variant.variant_name) {
                        None => IsSubtypeOf::No,
                        Some(other_variant) => variant.body.is_structural_subtype_of(&other_variant.body)
                    }
                }).min().unwrap_or(IsSubtypeOf::Yes)
            }
            (TypeStructure::CReprStruct { body }, TypeStructure::CReprStruct { body: other_body }) => {
                body.is_structural_subtype_of(other_body)
            }
            (TypeStructure::Pointer { ptr_kind, ptr_size, refd_id, refd_name }, TypeStructure::Pointer { ptr_kind: other_ptr_kind, ptr_size: other_ptr_size, refd_id: other_refd_id, refd_name: other_refd_name }) => {
                let refd_equal = match (refd_id, other_refd_id) {
                    (Some(refd_id), Some(other_refd_id)) => refd_id == other_refd_id,
                    _ => refd_name == other_refd_name
                };
                IsSubtypeOf::known(ptr_kind.is_subtype_of(other_ptr_kind) && ptr_size == other_ptr_size && refd_equal)
            }
            (TypeStructure::CTuple { elements }, TypeStructure::CTuple { elements: other_elements }) => {
                tuple_is_subtype_of(elements, other_elements)
            }
            (TypeStructure::Array { elem, length }, TypeStructure::Array { elem: other_elem, length: other_length }) => {
                if length != other_length {
                    IsSubtypeOf::No
                } else {
                    elem.is_rough_subtype_of(other_elem)
                }
            }
            (TypeStructure::Array { elem, length: _ }, TypeStructure::Slice { elem: other_elem }) |
            (TypeStructure::Slice { elem }, TypeStructure::Slice { elem: other_elem }) => {
                elem.is_rough_subtype_of(other_elem)
            }
            _ => IsSubtypeOf::No
        }
    }

    /// Attempts to unify with the other type, consuming it in the process: Unification may
    /// - Replace the type if it's opaque
    /// - And add more fields or *subtract* enum variants
    /// - Add type id to pointer
    /// - Add length to a slice, converting it into an array
    ///
    /// If these types are definitely different, than [unify] will not cause any changes.
    /// If you want different behavior (e.g. an error or bottom type), check
    /// [is_structural_subtype_of].
    pub fn unify(&mut self, other: TypeStructure) {
        // Can't put in match expr because of borrowing rules
        if matches!(self, TypeStructure::Opaque) {
            *self = other;
            return;
        }
        if let TypeStructure::Slice { elem } = self {
            match other {
                TypeStructure::Array { elem: other_elem, length: other_length } => {
                    elem.unify(*other_elem);
                    *self = TypeStructure::Array { elem: elem.clone(), length: other_length };
                    return;
                }
                _ => {}
            }
        }

        match (self, other) {
            (TypeStructure::Opaque, _) => unreachable!(),
            (TypeStructure::CReprEnum { variants }, TypeStructure::CReprEnum { variants: mut other_variants }) => {
                let _ = variants.drain_filter(|variant| {
                    if let Some(other_variant_idx) = other_variants.iter().position(|other_variant| variant.variant_name == other_variant.variant_name) {
                        let other_variant = other_variants.remove(other_variant_idx);
                        variant.body.unify(other_variant.body);
                        false
                    } else {
                        true
                    }
                });
            },
            (TypeStructure::CReprStruct { body }, TypeStructure::CReprStruct { body: other_body }) => {
                body.unify(other_body);
            }
            (TypeStructure::Pointer { ptr_kind, ptr_size: _, refd_id, refd_name: _ }, TypeStructure::Pointer { ptr_kind: other_ptr_kind, ptr_size: _, refd_id: other_refd_id, refd_name: _ }) => {
                ptr_kind.unify(other_ptr_kind);
                if refd_id.is_none() {
                    *refd_id = other_refd_id;
                }
                // Type names don't unify (TODO: may resolve unknowns later)
            }
            (TypeStructure::CTuple { elements }, TypeStructure::CTuple { elements: other_elements }) => {
                for (element, other_element) in elements.iter_mut().zip(other_elements.into_iter()) {
                    element.unify(other_element);
                }
            }
            (TypeStructure::Array { elem, length: _ }, TypeStructure::Array { elem: other_elem, length: _ }) => {
                elem.unify(*other_elem);
            }
            (TypeStructure::Slice { elem }, TypeStructure::Slice { elem: other_elem }) => {
                elem.unify(*other_elem)
            }
            _ => {}
        }
    }
}

impl TypeStructureBody {
    fn is_structural_subtype_of(&self, other: &TypeStructureBody) -> IsSubtypeOf {
        match (self, other) {
            (TypeStructureBody::None, TypeStructureBody::None) => IsSubtypeOf::Yes,
            (TypeStructureBody::Tuple(elements), TypeStructureBody::Tuple(other_elements)) => {
                tuple_is_subtype_of(elements, other_elements)
            }
            (TypeStructureBody::Fields(fields), TypeStructureBody::Fields(other_fields)) => {
                if fields.len() < other_fields.len() {
                    IsSubtypeOf::No
                } else {
                    other_fields.iter().map(|other_field| {
                        match fields.iter().find(|field| field.name == other_field.name) {
                            None => IsSubtypeOf::No,
                            Some(field) => field.rust_type.is_rough_subtype_of(&other_field.rust_type)
                        }
                    }).min().unwrap_or(IsSubtypeOf::Yes)
                }
            }
            _ => IsSubtypeOf::No
        }
    }

    fn unify(&mut self, other: TypeStructureBody) {
        match (self, other) {
            (TypeStructureBody::Tuple(elements), TypeStructureBody::Tuple(other_elements)) => {
                for (element, other_element) in elements.iter_mut().zip(other_elements.into_iter()) {
                    element.unify(other_element);
                }
            }
            (TypeStructureBody::Fields(fields), TypeStructureBody::Fields(mut other_fields)) => {
                for field in fields.iter_mut() {
                    if let Some(other_field_idx) = other_fields.iter().position(|other_field| field.name == other_field.name) {
                        let other_field = other_fields.remove(other_field_idx);
                        field.rust_type.unify(other_field.rust_type);
                    }
                }
            }
            _ => {}
        }
    }
}

impl RustPointerKind {
    fn is_subtype_of(&self, other: &RustPointerKind) -> bool {
        match (self, other) {
            (RustPointerKind::ImmRaw, RustPointerKind::ImmRaw) => true,
            (RustPointerKind::ImmRaw, RustPointerKind::MutRaw) => false,
            (RustPointerKind::ImmRaw, RustPointerKind::ImmRef) => false,
            (RustPointerKind::ImmRaw, RustPointerKind::MutRef) => false,
            (RustPointerKind::MutRaw, RustPointerKind::ImmRaw) => true,
            (RustPointerKind::MutRaw, RustPointerKind::MutRaw) => true,
            (RustPointerKind::MutRaw, RustPointerKind::ImmRef) => false,
            (RustPointerKind::MutRaw, RustPointerKind::MutRef) => false,
            (RustPointerKind::ImmRef, RustPointerKind::ImmRaw) => true,
            (RustPointerKind::ImmRef, RustPointerKind::MutRaw) => false,
            (RustPointerKind::ImmRef, RustPointerKind::ImmRef) => true,
            (RustPointerKind::ImmRef, RustPointerKind::MutRef) => false,
            (RustPointerKind::MutRef, RustPointerKind::ImmRaw) => true,
            (RustPointerKind::MutRef, RustPointerKind::MutRaw) => true,
            (RustPointerKind::MutRef, RustPointerKind::ImmRef) => true,
            (RustPointerKind::MutRef, RustPointerKind::MutRef) => true,
        }
    }

    fn unify(&mut self, other: RustPointerKind) {
        *self = match (*self, other) {
            (RustPointerKind::ImmRaw, RustPointerKind::ImmRaw) => RustPointerKind::ImmRaw,
            (RustPointerKind::ImmRaw, RustPointerKind::MutRaw) => RustPointerKind::ImmRaw,
            (RustPointerKind::ImmRaw, RustPointerKind::ImmRef) => RustPointerKind::ImmRaw,
            (RustPointerKind::ImmRaw, RustPointerKind::MutRef) => RustPointerKind::ImmRaw,
            (RustPointerKind::MutRaw, RustPointerKind::ImmRaw) => RustPointerKind::ImmRaw,
            (RustPointerKind::MutRaw, RustPointerKind::MutRaw) => RustPointerKind::MutRaw,
            (RustPointerKind::MutRaw, RustPointerKind::ImmRef) => RustPointerKind::ImmRaw,
            (RustPointerKind::MutRaw, RustPointerKind::MutRef) => RustPointerKind::MutRaw,
            (RustPointerKind::ImmRef, RustPointerKind::ImmRaw) => RustPointerKind::ImmRaw,
            (RustPointerKind::ImmRef, RustPointerKind::MutRaw) => RustPointerKind::ImmRaw,
            (RustPointerKind::ImmRef, RustPointerKind::ImmRef) => RustPointerKind::ImmRef,
            (RustPointerKind::ImmRef, RustPointerKind::MutRef) => RustPointerKind::ImmRef,
            (RustPointerKind::MutRef, RustPointerKind::ImmRaw) => RustPointerKind::ImmRaw,
            (RustPointerKind::MutRef, RustPointerKind::MutRaw) => RustPointerKind::MutRaw,
            (RustPointerKind::MutRef, RustPointerKind::ImmRef) => RustPointerKind::ImmRef,
            (RustPointerKind::MutRef, RustPointerKind::MutRef) => RustPointerKind::MutRef,
        }
    }
}

fn tuple_is_subtype_of(elements: &Vec<RustType>, other_elements: &Vec<RustType>) -> IsSubtypeOf {
    if elements.len() != other_elements.len() {
        IsSubtypeOf::No
    } else {
        zip(elements.iter(), other_elements.iter()).map(|(element, other_element)| {
            element.is_rough_subtype_of(other_element)
        }).min().unwrap_or(IsSubtypeOf::Yes)
    }
}

impl IsSubtypeOf {
    pub fn known(x: bool) -> Self {
        if x {
            IsSubtypeOf::Yes
        } else {
            IsSubtypeOf::No
        }
    }
}

