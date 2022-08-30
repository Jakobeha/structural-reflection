use std::mem::{align_of, size_of};
use crate::{RustType, TypeEnumVariant, TypeStructureBody};
use crate::structure::TypeStructure;

impl TypeStructure {
    pub fn infer_size(&self) -> Option<usize> {
        match self {
            TypeStructure::Opaque => None,
            TypeStructure::Primitive(primitive) => Some(primitive.size()),
            TypeStructure::CReprEnum { variants } => {
                let discriminant_size = discriminant_size(variants.len());
                let data_size = variants.iter().map(|variant| variant.infer_size()).max().unwrap_or(0);
                Some(discriminant_size + data_size)
            }
            TypeStructure::CReprStruct { body } => Some(body.infer_size()),
            TypeStructure::Pointer { .. } => Some(size_of::<*const ()>()),
            TypeStructure::CTuple { elements } => Some(infer_c_tuple_size(elements)),
            TypeStructure::Array { elem, length } => Some(infer_array_size(elem, *length)),
            TypeStructure::Slice { .. } => None
        }
    }

    pub fn infer_align(&self) -> Option<usize> {
        match self {
            TypeStructure::Opaque => None,
            TypeStructure::Primitive(primitive) => Some(primitive.align()),
            TypeStructure::CReprEnum { variants } => {
                let discriminant_align = discriminant_align(variants.len());
                let data_align = variants.iter().map(|variant| variant.infer_align()).max().unwrap_or(0);
                Some(usize::max(discriminant_align, data_align))
            }
            TypeStructure::CReprStruct { body } => Some(body.infer_align()),
            TypeStructure::Pointer { .. } => Some(align_of::<*const ()>()),
            TypeStructure::CTuple { elements } => Some(infer_c_tuple_align(elements)),
            TypeStructure::Array { elem, length: _ } => Some(infer_array_align(elem)),
            TypeStructure::Slice { elem } => Some(infer_array_align(elem))
        }
    }
}

impl TypeEnumVariant {
    fn infer_size(&self) -> usize {
        self.body.infer_size()
    }

    fn infer_align(&self) -> usize {
        self.body.infer_align()
    }
}

impl TypeStructureBody {
    fn infer_size(&self) -> usize {
        match self {
            TypeStructureBody::None => 0,
            TypeStructureBody::Tuple(elems) => infer_c_tuple_size(elems),
            TypeStructureBody::Fields(fields) => infer_c_tuple_size(fields.iter().map(|field| &field.rust_type))
        }
    }

    fn infer_align(&self) -> usize {
        match self {
            TypeStructureBody::None => 0,
            TypeStructureBody::Tuple(elems) => infer_c_tuple_align(elems),
            TypeStructureBody::Fields(fields) => infer_c_tuple_align(fields.iter().map(|field| &field.rust_type))
        }
    }
}

// Note: technically tuples don't have a defined repr according to Rust

pub fn infer_c_tuple_size<'a>(elems: impl IntoIterator<Item=&'a RustType>) -> usize {
    let mut cumulative_size = 0;
    for elem in elems {
        let size = elem.size;
        let align = elem.align;
        if cumulative_size % align != 0 {
            cumulative_size += align - (cumulative_size % align);
        }
        cumulative_size += size;
    }
    cumulative_size
}

pub fn infer_c_tuple_align<'a>(elems: impl IntoIterator<Item=&'a RustType>) -> usize {
    let mut max_align = 0;
    for elem in elems {
        let align = elem.align;
        if max_align < align {
            max_align = align;
        }
    }
    max_align
}

pub fn infer_array_size(elem: &RustType, length: usize) -> usize {
    let mut aligned_size = elem.size;
    let align = elem.align;
    if aligned_size % align != 0 {
        aligned_size += align - aligned_size % align;
    }
    aligned_size * length
}

pub fn infer_array_align(elem: &RustType) -> usize {
    elem.align
}

fn discriminant_size(_num_discriminants: usize) -> usize {
    // "but it selects the same size as the C compiler would use for the given target for an equivalent C-enum declaration"
    // I have no idea if this is correct. C is defined to represent enums as ints. I know this is wrong on systems where int != 4 bytes,
    // but don't know how to detect that.
    4
}

fn discriminant_align(_num_discriminants: usize) -> usize {
    // same as above
    4
}
