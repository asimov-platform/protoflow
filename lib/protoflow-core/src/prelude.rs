// This is free and unencumbered software released into the public domain.

#[cfg(feature = "std")]
extern crate std;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
use std as alloc;

#[allow(unused)]
pub use alloc::{
    borrow::Cow,
    boxed::Box,
    collections::btree_set::Iter as BTreeSetIter,
    collections::{BTreeMap, BTreeSet, VecDeque},
    format,
    rc::Rc,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};

#[allow(unused)]
pub use core::{
    any::type_name,
    cell::RefCell,
    convert::{AsRef, TryFrom},
    fmt,
    marker::PhantomData,
    ops::{Deref, Index, Range},
    option::Option,
    result::Result,
    slice,
    str::FromStr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};

pub use bytes::{Bytes, BytesMut};

pub type Instant = Duration;

#[doc(hidden)]
pub use bytes;

#[doc(hidden)]
pub use parking_lot::RwLock;

#[doc(hidden)]
pub use prost;

#[doc(hidden)]
pub use prost_types;

#[cfg(feature = "sysml")]
#[doc(hidden)]
pub use sysml_model;

pub use dogma::traits::{Labeled, MaybeLabeled, MaybeNamed, Named};
