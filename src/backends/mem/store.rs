use std::collections::HashMap;

use crate::store::{ObjectStore, StoreError};

/// # MemoryStore
/// Guarda los objetos en un `HashMap` indexado por su id.
///
/// Nunca produce [`StoreError::Io`]; los demás errores (ej. `NotFound`) se comportan igual que
/// en [`FsStore`](crate::backends::fs::FsStore).
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    /// id crudo → bytes codificados (`<KIND> <len>\0<body>`)
    objects: HashMap<Vec<u8>, Vec<u8>>,
}

impl ObjectStore for MemoryStore {
    /// Inserta `raw` bajo `id`, reemplazando lo que hubiera (con el mismo id, son los mismos bytes).
    fn put_raw(&mut self, id: &[u8], raw: &[u8]) -> Result<(), StoreError> {
        todo!()
    }

    /// Devuelve una copia de los bytes, o [`StoreError::NotFound`] si `id` no está.
    fn get_raw(&self, id: &[u8]) -> Result<Vec<u8>, StoreError> {
        todo!()
    }

    fn contains(&self, id: &[u8]) -> Result<bool, StoreError> {
        todo!()
    }
}
