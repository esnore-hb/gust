

use crate::hash::{Deserializable, DeserializeError, GitObject, HashStrategy, Serializable};
use crate::objects::Meta;

/// # Commit
/// Representa un *snapshot* del proyecto en un punto concreto del historial
///
/// Contiene el [`Tree`] de la raíz con los *commits* padres que le dieron origen,
/// junto a un mensaje descriptivo y sus metadatos (ej. `"author"`, `"committer"` o `"date"`).
///
/// ## Ejemplo
/// ```ignore
/// // Definimos los metadatos básicos del commit
/// let mut fields = BTreeMap::new();
/// fields.insert(String::from("author"), String::from("Linus Torvalds"));
///
/// // Creamos el commit apuntando al tree raíz y al commit anterior
/// let commit = Commit::<HashKetchum> {
///     root_hash: root_tree_id,
///     parents: vec![parent_commit_id],
///     message: String::from("feat: Agrega soporte inicial para commits"),
///     meta: Meta { fields },
/// };
/// ```
pub struct Commit<H: HashStrategy> {
    ///  Hash del directorio raíz del proyecto, es decir, del [`Tree`] raíz. Captura el estado completo del proyecto
    pub root_hash: H::Digest,
    /// Hash de los commits padres (vacío en el commit raíz, 2+ en un merge)
    pub parents: Vec<H::Digest>,
    /// Mensaje del commit
    pub message: String,
    /// Metadatos asociados al commit, como autor, fecha, etc.
    pub meta: Meta,
}

/// Prefijo de la línea que apunta al [`Tree`] raíz en un [`Commit`] serializado
const TREE_PREFIX: &[u8] = b"tree ";
/// Prefijo de cada línea que apunta a un *commit* padre en un [`Commit`] serializado
const PARENT_PREFIX: &[u8] = b"parent ";

/// Extrae `<digest>\n` desde el inicio de `bytes`, validando el largo y el salto de línea final.
fn parse_digest_line<H: HashStrategy>(
    bytes: &[u8],
) -> Result<(H::Digest, &[u8]), DeserializeError> {
    if bytes.len() < H::DIGEST_LEN + 1 {
        return Err(DeserializeError::UnexpectedEof);
    }
    let (digest_bytes, rest) = bytes.split_at(H::DIGEST_LEN);
    let rest = rest
        .strip_prefix(b"\n")
        .ok_or(DeserializeError::InvalidFormat(
            "se esperaba '\\n' tras el digest",
        ))?;
    let digest = H::digest_from_bytes(digest_bytes)
        .ok_or(DeserializeError::InvalidFormat("largo de digest inválido"))?;
    Ok((digest, rest))
}

impl<H: HashStrategy> Serializable for Commit<H> {
    /// Formato:
    /// ```text
    /// tree <digest raíz>\n
    /// parent <digest>\n      (0 o más, en orden de inserción)
    /// \n
    /// <meta: key value\n ...>
    /// \n
    /// <message>
    /// ```
    /// Los prefijos y las líneas en blanco separan cada sección sin ambigüedad,
    /// incluso si un digest crudo contiene el byte `\n`. El mensaje va al final,
    /// por lo que puede contener cualquier texto.
    fn serialize(&self) -> Vec<u8> {
        let meta = self.meta.serialize();
        let parents_len: usize = self
            .parents
            .iter()
            .map(|p| PARENT_PREFIX.len() + p.as_ref().len() + 1)
            .sum();
        let total_len = TREE_PREFIX.len()
            + self.root_hash.as_ref().len()
            + 1
            + parents_len
            + 1
            + meta.len()
            + 1
            + self.message.len();
        let mut buff = Vec::with_capacity(total_len);

        buff.extend_from_slice(TREE_PREFIX);
        buff.extend_from_slice(self.root_hash.as_ref());
        buff.push(b'\n');

        // Orden de inserción: el primer padre es la rama donde se hizo el merge
        for parent in &self.parents {
            buff.extend_from_slice(PARENT_PREFIX);
            buff.extend_from_slice(parent.as_ref());
            buff.push(b'\n');
        }
        buff.push(b'\n');

        buff.extend_from_slice(&meta);
        buff.push(b'\n');

        buff.extend_from_slice(self.message.as_bytes());

        buff
    }
}

impl<H: HashStrategy> Deserializable for Commit<H> {
    /// Reconstruye un [`Commit`] a partir del formato producido por [`Serializable::serialize`].
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let rest = bytes
            .strip_prefix(TREE_PREFIX)
            .ok_or(DeserializeError::InvalidFormat("falta prefijo tree"))?;
        let (root_hash, mut rest) = parse_digest_line::<H>(rest)?;

        let mut parents = Vec::new();
        while let Some(after_parent) = rest.strip_prefix(PARENT_PREFIX) {
            let (parent, tail) = parse_digest_line::<H>(after_parent)?;
            parents.push(parent);
            rest = tail;
        }

        if rest.is_empty() {
            return Err(DeserializeError::UnexpectedEof);
        }
        let rest = rest
            .strip_prefix(b"\n")
            .ok_or(DeserializeError::InvalidFormat(
                "se esperaba línea en blanco tras padres",
            ))?;

        let (meta_bytes, msg_bytes): (&[u8], &[u8]) = if let Some(after) = rest.strip_prefix(b"\n")
        {
            (&[], after)
        } else {
            let sep = rest
                .windows(2)
                .position(|w| w == b"\n\n")
                .ok_or(DeserializeError::UnexpectedEof)?;
            (&rest[..sep + 1], &rest[sep + 2..])
        };

        let meta = Meta::deserialize(meta_bytes)?;
        let message = std::str::from_utf8(msg_bytes)?.to_owned();

        Ok(Self {
            root_hash,
            parents,
            message,
            meta,
        })
    }
}

impl<H: HashStrategy> GitObject for Commit<H> {
    type H = H;
    const KIND: &'static str = "commit";
}