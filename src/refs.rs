//! Abstrae el almacenamiento del *estado mutable* del repositorio, es decir, está pensado
//! para el manejo de `HEAD` y referencias con nombre como ramas, tags, remotos y stash.
//!
//! A diferencia de [`crate::store2::ObjectStore`], que guarda objetos inmutables indexados por su
//! contenido, `RefStore` administra los punteros humanos (como ramas) que avanzan con cada *commit*.
//!
//! ## Limitaciones conocidas
//! - **Sin modo *detached*:** [`Head`] solo contempla apuntar a una rama (`Head::Branch`).
//!   Se marca `#[non_exhaustive]` para extenderlo a futuro (si una variante nueva requiriese
//!   guardar `H::Digest` directo en `Head`, implicaría parametrizar el tipo o modelarlo en un
//!   sub-trait sin alterar `RefStore`).
//! - **Actualización de una sola referencia por llamada:** `compare_and_swap` es atómico sobre una
//!   única referencia a la vez; aún no agrupa cambios de múltiples referencias en una transacción.
//! - **Listado completo en memoria:** `list_refs` devuelve todas las referencias en un `BTreeMap`
//!   sin paginación ni filtrado por prefijo, suficiente para el MVP pero costoso con miles de ramas.

use std::{collections::BTreeMap, error::Error, fmt, io};

use crate::hash::HashStrategy;

/// # Head
/// Representa la referencia activa donde está posicionado el área de trabajo,
/// puede ser una rama o eventualmente un commit (estado dettached, no implementado).
///
/// Se marca `#[non_exhaustive]` para permitir agregar nuevos estados de `HEAD`
/// manteniendo el módulo abierto a extensión.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Head {
    /// Apunta simbólicamente al nombre de una rama (tenga o no *commits* registrados)
    Branch(String),
}

/// # RefError
/// Error producido al consultar o actualizar referencias en un [`RefStore`].
#[derive(Debug)]
#[non_exhaustive]
pub enum RefError {
    /// Error de entrada/salida del *backend* subyacente
    Io(io::Error),
    /// El nombre de la referencia es vacío o contiene caracteres inválidos
    InvalidName(&'static str),
    /// El valor actual de la referencia no coincide con el esperado en `compare_and_swap`
    Conflict,
    /// Los datos almacenados de las referencias tienen un formato inválido
    Corrupt(&'static str),
}

impl fmt::Display for RefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "error de E/S en referencias: {err}"),
            Self::InvalidName(msg) => write!(f, "nombre de referencia inválido: {msg}"),
            Self::Conflict => write!(f, "conflicto de actualización concurrente en referencia"),
            Self::Corrupt(msg) => write!(f, "almacén de referencias corrupto: {msg}"),
        }
    }
}

impl Error for RefError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidName(_) | Self::Conflict | Self::Corrupt(_) => None,
        }
    }
}

impl From<io::Error> for RefError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Valida que `name` sea un nombre de referencia seguro tanto para registros de texto
/// como para rutas jerárquicas en disco.
///
/// Rechaza nombres vacíos, con caracteres de control o espacios, que empiecen o terminen en `'/'`,
/// o que contengan componentes vacíos (`"//"`), `"."` o `".."`.
pub fn validate_ref_name(name: &str) -> Result<(), RefError> {
    if name.is_empty() {
        return Err(RefError::InvalidName("el nombre no puede ser vacío"));
    }
    if name.starts_with('/') || name.ends_with('/') {
        return Err(RefError::InvalidName(
            "el nombre no puede iniciar ni terminar con '/'",
        ));
    }
    if name.bytes().any(|b| b <= b' ' || b == 0x7f) { // caracteres de control
        return Err(RefError::InvalidName(
            "el nombre contiene espacios o caracteres de control",
        ));
    }
    if name
        .split('/')
        .any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(RefError::InvalidName(
            "el nombre contiene un componente de ruta inválido",
        ));
    }
    Ok(())
}

/// # RefStore
/// Define la estrategia para consultar y actualizar las referencias mutables del repositorio.
///
/// Desacopla el manejo de ramas y `HEAD` del medio concreto de persistencia (memoria, archivos
/// sueltos o un registro *append-only* tipo WAL), permitiendo sumar nuevos *backends* o traits
/// de extensión sin modificar este módulo.
///
/// ## Ejemplo
/// ```ignore
/// // En un repositorio recién creado, HEAD apunta a "main" pero aún no hay commits
/// let parent = refs.resolve_head()?; // Ok(None)
///
/// // Tras guardar el primer Commit en ObjectStore, avanzamos la rama activa de forma atómica
/// refs.advance_head(parent.as_ref(), first_commit_id, "commit: inicial")?;
/// ```
pub trait RefStore<H: HashStrategy> {
    /// Devuelve el destino actual de `HEAD`.
    fn head(&self) -> Result<Head, RefError>;

    /// Cambia el destino al que apunta `HEAD`, registrando `reason` si el *backend* guarda historial.
    /// e.j. cambio de rama o entrar en modo dettached
    fn set_head(&mut self, target: Head, reason: &str) -> Result<(), RefError>;

    /// Devuelve el *digest* al que apunta la referencia `name`, o `Ok(None)` si aún no existe.
    fn get_ref(&self, name: &str) -> Result<Option<H::Digest>, RefError>;

    /// Devuelve todas las referencias existentes ordenadas alfabéticamente por nombre.
    fn list_refs(&self) -> Result<BTreeMap<String, H::Digest>, RefError>;

    /// Actualiza `name` a `new` (o la elimina si `new` es `None`) solo si su valor actual es `expected`.
    ///
    /// Falla con [`RefError::Conflict`] si la referencia cambió respecto a `expected`, o con
    /// [`RefError::InvalidName`] si `name` no cumple [`validate_ref_name`].
    fn compare_and_swap(
        &mut self,
        name: &str,
        expected: Option<&H::Digest>,
        new: Option<H::Digest>,
        reason: &str,
    ) -> Result<(), RefError>;

    /// Resuelve el *digest* del *commit* actual al que apunta `HEAD`, o `Ok(None)` si la rama aún no nace.
    fn resolve_head(&self) -> Result<Option<H::Digest>, RefError> {
        match self.head()? {
            Head::Branch(name) => self.get_ref(&name),
        }
    }

    /// Avanza la referencia activa en `HEAD` hacia `new_id`, verificando que antes estuviera en `expected`.
    fn advance_head(
        &mut self,
        expected: Option<&H::Digest>,
        new_id: H::Digest,
        reason: &str,
    ) -> Result<(), RefError> {
        match self.head()? {
            Head::Branch(name) => self.compare_and_swap(&name, expected, Some(new_id), reason),
        }
    }
}