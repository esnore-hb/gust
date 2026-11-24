/// store2.rs
/// Su principal objetivo es abstraer la lógica del almacenamiento de objetos.
/// 
/// Similar a `HashStrategy`, `ObjectStore` define de manera general una *estrategia*
/// para almacenar objetos.
/// 
/// Por ejemplo, una estrategia `MemoryStore` implementa `put_raw`, `get_raw` y `content` que
/// recupera y almacena los objetos en memoria RAM, mientras que una estrategia `FsStore` puede
/// implementarlo en disco.
/// 
/// Es más, en este útlimo caso, se podría definir una `CompressionStrategy` para que FsStore
/// pueda almacenar usando algún tipo de compresión.

use crate::hash::{HashStrategy, Deserializable, GitObject, DeserializeError};
use std::io;

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

