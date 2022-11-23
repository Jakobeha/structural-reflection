use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use structural_reflection::c_tuple::CTuple2;
use structural_reflection::{HasTypeName, HasStructure};
use structural_reflection_derive::{HasTypeName, HasStructure};
use std::marker::PhantomData;
use std::path::PathBuf;

#[derive(HasTypeName, HasStructure)]
#[repr(C)]
pub struct Unit;

#[derive(HasTypeName, HasStructure)]
#[repr(transparent)]
pub struct ViewId(usize);

#[derive(HasTypeName, HasStructure)]
#[repr(C)]
pub struct CopyRange<Idx> {
    pub start: Idx,
    pub end: Idx
}

#[derive(HasTypeName, HasStructure)]
#[repr(C)]
pub struct FooBar<'a, Baz: 'a + Read, Qux> {
    pub range: CopyRange<usize>,
    pub abc: CTuple2<Baz, Qux>,
    _p: PhantomData<&'a ()>
}

#[derive(HasTypeName, HasStructure)]
#[repr(C)]
pub enum AnEnum<'a, 'b: 'a, Baz: 'b> where &'b mut Baz: Read {
    Unit,
    Tuple(CopyRange<i32>, FooBar<'a, &'b mut Baz, &'b PathBuf>),
    Fields {
        red: u64,
        green: u64,
        blue: u64
    }
}

#[test]
fn derive_has_type_name() {
    assert_eq!(Unit::type_name().unqualified().to_string(), "Unit");
    assert_eq!(ViewId::type_name().unqualified().to_string(), "ViewId");
    assert_eq!(CopyRange::<usize>::type_name().unqualified().to_string(), "CopyRange<usize>");
    assert_eq!(FooBar::<'_, File, &str>::type_name().unqualified().to_string(), "FooBar<File, &str>");
    assert_eq!(FooBar::<'_, File, &str>::type_name().qualified().to_string(), "FooBar<std::fs::File, &str>");
    assert_eq!(AnEnum::<'_, 'static, Box<File>>::type_name().unqualified().to_string(), "AnEnum<Box<File>>");
    assert_eq!(AnEnum::<'_, 'static, Box<File>>::type_name().qualified().to_string(), "AnEnum<std::boxed::Box<std::fs::File>>");
}

#[test]
fn derive_has_structure_regression() {
    assert_eq!(dbg(usize::structure()), "Primitive(Usize)");
    assert_eq!(dbg(Box::<String>::structure()), "Opaque");
    assert_eq!(dbg(Unit::structure()), "CReprStruct { body: None }");
    assert_eq!(dbg(ViewId::structure()), "CReprStruct { body: Tuple([RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }, size: 8, align: 8, structure: Primitive(Usize) }]) }");
    assert_eq!(dbg(CopyRange::<usize>::structure()), "CReprStruct { body: Fields([TypeStructureBodyField { name: \"start\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }, size: 8, align: 8, structure: Primitive(Usize) } }, TypeStructureBodyField { name: \"end\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }, size: 8, align: 8, structure: Primitive(Usize) } }]) }");
    assert_eq!(dbg(FooBar::<'_, File, &str>::structure()), "CReprStruct { body: Fields([TypeStructureBodyField { name: \"range\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"CopyRange\", generic_args: [Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }] }, size: 16, align: 8, structure: CReprStruct { body: Fields([TypeStructureBodyField { name: \"start\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }, size: 8, align: 8, structure: Primitive(Usize) } }, TypeStructureBodyField { name: \"end\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }, size: 8, align: 8, structure: Primitive(Usize) } }]) } } }, TypeStructureBodyField { name: \"abc\", rust_type: RustType { type_id: None, type_name: Tuple { elems: [Ident { qualifier: [\"std\", \"fs\"], simple_name: \"File\", generic_args: [] }, Pointer { refd: Ident { qualifier: Qualifier([]), simple_name: \"str\", generic_args: [] }, ptr_kind: ImmRef }] }, size: 24, align: 8, structure: CTuple { elements: [RustType { type_id: None, type_name: Ident { qualifier: [\"std\", \"fs\"], simple_name: \"File\", generic_args: [] }, size: 4, align: 4, structure: Opaque }, RustType { type_id: None, type_name: Pointer { refd: Ident { qualifier: Qualifier([]), simple_name: \"str\", generic_args: [] }, ptr_kind: ImmRef }, size: 16, align: 8, structure: Pointer { ptr_kind: ImmRef, refd_id: Some(TypeId { t: 17258340640123294832 }), refd_name: Ident { qualifier: Qualifier([]), simple_name: \"str\", generic_args: [] } } }] } } }, TypeStructureBodyField { name: \"_p\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: [\"std\", \"marker\"], simple_name: \"PhantomData\", generic_args: [Pointer { refd: Tuple { elems: [] }, ptr_kind: ImmRef }] }, size: 0, align: 1, structure: CReprStruct { body: None } } }]) }");
    assert_eq!(dbg(AnEnum::<'_, 'static, Box<File>>::structure()), "CReprEnum { variants: [TypeEnumVariant { variant_name: \"Unit\", body: None }, TypeEnumVariant { variant_name: \"Tuple\", body: Tuple([RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"CopyRange\", generic_args: [Ident { qualifier: Qualifier([]), simple_name: \"i32\", generic_args: [] }] }, size: 8, align: 4, structure: CReprStruct { body: Fields([TypeStructureBodyField { name: \"start\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"i32\", generic_args: [] }, size: 4, align: 4, structure: Primitive(I32) } }, TypeStructureBodyField { name: \"end\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"i32\", generic_args: [] }, size: 4, align: 4, structure: Primitive(I32) } }]) } }, RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"FooBar\", generic_args: [Pointer { refd: Ident { qualifier: [\"std\", \"boxed\"], simple_name: \"Box\", generic_args: [Ident { qualifier: [\"std\", \"fs\"], simple_name: \"File\", generic_args: [] }] }, ptr_kind: MutRef }, Pointer { refd: Ident { qualifier: [\"std\", \"path\"], simple_name: \"PathBuf\", generic_args: [] }, ptr_kind: ImmRef }] }, size: 32, align: 8, structure: CReprStruct { body: Fields([TypeStructureBodyField { name: \"range\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"CopyRange\", generic_args: [Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }] }, size: 16, align: 8, structure: CReprStruct { body: Fields([TypeStructureBodyField { name: \"start\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }, size: 8, align: 8, structure: Primitive(Usize) } }, TypeStructureBodyField { name: \"end\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"usize\", generic_args: [] }, size: 8, align: 8, structure: Primitive(Usize) } }]) } } }, TypeStructureBodyField { name: \"abc\", rust_type: RustType { type_id: None, type_name: Tuple { elems: [Pointer { refd: Ident { qualifier: [\"std\", \"boxed\"], simple_name: \"Box\", generic_args: [Ident { qualifier: [\"std\", \"fs\"], simple_name: \"File\", generic_args: [] }] }, ptr_kind: MutRef }, Pointer { refd: Ident { qualifier: [\"std\", \"path\"], simple_name: \"PathBuf\", generic_args: [] }, ptr_kind: ImmRef }] }, size: 16, align: 8, structure: CTuple { elements: [RustType { type_id: None, type_name: Pointer { refd: Ident { qualifier: [\"std\", \"boxed\"], simple_name: \"Box\", generic_args: [Ident { qualifier: [\"std\", \"fs\"], simple_name: \"File\", generic_args: [] }] }, ptr_kind: MutRef }, size: 8, align: 8, structure: Pointer { ptr_kind: MutRef, refd_id: Some(TypeId { t: 10546209595991191354 }), refd_name: Ident { qualifier: [\"std\", \"boxed\"], simple_name: \"Box\", generic_args: [Ident { qualifier: [\"std\", \"fs\"], simple_name: \"File\", generic_args: [] }] } } }, RustType { type_id: None, type_name: Pointer { refd: Ident { qualifier: [\"std\", \"path\"], simple_name: \"PathBuf\", generic_args: [] }, ptr_kind: ImmRef }, size: 8, align: 8, structure: Pointer { ptr_kind: ImmRef, refd_id: Some(TypeId { t: 3554803581706964995 }), refd_name: Ident { qualifier: [\"std\", \"path\"], simple_name: \"PathBuf\", generic_args: [] } } }] } } }, TypeStructureBodyField { name: \"_p\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: [\"std\", \"marker\"], simple_name: \"PhantomData\", generic_args: [Pointer { refd: Tuple { elems: [] }, ptr_kind: ImmRef }] }, size: 0, align: 1, structure: CReprStruct { body: None } } }]) } }]) }, TypeEnumVariant { variant_name: \"Fields\", body: Fields([TypeStructureBodyField { name: \"red\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"u64\", generic_args: [] }, size: 8, align: 8, structure: Primitive(U64) } }, TypeStructureBodyField { name: \"green\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"u64\", generic_args: [] }, size: 8, align: 8, structure: Primitive(U64) } }, TypeStructureBodyField { name: \"blue\", rust_type: RustType { type_id: None, type_name: Ident { qualifier: Qualifier([]), simple_name: \"u64\", generic_args: [] }, size: 8, align: 8, structure: Primitive(U64) } }]) }] }");
}

fn dbg(t: impl Debug) -> String {
    format!("{:?}", t)
}