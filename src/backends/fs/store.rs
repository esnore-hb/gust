use crate::store::{ObjectStore, StoreError};

use super::Layout;

/// # FsStore
/// Guarda cada objeto como un archivo en `.gust/objects/`, nombrado con su id en hexadecimal.
///
/// Los objetos nunca cambian, así que cada archivo se escribe una sola vez y después solo se lee.
///
/// ## Limitaciones conocidas (post-MVP)
/// - **Sin compresión:** los bytes se guardan tal cual. Comprimir con `flate2` dentro de
///   `put_raw`/`get_raw` no afectaría a nadie más, porque el id se calcula antes, sobre los bytes
///   sin comprimir.
#[derive(Debug, Clone)]
pub struct FsStore {
    layout: Layout,
}

impl FsStore {
    /// Abre el almacén de objetos de un repositorio ya inicializado con [`Layout::init`].
    pub fn new(layout: Layout) -> Self {
        todo!()
    }
}

impl ObjectStore for FsStore {
    /// Escribe `raw` en [`Layout::object_path`] con [`super::write_atomic`].
    ///
    /// Si dos procesos guardan el mismo objeto a la vez no hay conflicto: ambos escriben los
    /// mismos bytes y el último `rename` gana sin cambiar nada.
    fn put_raw(&mut self, id: &[u8], raw: &[u8]) -> Result<(), StoreError> {
        todo!()
    }

    /// Lee el archivo del objeto completo; traduce `io::ErrorKind::NotFound` a
    /// [`StoreError::NotFound`].
    fn get_raw(&self, id: &[u8]) -> Result<Vec<u8>, StoreError> {
        todo!()
    }

    /// Consulta si existe el archivo del objeto (`Path::try_exists`), sin abrirlo.
    fn contains(&self, id: &[u8]) -> Result<bool, StoreError> {
        todo!()
    }
}
