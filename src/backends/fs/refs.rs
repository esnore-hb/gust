use std::{collections::BTreeMap, marker::PhantomData};

use crate::hash::HashStrategy;
use crate::refs::{Head, RefError, RefStore};

use super::Layout;

/// # FsRefs
/// Guarda `HEAD` y cada referencia como un archivo de texto dentro de `.gust/`, y registra cada
/// cambio con su razón en `.gust/logs/refs`.
///
/// Usa un archivo de bloqueo por referencia para que `compare_and_swap` sea atómico también
/// entre procesos distintos.
///
/// ## *Compare & swap* en disco
/// 1. Crea `refs/<name>.lock` con `OpenOptions::create_new`: si ya existe, otro proceso está
///    escribiendo y la operación falla con [`RefError::Conflict`].
/// 2. Lee el valor actual y lo compara con `expected`; si no coincide, borra el *lock* y falla.
/// 3. Escribe el nuevo valor en el *lock* y lo renombra sobre `refs/<name>` (o borra ambos si
///    `new` es `None`).
/// 4. Agrega una línea a `logs/refs` con la referencia, el valor anterior, el nuevo y `reason`.
///
/// ## Limitaciones conocidas (post-MVP)
/// - **Nombres terminados en `.lock`:** una rama llamada así chocaría con el archivo de bloqueo
///   de otra; falta reservar ese sufijo en [`validate_ref_name`].
/// - ***Locks* huérfanos:** si el proceso muere entre los pasos 1 y 3, el `.lock` queda en disco
///   y bloquea la referencia hasta que alguien lo borre a mano.
///
/// [`validate_ref_name`]: crate::refs::validate_ref_name
#[derive(Debug, Clone)]
pub struct FsRefs<H: HashStrategy> {
    layout: Layout,
    // `H` solo fija el tipo de digest que se lee y escribe; no se guarda ningún valor de él
    _hash: PhantomData<H>,
}

impl<H: HashStrategy> FsRefs<H> {
    /// Abre las referencias de un repositorio ya inicializado con [`Layout::init`].
    pub fn new(layout: Layout) -> Self {
        todo!()
    }
}

impl<H: HashStrategy> RefStore<H> for FsRefs<H> {
    /// Lee `.gust/HEAD` y lo interpreta como el nombre de una rama.
    ///
    /// Falla con [`RefError::Corrupt`] si el archivo está vacío o el nombre no es válido.
    fn get_head(&self) -> Result<Head, RefError> {
        todo!()
    }

    /// Reemplaza `.gust/HEAD` con escritura atómica y registra `reason` en el *log*.
    fn set_head(&mut self, target: Head, reason: &str) -> Result<(), RefError> {
        todo!()
    }

    /// Lee `refs/<name>` y decodifica el hexadecimal con [`HashStrategy::digest_from_bytes`].
    ///
    /// Si el archivo no existe devuelve `Ok(None)`; si no es un *digest* válido, falla con
    /// [`RefError::Corrupt`].
    fn get_ref(&self, name: &str) -> Result<Option<H::Digest>, RefError> {
        todo!()
    }

    /// Recorre `refs/` recursivamente y arma el nombre de cada referencia uniendo las carpetas
    /// con `'/'`. Ignora los archivos `.lock` de escrituras en curso.
    fn list_refs(&self) -> Result<BTreeMap<String, H::Digest>, RefError> {
        todo!()
    }

    /// Actualiza `name` bajo un archivo de bloqueo; los pasos están descritos en [`FsRefs`].
    fn compare_and_swap(
        &mut self,
        name: &str,
        expected: Option<&H::Digest>,
        new: Option<H::Digest>,
        reason: &str,
    ) -> Result<(), RefError> {
        todo!()
    }
}
