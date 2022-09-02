use std::any::TypeId;
use crate::{TypeStructure, RustTypeName, RustPointerKind, PrimitiveType, RustType};

pub trait HasTypeName {
    /// "type id" used for this type, which may not actually be static.
    /// Instances of this type with different lifetime parameters will share the same `StaticId`,
    /// instances of different types or generics should not.
    /// This is the type id that is used in the rest of `structural_reflection`
    type StaticId: ?Sized + 'static;

    fn type_name() -> RustTypeName;

    fn static_type_id() -> TypeId {
        TypeId::of::<Self::StaticId>()
    }
}

/// A type we know the structure of at compile type. We can derive this
pub trait HasStructure: HasTypeName {
    fn structure() -> TypeStructure;
}

impl HasTypeName for () {
    type StaticId = ();

    fn type_name() -> RustTypeName {
        RustTypeName::Tuple {
            elems: vec![]
        }
    }
}

impl HasStructure for () {
    fn structure() -> TypeStructure {
        TypeStructure::CTuple {
            elements: vec![],
        }
    }
}

impl HasTypeName for str {
    type StaticId = str;

    fn type_name() -> RustTypeName {
        RustTypeName::Ident {
            qualifiers: vec![],
            simple_name: "str".to_string(),
            generic_args: vec![]
        }
    }
}

impl<T: HasTypeName> HasTypeName for [T] where T::StaticId: Sized {
    type StaticId = [T::StaticId];

    fn type_name() -> RustTypeName {
        RustTypeName::Slice {
            elem: Box::new(T::type_name())
        }
    }
}

impl<T: HasStructure> HasStructure for [T] where T::StaticId: Sized {
    fn structure() -> TypeStructure {
        TypeStructure::Slice {
            elem: Box::new(RustType::of::<T>())
        }
    }
}

impl<T: HasTypeName, const LEN: usize> HasTypeName for [T; LEN] where T::StaticId: Sized {
    type StaticId = [T::StaticId; LEN];

    fn type_name() -> RustTypeName {
        RustTypeName::Array {
            elem: Box::new(T::type_name()),
            length: LEN
        }
    }
}

impl<T: HasStructure, const LEN: usize> HasStructure for [T; LEN] where T::StaticId: Sized {
    fn structure() -> TypeStructure {
        TypeStructure::Array {
            elem: Box::new(RustType::of::<T>()),
            length: LEN
        }
    }
}

macro impl_has_structure_primitive($prim_tt:tt, $prim_type:ident) {
impl HasTypeName for $prim_tt {
    type StaticId = $prim_tt;

    fn type_name() -> RustTypeName {
        RustTypeName::Ident {
            qualifiers: vec![],
            simple_name: stringify!($prim_tt).to_string(),
            generic_args: vec![]
        }
    }
}

impl HasStructure for $prim_tt {
    fn structure() -> TypeStructure {
        TypeStructure::Primitive(PrimitiveType::$prim_type)
    }
}
}

macro impl_has_structure_pointer(($($ptr_tt:tt)+), ($($static_ptr_tt:tt)+), $ptr_kind:ident) {
impl<T: HasTypeName + ?Sized> HasTypeName for $($ptr_tt)+ T {
    type StaticId = $($static_ptr_tt)+ T::StaticId;

    fn type_name() -> RustTypeName {
        RustTypeName::Pointer {
            ptr_kind: RustPointerKind::$ptr_kind,
            refd: Box::new(T::type_name())
        }
    }
}

impl<T: HasTypeName + ?Sized> HasStructure for $($ptr_tt)+ T {
    fn structure() -> TypeStructure {
        TypeStructure::Pointer {
            ptr_kind: RustPointerKind::$ptr_kind,
            refd_id: Some(T::static_type_id()),
            refd_name: T::type_name(),
        }
    }
}
}

impl_has_structure_primitive!(u8, U8);
impl_has_structure_primitive!(u16, U16);
impl_has_structure_primitive!(u32, U32);
impl_has_structure_primitive!(u64, U64);
impl_has_structure_primitive!(u128, U128);
impl_has_structure_primitive!(usize, Usize);
impl_has_structure_primitive!(i8, I8);
impl_has_structure_primitive!(i16, I16);
impl_has_structure_primitive!(i32, I32);
impl_has_structure_primitive!(i64, I64);
impl_has_structure_primitive!(i128, I128);
impl_has_structure_primitive!(isize, Isize);
impl_has_structure_primitive!(bool, Bool);
impl_has_structure_primitive!(char, Char);
impl_has_structure_primitive!(f32, F32);
impl_has_structure_primitive!(f64, F64);

impl_has_structure_pointer!((&), (&'static), ImmRef);
impl_has_structure_pointer!((&mut), (&'static mut), MutRef);
impl_has_structure_pointer!((*const), (*const), ImmRaw);
impl_has_structure_pointer!((*mut), (*mut), MutRaw);

