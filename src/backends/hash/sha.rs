//! Estrategia SHA-1 basada en el *crate* `sha1`.
//!
//! ## Limitaciones conocidas (post-MVP)
//! - **SHA-1 ya no es resistente a colisiones:** basta para detectar corrupción accidental,
//!   pero no protege contra un atacante que fabrique dos objetos con el mismo id. Agregar
//!   `Sha256` (con el *crate* `sha2`) es cuestión de otra implementación de [`HashStrategy`]
//!   con `Digest = [u8; 32]`, sin tocar el resto del código.

use crate::hash::HashStrategy;

/// # Sha1
/// Estrategia de *hash* SHA-1, con *digests* de 20 bytes.
///
/// Es un tipo sin datos: solo se usa como parámetro genérico (ej. `Blob<Sha1>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sha1;

impl HashStrategy for Sha1 {
    type Digest = [u8; 20];
    const DIGEST_LEN: usize = 20;

    /// Calcula el SHA-1 de `data` en un solo paso con `sha1::Sha1::digest`.
    fn hash(data: &[u8]) -> Self::Digest {
        todo!()
    }

    /// Copia `bytes` en un `[u8; 20]`; con un largo distinto devuelve `None`
    /// (basta `<[u8; 20]>::try_from(bytes).ok()`).
    fn digest_from_bytes(bytes: &[u8]) -> Option<Self::Digest> {
        todo!()
    }
}
