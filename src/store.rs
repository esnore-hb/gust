//! Abstrae el almacenamiento de los *objetos inmutables* del repositorio (*blobs*, *trees* y
//! *commits*), guardándolos e indexándolos según su propio contenido.
//!
//! A diferencia de [`crate::refs::RefStore`], que administra punteros que cambian con cada
//! *commit* (ramas, `HEAD`), `ObjectStore` solo acumula objetos: nunca modifica ni sobrescribe
//! uno ya guardado.
//!
//! ## Repaso
//! ### Almacenamiento direccionado por contenido
//! La llave de cada objeto es el *hash* de sus bytes (su id, ver [`crate::hash`]). Así:
//! - El mismo contenido se guarda una sola vez.
//! - Al leer, se re-hashean los bytes: si no coinciden con la llave, están corruptos.
//! - "Modificar" un objeto es, en realidad, guardar uno nuevo.
//!
//! ## Diseño
//! Igual que [`HashStrategy`], el trait define **qué** se puede hacer y cada *backend* decide
//! **dónde** (ej. `MemoryStore` en RAM, `FsStore` en disco, quizás con compresión).
//!
//! - **Crudas** (las implementa el *backend*): `put_raw`, `get_raw` y `contains`. Usan `&[u8]`
//!   como llave, así el *backend* no depende del algoritmo de *hash*.
//! - **Tipadas** (gratis): `write` guarda un [`GitObject`] si no existe y devuelve su id;
//!   `read` lo recupera verificando su integridad.
//!
//! ## Limitaciones conocidas (post-MVP)
//! - **Operaciones uno a uno (`put_raw` / `get_raw`):** Sin lecturas ni escrituras por lotes
//!   (*batching*), recorrer o guardar un DAG grande en disco o red sufre por exceso de *syscalls*
//!   e inodos (*loose objects*) o por latencia de *round-trips*. A futuro conviene agrupar
//!   objetos en segmentos *append-only*.
//! - **Carga completa en RAM y opacidad del `KIND`:** `get_raw` lee todo el objeto en memoria sin
//!   opción de consultar solo su *header* (tipo y largo), y `put_raw` no recibe el `KIND` por
//!   separado (útil para separar segmentos de grafo `commit`/`tree` de segmentos de `blob`).
//! - **Granularidad de errores en `ObjectStore::read`:** `GitObject::decode` unifica fallos de
//!   *header* y de tipo en `DeserializeError`, por lo que `StoreError::InvalidHeader` y
//!   `StoreError::KindMismatch` aún no se emiten por separado.

use crate::hash::{HashStrategy, Deserializable, GitObject, DeserializeError};
use std::{error::Error, fmt, io};

/// # StoreError
/// Error producido al leer o escribir objetos en un [`ObjectStore`].
#[derive(Debug)] // requerido por std::error::Error
pub enum StoreError {
    /// Error de entrada/salida del backend de almacenamiento
    Io(io::Error),
    /// El objeto solicitado no existe en el almacenamiento
    NotFound,
    /// El *header* `<KIND> <len>\0` es inválido o el largo no coincide
    InvalidHeader(&'static str),
    /// El tipo almacenado no coincide con el tipo solicitado
    KindMismatch { expected: &'static str, found: String },
    /// El hash de los bytes leídos no coincide con el *digest* esperado
    IntegrityMismatch,
    /// Error al deserializar el cuerpo del objeto
    Deserialize(DeserializeError),
}


impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "error de I/O en el store de objetos: {err}"),
            Self::NotFound => write!(f, "objeto no encontrado"),
            Self::InvalidHeader(msg) => write!(f, "header de objeto inválido: {msg}"),
            Self::KindMismatch { expected, found } => {
                write!(f, "tipo de objeto inesperado: se esperaba {expected}, se encontró {found}")
            }
            Self::IntegrityMismatch => write!(f, "el hash del objeto no coincide con su id"),
            Self::Deserialize(err) => write!(f, "objeto corrupto: {err}"),
        }
    }
}

impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Deserialize(err) => Some(err),
            Self::NotFound
            | Self::InvalidHeader(_)
            | Self::KindMismatch { .. }
            | Self::IntegrityMismatch => None,
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}


/// # ObjectStore
/// Define el almacenamiento de objetos indexados por contenido ([`GitObject`]).
///
/// Desacopla la lógica del repositorio del medio concreto donde se guardan los bytes
/// (por ejemplo, disco o memoria). Cada implementación solo resuelve el guardado y
/// lectura de bytes crudos, y obtiene gratis los métodos tipados `write` y `read`.
///
/// ## Ejemplo
/// ```ignore
/// // Guarda cualquier GitObject y devuelve su digest
/// let blob = Blob::<HashKetchum>::new(b"hola".to_vec());
/// let id = store.write(&blob)?;
///
/// // Recupera el objeto verificando integridad, header y deserialización
/// let recovered = store.read::<Blob<HashKetchum>>(&id)?;
/// ```
pub trait ObjectStore {
    /// Guarda los bytes asociados el identificador `id`.
    /// 
    /// No valida el *header* ni el contenido, ya que [`ObjectStore::write`] se encarga de eso.
    fn put_raw(&mut self, id: &[u8], raw: &[u8]) -> Result<(), StoreError>;
    /// Recupera los bytes crudos del objeto `id` almacenado.
    /// 
    /// Debe devolver [`StoreError::NotFound`] si no existe ningún objeto con ese identificador
    fn get_raw(&self, id: &[u8]) -> Result<Vec<u8>, StoreError>;
    /// Indica si ya existe un objeto almacenado para `id`, sin leer su contenido.
    fn contains(&self, id: &[u8]) -> Result<bool, StoreError>;

    /// Calcula el `id` del [`GitObject`] `obj` y guarda su serialización si no existe.
    /// 
    /// Si el objeto ya estaba almacenado, no vuelve a escribirlo y devuelve su id igualmente.
    fn write<T: GitObject>(
        &mut self,
        obj: &T,
    ) -> Result<<T::H as HashStrategy>::Digest, StoreError> {
        // serialización
        let raw = obj.encode();
        // hash de la serialización (su id)
        let id = T::H::hash(&raw);
        if !self.contains(id.as_ref())? {
            self.put_raw(id.as_ref(), &raw)?;
        }
        Ok(id)
    }

    /// Lee el objeto identificado por `id`, verifica su integridad y lo deserializa según el *digest* con
    /// el que fue hasheado.
    /// 
    /// Falla con [`StoreError::IntegrityMismatch`] si el hash de los bytes leídos no coincide
    /// con `id`, con [`StoreError::KindMismatch`] si el tipo guardado no es `T::KIND`, o con
    /// [`StoreError::Deserialize`] si el cuerpo es inválido.
    fn read<T: GitObject + Deserializable>(
        &self,
        id: &<T::H as HashStrategy>::Digest,
    ) -> Result<T, StoreError> {
        let raw = self.get_raw(id.as_ref())?;
        if T::H::hash(&raw) != *id {
            return Err(StoreError::IntegrityMismatch);
        }
        // cualquier error de decode pasa a ser un StoreError::Deserialize
        T::decode(&raw).map_err(StoreError::Deserialize)  
    }
}

