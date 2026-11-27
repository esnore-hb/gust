//! Algoritmos de *hash* concretos que implementan [`HashStrategy`].
//!
//! Cada algoritmo se apoya en un *crate* externo; el resto del proyecto solo ve el trait.
//!
//! [`HashStrategy`]: crate::hash::HashStrategy

mod sha;

pub use sha::Sha1;
