//! Recorrido del historial de *commits* a partir de un punto de partida.
//!
//! Es genérico sobre el algoritmo de hash ([`HashStrategy`]) y el almacén de objetos
//! ([`ObjectStore`]), así que no depende de ningún *backend* concreto.

use crate::{
    hash::HashStrategy,
    objects::Commit,
    store::{ObjectStore, StoreError},
};

/// # FirstParentLog
/// Recorre el historial desde un *commit* siguiendo siempre el primer padre.
/// Entrega `(id, commit)` hasta llegar al *commit* raíz o al primer error.
///
/// En un *merge* solo sigue el primer padre, por lo que no visita la rama integrada.
///
/// ## Ejemplo
/// ```ignore
/// // Imprime el historial desde HEAD
/// for entry in first_parent_log::<Sha1, _>(&store, refs.resolve_head()?) {
///     let (id, commit) = entry?;
///     println!("{id:?} {}", commit.message);
/// }
/// ```
pub struct FirstParentLog<'s, H: HashStrategy, S> {
    store: &'s S,
    next: Option<H::Digest>,
}

/// Crea un [`FirstParentLog`] que parte en `start`. Con `None` el iterador queda vacío.
pub fn first_parent_log<H: HashStrategy, S: ObjectStore>(
    store: &S,
    start: Option<H::Digest>,
) -> FirstParentLog<'_, H, S> {
    FirstParentLog { store, next: start }
}

impl<H: HashStrategy, S: ObjectStore> Iterator for FirstParentLog<'_, H, S> {
    type Item = Result<(H::Digest, Commit<H>), StoreError>;

    fn next(&mut self) -> Option<Self::Item> {
        // `take` deja `next` en None: si la lectura falla, el iterador termina solo
        let id = self.next.take()?;
        Some(self.store.read::<Commit<H>>(&id).map(|commit| {
            self.next = commit.parents.first().cloned();
            (id, commit)
        }))
    }
}
