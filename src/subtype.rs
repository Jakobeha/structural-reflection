use std::iter::zip;
use crate::{RustPointerKind, RustType, TypeEnumVariant, TypeStructureBody, TypeStructureBodyField};
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

    /// **Unifies** `self`, the "explicitly-provided" type, with `other`, the "inferred-by-value" type:
    /// attempts to *create the lowest common subtype of both `self` and `other`*,
    /// consuming `other` in the process and *guaranteeing that the result type is a subtype of `self` (assuming unknown = bottom)*.
    /// Prioritizes `self` when the types are completely different and there is no common subtype which is not bottom.
    ///
    /// This works just like [TypeStructure::unify], except if this type has a type_id and known size/align
    /// it will not change, and if this is unknown, it will become the other type, including its name, type_id,
    /// and layout (unifying the structure wouldn't change those things).
    pub fn unify(&mut self, other: RustType) {
        if self.type_name.is_unknown() {
            *self = other;
        } else if self.type_id.is_none() || self.size == usize::MAX || self.align == usize::MAX {
            self.structure.unify(other.structure);
            if self.size == usize::MAX {
                self.size = self.structure.infer_size().unwrap_or(other.size);
            }
            if self.align == usize::MAX {
                self.align = self.structure.infer_align().unwrap_or(other.align);
            }
        }
    }
}

impl TypeStructure {
    /// Returns true if a value of this type can be casted (e.g. assigned) to the other type. That is:
    ///
    /// - If either type is opaque: returns [IsSubtypeOf::Unknown].
    /// - If either type is an opaque tuple: if the other type is a tuple and elements may be subtypes, returns [IsSubtypeOf::Unknown], otherwise [IsSubtypeOf::No].
    /// - If either type is an opaque field compound: if the other type is a field compound and for each of `self`'s fields one of `other`'s fields may be a subtype, returns [IsSubtypeOf::Unknown], otherwise [IsSubtypeOf::No].
    /// - If both types are tuples or tuple structures: the lengths must be equal and each corresponding element must be a subtype.
    /// - If both types are field structures: `self` may have extra fields and will still be a subtype. Shared fields must be subtypes
    /// - If both types are enums: `self` may *be missing variants* will still be a subtype. Shared variants must be subtypes
    /// - If both types are pointers: the inner type ids must be equal if both are provided, else the names must be equal; *and* the pointer kind must be a subtype:
    ///   - Mutable references are subtypes of immutable references.
    ///   - Mutable raw pointers are subtypes of immutable raw pointers.
    ///   - References are subtypes of raw pointers, except mutable reference is not a subtype of immutable raw pointer.
    /// - If both types are arrays or slices: the element must be a subtype. If both types are arrays the length must be equal. If `self` is an array `other` can be a slice, but not vice versa.
    pub fn is_structural_subtype_of(&self, other: &TypeStructure) -> IsSubtypeOf {
        match (self, other) {
            (TypeStructure::Opaque, _) | (_, TypeStructure::Opaque) => IsSubtypeOf::Unknown,
            (TypeStructure::OpaqueTuple { elements }, other) => {
                match other.general_tuple_item_types2(elements.len()) {
                    None => IsSubtypeOf::No,
                    Some(other_elements) => {
                        tuple_is_subtype_of2(elements.iter(), other_elements).downgrade_to_unknown()
                    }
                }
            }
            (this, TypeStructure::OpaqueTuple { elements: other_elements }) => {
                match this.general_tuple_item_types2(other_elements.len()) {
                    None => IsSubtypeOf::No,
                    Some(elements) => {
                        tuple_is_subtype_of2(elements, other_elements.iter()).downgrade_to_unknown()
                    }
                }
            }
            (TypeStructure::OpaqueFields { fields }, other) => {
                match other.general_field_compound_field_types() {
                    None => IsSubtypeOf::No,
                    Some(other_fields) => {
                        fields_is_subtype_of(fields, other_fields).downgrade_to_unknown()
                    }
                }
            }
            (this, TypeStructure::OpaqueFields { fields: other_fields }) => {
                match this.general_field_compound_field_types() {
                    None => IsSubtypeOf::No,
                    Some(fields) => {
                        fields_is_subtype_of(fields, other_fields).downgrade_to_unknown()
                    }
                }
            }
            (TypeStructure::Primitive(primitive), TypeStructure::Primitive(other_primitive)) => {
                IsSubtypeOf::known(primitive == other_primitive)
            },
            (TypeStructure::CReprEnum { variants }, TypeStructure::CReprEnum { variants: other_variants }) => {
                other_variants.iter().map(|other_variant| {
                    match variants.iter().find(|variant| other_variant.variant_name == variant.variant_name) {
                        None => IsSubtypeOf::No,
                        Some(variant) => variant.body.is_structural_subtype_of(&other_variant.body)
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

    /// **Unifies** `self`, the "explicitly-provided" type, with `other`, the "inferred-by-value" type:
    /// attempts to *create the lowest common subtype of both `self` and `other`*,
    /// consuming `other` in the process and *guaranteeing that the result type is a subtype of `self` (assuming unknown = bottom)*.
    /// Prioritizes `self` when the types are completely different and there is no common subtype which is not bottom.
    ///
    /// Unification may, on `self`
    /// - Replace if it's opaque. Assign compound type if an opaque compound
    /// - Add fields or *subtract* enum variants
    /// - Add type id to pointer
    /// - Add length to a slice, converting it into an array
    ///
    /// Note: if these types are definitely different, than [TypeStructure::unify] will succeed but
    /// not cause any changes. If you want different behavior (e.g. an error or bottom type), check
    /// [TypeStructure::is_structural_subtype_of].
    pub fn unify(&mut self, other: TypeStructure) {
        // Can't put in match expr because of borrowing rules
        if matches!(self, TypeStructure::Opaque) {
            *self = other;
            return;
        }
        if let TypeStructure::OpaqueTuple { elements } = self {
            match other {
                TypeStructure::OpaqueTuple { elements: other_elements } => {
                    unify_tuple(elements, other_elements)
                }
                TypeStructure::CTuple { elements: other_elements } => {
                    unify_tuple(elements, other_elements);
                    *self = TypeStructure::CTuple { elements: elements.clone() };
                }
                TypeStructure::CReprStruct { body: TypeStructureBody::Tuple(other_elements) } => {
                    unify_tuple(elements, other_elements);
                    *self = TypeStructure::CReprStruct { body: TypeStructureBody::Tuple(elements.clone()) };
                }
                TypeStructure::CReprEnum { variants: other_variants } if other_variants.len() == 1 => match other_variants.into_iter().next().unwrap() {
                    TypeEnumVariant { variant_name, body: TypeStructureBody::Tuple(other_elements) } => {
                        unify_tuple(elements, other_elements);
                        *self = TypeStructure::CReprEnum {
                            variants: vec![TypeEnumVariant {
                                variant_name,
                                body: TypeStructureBody::Tuple(elements.clone())
                            }]
                        };
                    }
                    _ => {}
                },
                TypeStructure::Array { elem: other_elem, length } => {
                    if elements.iter().all(|elem| elem.is_structural_subtype_of(other_elem.as_ref()) != IsSubtypeOf::No) {
                        *self = TypeStructure::Array { elem: other_elem, length };
                    } else {
                        elements.truncate(length);
                        for element in elements {
                            element.unify(other_elem.as_ref().clone());
                        }
                    }
                }
                TypeStructure::Slice { elem: other_elem } => {
                    if elements.iter().all(|elem| elem.is_structural_subtype_of(other_elem.as_ref()) != IsSubtypeOf::No) {
                        *self = TypeStructure::Array { elem: other_elem, length: elements.len() };
                    } else {
                        for element in elements {
                            element.unify(other_elem.as_ref().clone());
                        }
                    }
                }
                _ => {}
            }
            return;
        }
        if let TypeStructure::OpaqueFields { fields } = self {
            match other {
                TypeStructure::OpaqueFields { fields: other_fields } => {
                    unify_fields(fields, other_fields);
                }
                TypeStructure::CReprStruct { body: TypeStructureBody::Fields(other_fields) } => {
                    unify_fields(fields, other_fields);
                    *self = TypeStructure::CReprStruct { body: TypeStructureBody::Fields(fields.clone()) };
                }
                TypeStructure::CReprEnum { variants: other_variants } if other_variants.len() == 1 => match other_variants.into_iter().next().unwrap() {
                    TypeEnumVariant { variant_name, body: TypeStructureBody::Fields(other_fields) } => {
                        unify_fields(fields, other_fields);
                        *self = TypeStructure::CReprEnum {
                            variants: vec![TypeEnumVariant {
                                variant_name,
                                body: TypeStructureBody::Fields(fields.clone())
                            }]
                        }
                    }
                    _ => {}
                }
                _ => {}
            }
            return;
        }
        if let TypeStructure::Slice { elem } = self {
            if let TypeStructure::Array { elem: other_elem, length: other_length } = other {
                elem.unify(*other_elem);
                *self = TypeStructure::Array { elem: elem.clone(), length: other_length };
                return;
            } else if let TypeStructure::OpaqueTuple { elements: other_elements } = other {
                let length = other_elements.len();
                for other_elem in other_elements {
                    elem.unify(other_elem);
                }
                *self = TypeStructure::Array { elem: elem.clone(), length };
                return;
            }
            // else don't return because we handle (self, other) both slices below
        }

        match (self, other) {
            (TypeStructure::Opaque, _) |
            (TypeStructure::OpaqueTuple { .. }, _) |
            (TypeStructure::OpaqueFields { .. }, _) => unreachable!(),
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
            (TypeStructure::CReprEnum { variants }, TypeStructure::OpaqueTuple { elements: other_elements }) if variants.len() == 1 => {
                if let TypeStructureBody::Tuple(elements) = &mut variants[0].body {
                    unify_tuple(elements, other_elements);
                }
            }
            (TypeStructure::CReprEnum { variants }, TypeStructure::OpaqueFields { fields: other_fields }) if variants.len() == 1 => {
                if let TypeStructureBody::Fields(fields) = &mut variants[0].body {
                    unify_fields(fields, other_fields);
                }
            }
            (TypeStructure::CReprStruct { body }, TypeStructure::CReprStruct { body: other_body }) => {
                body.unify(other_body);
            }
            (TypeStructure::CReprStruct { body }, TypeStructure::OpaqueTuple { elements: other_elements }) => {
                if let TypeStructureBody::Tuple(elements) = body {
                    unify_tuple(elements, other_elements);
                }
            }
            (TypeStructure::CReprStruct { body }, TypeStructure::OpaqueFields { fields: other_fields }) => {
                if let TypeStructureBody::Fields(fields) = body {
                    unify_fields(fields, other_fields);
                }
            }
            (TypeStructure::Pointer { ptr_kind, ptr_size: _, refd_id, refd_name: _ }, TypeStructure::Pointer { ptr_kind: other_ptr_kind, ptr_size: _, refd_id: other_refd_id, refd_name: _ }) => {
                ptr_kind.unify(other_ptr_kind);
                if refd_id.is_none() {
                    *refd_id = other_refd_id;
                }
                // Type names don't unify (TODO: may resolve unknown names in the future)
            }
            (TypeStructure::CTuple { elements }, TypeStructure::CTuple { elements: other_elements }) |
            (TypeStructure::CTuple { elements }, TypeStructure::OpaqueTuple { elements: other_elements }) => {
                unify_tuple(elements, other_elements);
            }
            (TypeStructure::Array { elem, length: _ }, TypeStructure::Array { elem: other_elem, length: _ }) |
            (TypeStructure::Array { elem, length: _ }, TypeStructure::Slice { elem: other_elem }) |
            (TypeStructure::Slice { elem }, TypeStructure::Slice { elem: other_elem }) => {
                elem.unify(*other_elem);
            }
            (TypeStructure::Array { elem, length: _ }, TypeStructure::OpaqueTuple { elements: other_elements }) => {
                for other_elem in other_elements {
                    elem.unify(other_elem);
                }
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
                fields_is_subtype_of(fields, other_fields)
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
            (TypeStructureBody::Fields(fields), TypeStructureBody::Fields(other_fields)) => {
                unify_fields(fields, other_fields);
            }
            _ => {}
        }
    }
}

impl RustPointerKind {
    fn is_subtype_of(&self, other: &RustPointerKind) -> bool {
        match (self, other) {
            (RustPointerKind::ImmRaw, RustPointerKind::ImmRaw) => true,
            (RustPointerKind::ImmRaw, RustPointerKind::MutRaw) => true,
            (RustPointerKind::ImmRaw, RustPointerKind::ImmRef) => false,
            (RustPointerKind::ImmRaw, RustPointerKind::MutRef) => false,
            (RustPointerKind::MutRaw, RustPointerKind::ImmRaw) => false,
            (RustPointerKind::MutRaw, RustPointerKind::MutRaw) => true,
            (RustPointerKind::MutRaw, RustPointerKind::ImmRef) => false,
            (RustPointerKind::MutRaw, RustPointerKind::MutRef) => false,
            (RustPointerKind::ImmRef, RustPointerKind::ImmRaw) => true,
            (RustPointerKind::ImmRef, RustPointerKind::MutRaw) => true,
            (RustPointerKind::ImmRef, RustPointerKind::ImmRef) => true,
            (RustPointerKind::ImmRef, RustPointerKind::MutRef) => true,
            (RustPointerKind::MutRef, RustPointerKind::ImmRaw) => false,
            (RustPointerKind::MutRef, RustPointerKind::MutRaw) => true,
            (RustPointerKind::MutRef, RustPointerKind::ImmRef) => false,
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

fn tuple_is_subtype_of(elements: &[RustType], other_elements: &[RustType]) -> IsSubtypeOf {
    tuple_is_subtype_of2(elements.iter(), other_elements.iter())
}

fn tuple_is_subtype_of2<'a>(elements: impl ExactSizeIterator<Item=&'a RustType>, other_elements: impl ExactSizeIterator<Item=&'a RustType>) -> IsSubtypeOf {
    if elements.len() != other_elements.len() {
        IsSubtypeOf::No
    } else {
        zip(elements, other_elements).map(|(element, other_element)| {
            element.is_rough_subtype_of(other_element)
        }).min().unwrap_or(IsSubtypeOf::Yes)
    }
}

fn fields_is_subtype_of(fields: &[TypeStructureBodyField], other_fields: &[TypeStructureBodyField]) -> IsSubtypeOf {
    if fields.len() < other_fields.len() {
        IsSubtypeOf::No
    } else {
        fields.iter().map(|field| {
            match other_fields.iter().find(|other_field| other_field.name == field.name) {
                None => IsSubtypeOf::No,
                Some(other_field) => field.rust_type.is_rough_subtype_of(&other_field.rust_type)
            }
        }).min().unwrap_or(IsSubtypeOf::Yes)
    }
}

fn unify_tuple(elements: &mut Vec<RustType>, other_elements: Vec<RustType>) {
    for (element, other_element) in zip(elements, other_elements) {
        element.unify(other_element);
    }
}

fn unify_fields(fields: &mut Vec<TypeStructureBodyField>, mut other_fields: Vec<TypeStructureBodyField>) {
    for field in fields {
        if let Some(other_field_idx) = other_fields.iter().position(|other_field| field.name == other_field.name) {
            let other_field = other_fields.remove(other_field_idx);
            field.rust_type.unify(other_field.rust_type);
        }
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

    /// If `self` is `Yes`, returns `Unknown`, otherwise returns `self`.
    pub fn downgrade_to_unknown(&self) -> Self {
        match self {
            IsSubtypeOf::Yes => IsSubtypeOf::Unknown,
            IsSubtypeOf::Unknown => IsSubtypeOf::Unknown,
            IsSubtypeOf::No => IsSubtypeOf::No,
        }
    }
}

