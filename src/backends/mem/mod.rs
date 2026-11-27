//! *Backends* que viven solo en RAM: implementaciones de referencia y para pruebas.
//!
//! No persisten nada: todo se pierde al soltar el valor. A cambio, son tan simples (un
//! `HashMap` o un `BTreeMap` y poco más) que sirven para dos cosas:
//! - **Pruebas rápidas** de la lógica del repositorio, sin crear carpetas temporales.
//! - **Oráculo** para los *backends* de [`super::fs`]: la misma secuencia de operaciones debe
//!   dar el mismo resultado en ambos.
//!
//! No usan *locks*: el préstamo `&mut self` ya impide escrituras simultáneas dentro de un
//! proceso, y nada más puede verlos.

mod refs;
mod store;
mod workdir;

pub use refs::MemoryRefs;
pub use store::MemoryStore;
pub use workdir::MemoryWorkDir;
