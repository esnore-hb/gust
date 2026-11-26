

use std::{marker::PhantomData};

use crate::hash::{Deserializable, DeserializeError, GitObject, HashStrategy, Serializable};


/// # Blob
/// Representa el contenido crudo de un archivo como secuencia de bytes (*Binary Large Object*)
///
/// Solo guarda los datos del archivo, sin su nombre ni permisos. El parámetro genérico `H`
/// define la [`HashStrategy`] con la que se calculará su id al usarlo como [`GitObject`].
pub struct Blob<H: HashStrategy> {
    /// Contenido crudo almacenado como un vector de bytes
    content: Vec<u8>,
    /// *HashStrategy* a utilizar
    _h: PhantomData<H>,
}

impl<H: HashStrategy> Blob<H> {
    /// Crea un [`Blob`] tomando posesión del contenido, sin copiarlo.
    pub fn new(content: Vec<u8>) -> Self {
        Self {
            content,
            _h: PhantomData,
        }
    }

    /// Devuelve el contenido crudo del archivo.
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

impl<H: HashStrategy> Serializable for Blob<H> {
    /// Devuelve una copia de los bytes crudos del archivo.
    ///
    /// Formato: `<content>`, sin header ni separadores.
    fn serialize(&self) -> Vec<u8> {
        self.content.clone()
    }
}

impl<H: HashStrategy> Deserializable for Blob<H> {
    /// Reconstruye el [`Blob`] copiando los bytes crudos recibidos.
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        Ok(Self {
            content: bytes.to_vec(),
            _h: PhantomData,
        })
    }
}

impl<H: HashStrategy> GitObject for Blob<H> {
    type H = H;
    const KIND: &'static str = "blob";
}