//! Implementaciones concretas de las estrategias del repositorio: dónde y cómo se guardan los
//! objetos ([`ObjectStore`]), las referencias ([`RefStore`]) y el directorio de trabajo
//! ([`WorkDir`]), y con qué algoritmo se hashea ([`HashStrategy`]).
//!
//! Los traits viven en sus propios módulos y dependen solo de `std`. Este módulo es el único
//! lugar del crate donde se permiten dependencias externas (ej. `sha1`, `flate2`), de modo que
//! cambiar de medio o de algoritmo no obliga a tocar la lógica del repositorio.
//!
//! ## Organización
//! | Módulo   | Contenido                                                   | Uso                        |
//! |----------|-------------------------------------------------------------|----------------------------|
//! | [`fs`]   | `FsStore`, `FsRefs`, `FsWorkDir` y el layout de `.gust/`    | El repositorio real, en disco |
//! | [`mem`]  | `MemoryStore`, `MemoryRefs`, `MemoryWorkDir`                | Referencia y pruebas       |
//! | [`hash`] | `Sha1`                                                      | Algoritmo de *hash* concreto |
//!
//! Los *backends* en memoria sirven de **implementación de referencia**: son lo bastante
//! simples para ser obviamente correctos, así que una prueba puede ejecutar la misma secuencia
//! de operaciones sobre `mem` y `fs` y exigir que ambos se comporten igual.
//!
//! [`ObjectStore`]: crate::store::ObjectStore
//! [`RefStore`]: crate::refs::RefStore
//! [`WorkDir`]: crate::workdir::WorkDir
//! [`HashStrategy`]: crate::hash::HashStrategy

pub mod fs;
pub mod hash;
pub mod mem;
