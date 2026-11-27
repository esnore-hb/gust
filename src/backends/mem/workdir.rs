use std::collections::BTreeMap;

use crate::workdir::{DirEntry, WorkDir, WorkDirError, WorkDirMut};

/// # MemoryWorkDir
/// Simula un directorio de trabajo con un mapa de rutas completas a contenidos.
///
/// Solo guarda archivos: los directorios existen de forma implícita mientras contengan algún
/// archivo (ej. `"src/main.rs"` hace aparecer `"src"`), así que nunca hay directorios vacíos.
/// Para armar el escenario de una prueba se usa [`WorkDirMut::write_file`].
#[derive(Debug, Clone, Default)]
pub struct MemoryWorkDir {
    /// Ruta relativa con `'/'` (ej. `"src/main.rs"`) → contenido del archivo
    files: BTreeMap<String, Vec<u8>>,
}

impl WorkDir for MemoryWorkDir {
    /// Deduce las entradas de `dir` a partir de las rutas que empiezan con `"<dir>/"` (o de
    /// todas, si `dir` es la raíz): el primer segmento tras ese prefijo es un archivo si es el
    /// último, o un directorio si le siguen más. Cada nombre aparece una sola vez.
    ///
    /// Falla con [`WorkDirError::NotFound`] si ninguna ruta está bajo `dir` (salvo en la raíz,
    /// que siempre existe).
    fn read_dir(&self, dir: &str) -> Result<Vec<DirEntry>, WorkDirError> {
        todo!()
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, WorkDirError> {
        todo!()
    }
}

impl WorkDirMut for MemoryWorkDir {
    /// Inserta o reemplaza `path`.
    ///
    /// Falla con [`WorkDirError::InvalidName`] si `path` choca con un directorio implícito
    /// (ej. escribir `"src"` cuando ya existe `"src/main.rs"`) o al revés.
    fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), WorkDirError> {
        todo!()
    }

    /// Quita `path` del mapa; sus directorios padres desaparecen solos si quedan vacíos.
    fn remove_file(&mut self, path: &str) -> Result<(), WorkDirError> {
        todo!()
    }
}
