use std::io::Read;
use structural_reflection::c_tuple::CTuple2;
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
pub enum AnEnum<'a, 'b: 'a, Baz: 'b> where &'b Baz: Read {
    Unit,
    Tuple(CopyRange<i32>, FooBar<'a, &'b Baz, &'b PathBuf>),
    Fields {
        red: u64,
        green: u64,
        blue: u64
    }
}

#[test]
fn derive_has_type_name() {
    assert_eq!(Unit::type_name().to_string(), "Unit");
    assert_eq!(ViewId::type_name().to_string(), "ViewId");
    assert_eq!(CopyRange::<usize>::type_name().to_string(), "CopyRange<usize>");
    assert_eq!(FooBar::<'_, &PathBuf, &str>::type_name().to_string(), "FooBar<&PathBuf, &str>");
}