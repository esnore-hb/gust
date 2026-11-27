use std::collections::BTreeMap;

use crate::hash::HashStrategy;
use crate::refs::{Head, RefError, RefStore};

/// # MemoryRefs
/// Guarda `HEAD`, las referencias y su registro de cambios en estructuras en memoria.
///
/// El registro queda accesible con [`MemoryRefs::log`], para que las pruebas verifiquen qué
/// razones se registraron y en qué orden.
pub struct MemoryRefs<H: HashStrategy> {
    head: Head,
    refs: BTreeMap<String, H::Digest>,
    /// Razón de cada mutación exitosa, en el orden en que ocurrió
    log: Vec<String>,
}

impl<H: HashStrategy> MemoryRefs<H> {
    /// Crea un almacén vacío con `HEAD` apuntando a `initial_branch`, que aún no existe
    /// (igual que un repositorio recién inicializado).
    pub fn new(initial_branch: &str) -> Self {
        todo!()
    }

    /// Razones registradas por `set_head` y `compare_and_swap`, de la más antigua a la más reciente.
    pub fn log(&self) -> &[String] {
        todo!()
    }
}

impl<H: HashStrategy> RefStore<H> for MemoryRefs<H> {
    fn get_head(&self) -> Result<Head, RefError> {
        todo!()
    }

    /// Reemplaza `HEAD` y agrega `reason` al registro.
    fn set_head(&mut self, target: Head, reason: &str) -> Result<(), RefError> {
        todo!()
    }

    fn get_ref(&self, name: &str) -> Result<Option<H::Digest>, RefError> {
        todo!()
    }

    /// Devuelve una copia del mapa: ya está ordenado por nombre.
    fn list_refs(&self) -> Result<BTreeMap<String, H::Digest>, RefError> {
        todo!()
    }

    /// Valida `name`, compara el valor actual con `expected` y, si coinciden, inserta o borra la
    /// entrada y registra `reason`. Si no coinciden, no modifica nada ni registra la razón.
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
