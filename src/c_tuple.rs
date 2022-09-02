use crate::{HasStructure, HasTypeName, RustType, RustTypeName, TypeStructure};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct CTuple1<A>(pub A);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct CTuple2<A, B>(pub A, pub B);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct CTuple3<A, B, C>(pub A, pub B, pub C);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct CTuple4<A, B, C, D>(pub A, pub B, pub C, pub D);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct CTuple5<A, B, C, D, E>(pub A, pub B, pub C, pub D, pub E);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct CTuple6<A, B, C, D, E, F>(pub A, pub B, pub C, pub D, pub E, pub F);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct CTuple7<A, B, C, D, E, F, G>(pub A, pub B, pub C, pub D, pub E, pub F, pub G);

pub macro CTuple {
    () => { () },
    ($A:ty) => { $crate::c_tuple::CTuple1<$A> },
    ($A:ty, $B:ty) => { $crate::c_tuple::CTuple2<$A, $B> },
    ($A:ty, $B:ty, $C:ty) => { $crate::c_tuple::CTuple3<$A, $B, $C> },
    ($A:ty, $B:ty, $C:ty, $D:ty) => { $crate::c_tuple::CTuple4<$A, $B, $C, $D> },
    ($A:ty, $B:ty, $C:ty, $D:ty, $E:ty) => { $crate::c_tuple::CTuple5<$A, $B, $C, $D, $E> },
    ($A:ty, $B:ty, $C:ty, $D:ty, $E:ty, $F:ty) => { $crate::c_tuple::CTuple6<$A, $B, $C, $D, $E, $F> },
    ($A:ty, $B:ty, $C:ty, $D:ty, $E:ty, $F:ty, $G:ty) => { $crate::c_tuple::CTuple7<$A, $B, $C, $D, $E, $F, $G> },
}

pub macro c_tuple {
    () => { () },
    ($a:expr) => { $crate::c_tuple::CTuple1($a) },
    ($a:expr, $b:expr) => { $crate::c_tuple::CTuple2($a, $b) },
    ($a:expr, $b:expr, $c:expr) => { $crate::c_tuple::CTuple3($a, $b, $c) },
    ($a:expr, $b:expr, $c:expr, $d:expr) => { $crate::c_tuple::CTuple4($a, $b, $c, $d) },
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => { $crate::c_tuple::CTuple5($a, $b, $c, $d, $e) },
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr) => { $crate::c_tuple::CTuple6($a, $b, $c, $d, $e, $f) },
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr) => { $crate::c_tuple::CTuple7($a, $b, $c, $d, $e, $f, $g) },
}

macro impl_c_tuple($name:ident, $($t:ident),+) {
    impl<$($t: HasTypeName),+> HasTypeName for $name<$($t),+> where $($t::StaticId: Sized),+ {
        type StaticId = $name<$($t::StaticId),+>;

        fn type_name() -> RustTypeName {
            RustTypeName::Tuple {
                elems: vec![$(<$t as HasTypeName>::type_name()),+]
            }
        }
    }
    impl<$($t: HasStructure),+> HasStructure for $name<$($t),+> where $($t::StaticId: Sized),+ {
        fn structure() -> TypeStructure {
            TypeStructure::CTuple {
                elements: vec![$(RustType::of::<$t>()),+]
            }
        }
    }
}

impl_c_tuple!(CTuple1, A);
impl_c_tuple!(CTuple2, A, B);
impl_c_tuple!(CTuple3, A, B, C);
impl_c_tuple!(CTuple4, A, B, C, D);
impl_c_tuple!(CTuple5, A, B, C, D, E);
impl_c_tuple!(CTuple6, A, B, C, D, E, F);
impl_c_tuple!(CTuple7, A, B, C, D, E, F, G);