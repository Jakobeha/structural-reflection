use std::any::TypeId;
use std::borrow::Cow;
use std::sync::RwLock;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use bimap::BiHashMap;
use lazy_static::lazy_static;
use log::error;
use crate::{IntrinsicRustType, RustType};
use crate::intrinsic::UnknownIntrinsicType;

use crate::structure::TypeStructure;
use crate::type_name::RustTypeName;

lazy_static! {
    static ref KNOWN_TYPES: RwLock<HashMap<RustTypeName, RustType>> = RwLock::new({
        let mut known_types = HashMap::new();
        RustType::register_builtins(&mut known_types);
        known_types
    });
    static ref KNOWN_NAMES: RwLock<BiHashMap<TypeId, RustTypeName>> = RwLock::new(BiHashMap::new());
    static ref KNOWN_INTRINSICS: RwLock<HashMap<TypeId, IntrinsicRustType>> = RwLock::new(HashMap::new());
}

impl RustType {
    /// Registers the rust type so it can be looked up by name,
    /// registers the type name so it can be looked up by id (if the rust type has an id),
    /// and registers the intrinsic type so it can be looked up by id (if `Some`).
    ///
    /// The provided type is `Cow` so that it doesn't need to be cloned if already registered.
    /// If another type is registered with the same name and the type ids aren't equal, this will log an error.
    pub fn register(rust_type: Cow<'_, RustType>, intrinsic_rust_type: Option<IntrinsicRustType>) {
        if let Some(intrinsic_type) = intrinsic_rust_type {
            IntrinsicRustType::register(intrinsic_type);
        }
        if let Some(type_id) = rust_type.type_id {
            RustTypeName::register(type_id, Cow::Borrowed(&rust_type.type_name))
        }
        Self::register_just_type(rust_type);
    }

    /// Registers the rust type so it can be looked up by name.
    fn register_just_type(rust_type: Cow<'_, RustType>) {
        let type_name = &rust_type.type_name;
        if let Some(mut known_types) = catch_and_log!(KNOWN_TYPES.write(), "known rust types poisoned") {
            if let Some(existing_type) = known_types.get(type_name) {
                if existing_type != &*rust_type || &existing_type.type_id != &rust_type.type_id {
                    error!("rust type with name {} already registered with a different structure", type_name.qualified());
                }
            }
            known_types.insert(type_name.clone(), rust_type.into_owned());
        }
    }

    /// Index into the type registry, which is a global singleton.
    /// Returns the registered type with the given type name.
    pub fn lookup(type_name: &RustTypeName) -> Option<RustType> {
        match catch_and_log!(KNOWN_TYPES.read(), "known rust types poisoned") {
            None => None,
            Some(known_types) => known_types.get(type_name).cloned()
        }
    }

    /// Index into the type registry, which is a global singleton.
    /// Returns the registered type with the given type id.
    pub fn lookup_from_id(type_id: TypeId) -> Option<RustType> {
        match (catch_and_log!(KNOWN_NAMES.read(), "known rust type names poisoned"), catch_and_log!(KNOWN_TYPES.read(), "known rust types poisoned")) {
            (Some(known_names), Some(known_types)) => {
                known_names.get_by_left(&type_id)
                    .and_then(|type_name| known_types.get(&*type_name))
                    .cloned()
            },
            _ => None
        }
    }
}

impl RustTypeName {
    /// Registers the type id to the type name.
    ///
    /// The provided type name is `Cow` so that it doesn't need to be cloned if already registered.
    /// If another type id is registered with the same name or vice versa and they aren't equal, this will log an error.
    pub fn register(type_id: TypeId, type_name: Cow<'_, RustTypeName>) {
        if let Some(mut known_names) = catch_and_log!(KNOWN_NAMES.write(), "known rust type names poisoned") {
            if let Some(existing_name) = known_names.get_by_left(&type_id) {
                if existing_name != &*type_name {
                    error!("rust type with id {:?} already registered with a different name: old={} new={}", type_id, existing_name.qualified(), type_name.qualified());
                }
            }
            known_names.insert(type_id, type_name.into_owned());
        }
    }

    /// Index into the type registry, which is a global singleton.
    /// Returns the registered type name with the given type id.
    pub fn lookup(type_id: TypeId) -> Option<RustTypeName> {
        match catch_and_log!(KNOWN_NAMES.read(), "known rust type names poisoned") {
            None => None,
            Some(known_names) => known_names.get_by_left(&type_id).cloned()
        }
    }

    /// Index into the type registry, which is a global singleton.
    /// Returns the registered type id with the given type name.
    pub fn lookup_back(&self) -> Option<TypeId> {
        match catch_and_log!(KNOWN_NAMES.read(), "known rust type names poisoned") {
            None => None,
            Some(known_names) => known_names.get_by_right(self).copied()
        }
    }

    /// Index into the type registry, which is a global singleton.
    /// Returns the registered intrinsic type with the given type name.
    pub fn lookup_back_intrinsic(&self) -> Option<IntrinsicRustType> {
        self.lookup_back().and_then(IntrinsicRustType::lookup)
    }
}

impl IntrinsicRustType {
    /// Registers the intrinsic type so it can be looked up by id.
    pub fn register(intrinsic_type: IntrinsicRustType) {
        if let Some(mut known_intrinsics) = catch_and_log!(KNOWN_INTRINSICS.write(), "known intrinsic types poisoned") {
            if let Some(existing_intrinsic) = known_intrinsics.get(&intrinsic_type.type_id) {
                if existing_intrinsic != &intrinsic_type {
                    error!("intrinsic type with id {:?} already registered with a different name: old={} new={}", intrinsic_type.type_id, existing_intrinsic.type_name, intrinsic_type.type_name);
                }
            }
            known_intrinsics.insert(intrinsic_type.type_id, intrinsic_type);
        }
    }

    pub fn lookup(type_id: TypeId) -> Option<IntrinsicRustType> {
        match catch_and_log!(KNOWN_INTRINSICS.read(), "known intrinsic types poisoned") {
            None => None,
            Some(known_intrinsics) => known_intrinsics.get(&type_id).cloned()
        }
    }
}

// region builtins
impl RustType {
    fn register_builtins(builtins: &mut HashMap<RustTypeName, RustType>) {
        RustType::register_builtin::<String>(builtins, "String");
        RustType::register_builtin::<Box<UnknownIntrinsicType>>(builtins, "Box<{unknown}>");
        RustType::register_builtin::<Vec<UnknownIntrinsicType>>(builtins, "Vec<{unknown}>");
        RustType::register_builtin::<VecDeque<UnknownIntrinsicType>>(builtins, "VecDeque<{unknown}>");
        RustType::register_builtin::<BTreeSet<UnknownIntrinsicType>>(builtins, "BTreeSet<{unknown}>");
        RustType::register_builtin::<HashSet<UnknownIntrinsicType>>(builtins, "HashSet<{unknown}>");
        RustType::register_builtin::<BTreeMap<UnknownIntrinsicType, UnknownIntrinsicType>>(builtins, "BTreeSet<{unknown}, {unknown}>");
        RustType::register_builtin::<HashMap<UnknownIntrinsicType, UnknownIntrinsicType>>(builtins, "HashSet<{unknown}, {unknown}>");
    }

    fn register_builtin<T: 'static>(builtins: &mut HashMap<RustTypeName, RustType>, name: &str) {
        let type_name = RustTypeName::try_from(name).expect("bad builtin name");
        let intrinsic = IntrinsicRustType::of_static::<T>();
        let rust_type = RustType {
            type_id: Some(intrinsic.type_id),
            type_name: type_name.clone(),
            size: intrinsic.size,
            align: intrinsic.align,
            structure: TypeStructure::Opaque
        };
        let old = builtins.insert(type_name.clone(), rust_type);
        if old.is_some() {
            panic!("builtin type {} already registered", type_name.qualified());
        }

        // We can access and load the other lazy members here
        RustTypeName::register(intrinsic.type_id, Cow::Owned(type_name));
        IntrinsicRustType::register(intrinsic);
    }
}
// endregion

// region misc
/// Instead of propagating this error, just log it as an error.
/// Useful for non-essential failures like the file watcher
macro catch_and_log($e:expr, $msg:literal $(, $args:expr)*) {
match $e {
    Ok(v) => Some(v),
    Err(e) => {
        log::error!(concat!($msg, ": {}") $(, $args)*, e);
        None
    }
}
}
// endregion