

use crate::hash::{Deserializable, DeserializeError, GitObject, HashStrategy, Serializable};

/// Valida que `name` sea un único componente de ruta válido para una [`TreeEntry`].
///
/// Se rechazan `""`, `"."`, `".."` y nombres con `'/'` o `'\0'` para que un *tree* corrupto o
/// malicioso no pueda, al restaurar el directorio de trabajo, escribir archivos fuera del
/// repositorio (ej. `"../../etc"`), y para que el nombre no choque con el separador `\0` del
/// formato serializado.
pub fn validate_entry_name(name: &str) -> Result<(), DeserializeError> {
    if matches!(name, "" | "." | "..") || name.contains(['/', '\0']) {
        return Err(DeserializeError::InvalidFormat(
            "nombre de entrada inválido",
        ));
    }
    Ok(())
}

/// # EntryKind
/// Indica el tipo de objeto al que apunta una entrada dentro de un [`Tree`]
pub enum EntryKind {
    /// Archivo individual con contenido crudo ([`Blob`])
    Blob,
    /// Un subdirectorio que agrupa otras entradas ([`Tree`])
    Tree,
}

/// # TreeEntry
/// Representa un elemento individual de un directorio ([`Tree`]).
///
/// Vincula un nombre con el tipo de objeto y el hash que lo identifica.
pub struct TreeEntry<H: HashStrategy> {
    /// Tipo de objeto que referencia (archivo o subdirectorio)
    pub kind: EntryKind,
    /// Nombre del archivo o subdirectorio (ej. `a.txt` o `src`)
    pub name: String,
    /// Identificador del objeto al que apunta esta entrada
    pub hash: H::Digest,
}

impl EntryKind {
    /// Devuelve el nombre del tipo, igual al `KIND` del objeto referenciado.
    fn as_str(&self) -> &'static str {
        match self {
            EntryKind::Blob => "blob",
            EntryKind::Tree => "tree",
        }
    }

    /// Reconstruye el tipo a partir de su representación en bytes (`"blob"` o `"tree"`).
    fn from_bytes(bytes: &[u8]) -> Result<Self, DeserializeError> {
        match bytes {
            b"blob" => Ok(EntryKind::Blob),
            b"tree" => Ok(EntryKind::Tree),
            _ => Err(DeserializeError::InvalidFormat(
                "tipo de entrada desconocido",
            )),
        }
    }
}

impl<H: HashStrategy> TreeEntry<H> {
    /// Cantidad exacta de bytes que ocupa la entrada serializada.
    fn serialized_len(&self) -> usize {
        self.kind.as_str().len() + 1 + self.name.len() + 1 + self.hash.as_ref().len()
    }

    /// Escribe la entrada serializada al final de `buff`, sin reservas intermedias.
    fn write_into(&self, buff: &mut Vec<u8>) {
        buff.extend_from_slice(self.kind.as_str().as_bytes());
        buff.push(b' ');
        buff.extend_from_slice(self.name.as_bytes());
        buff.push(0);
        buff.extend_from_slice(self.hash.as_ref());
    }

    /// Lee una entrada desde el inicio de `bytes` y devuelve la entrada junto al resto del slice.
    ///
    /// ## Ejemplo
    /// ```ignore
    /// // Suponiendo `HashKetchum` con digests de 4 bytes (`[u8; 4]`)
    /// let (entry, rest) = TreeEntry::<HashKetchum>::from_bytes(b"blob README.md\0\xab\xcd\xef\x01tree src\0...")?;
    /// // entry.kind --> EntryKind::Blob
    /// // entry.name --> "README.md"
    /// // entry.hash --> [0xab, 0xcd, 0xef, 0x01]
    /// // rest       --> b"tree src\0..."
    /// ```
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), DeserializeError> {
        let space = bytes
            .iter()
            .position(|&b| b == b' ')
            .ok_or(DeserializeError::UnexpectedEof)?;
        let kind = EntryKind::from_bytes(&bytes[..space])?;
        let rest = &bytes[space + 1..];

        let nul = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(DeserializeError::UnexpectedEof)?;
        let name = std::str::from_utf8(&rest[..nul])?;
        validate_entry_name(name)?;

        // El digest tiene largo fijo: si faltan bytes, la entrada está truncada
        let (digest_bytes, remaining) = rest[nul + 1..]
            .split_at_checked(H::DIGEST_LEN)
            .ok_or(DeserializeError::UnexpectedEof)?;
        let hash = H::digest_from_bytes(digest_bytes)
            .ok_or(DeserializeError::InvalidFormat("largo de digest inválido"))?;

        Ok((
            Self {
                kind,
                name: name.to_owned(),
                hash,
            },
            remaining,
        ))
    }
}

impl<H: HashStrategy> Serializable for TreeEntry<H> {
    /// Formato: `<kind> <name>\0<digest crudo>`.
    ///
    /// El nombre termina en `\0` porque no puede aparecer en un nombre de archivo,
    /// y el digest tiene largo fijo, por lo que no necesita separador final.
    fn serialize(&self) -> Vec<u8> {
        let mut buff = Vec::with_capacity(self.serialized_len());
        self.write_into(&mut buff);
        buff
    }
}

impl<H: HashStrategy> Deserializable for TreeEntry<H> {
    /// Reconstruye una entrada a partir del formato `<kind> <name>\0<digest crudo>`.
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let (entry, remaining) = Self::from_bytes(bytes)?;
        if !remaining.is_empty() {
            return Err(DeserializeError::InvalidFormat(
                "bytes sobrantes tras TreeEntry",
            ));
        }
        Ok(entry)
    }
}

/// # Tree
/// Representa un *snapshot* de un directorio mediante la lista de sus entradas.
///
/// Similar a Git, no guarda el contenido de los archivos directamente, sino referencias ([`TreeEntry`])
/// a cada archivo o subcarpeta.
/// ## Ejemplo
/// ```ignore
/// let file_entry = TreeEntry::<HashKetchum> {
///     kind: EntryKind::Blob,
///     name: String::from("README.md"),
///     hash: blob_id,
/// };
/// // No guarda archivos, ya que no almacena ningún blob.
/// // En algún momento se procesó el blob del archivo y
/// // se calculó su hash.
///
/// let dir = Tree {
///     entries: vec![file_entry],
/// };
/// ```
pub struct Tree<H: HashStrategy> {
    pub entries: Vec<TreeEntry<H>>,
}


impl<H: HashStrategy> Tree<H> {
    /// Construye un [`Tree`] válido y ordenado: entradas ordenadas por los bytes del nombre,
    /// sin nombres repetidos ni inválidos (ver [`validate_entry_name`]).
    ///
    /// Un [`Tree`] armado a mano con nombres repetidos se serializa sin error, pero su lectura
    /// posterior falla; este constructor detecta ese caso al crearlo.
    pub fn from_entries(mut entries: Vec<TreeEntry<H>>) -> Result<Self, DeserializeError> {
        for entry in &entries {
            validate_entry_name(&entry.name)?;
        }
        entries.sort_unstable_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        if entries.windows(2).any(|w| w[0].name == w[1].name) {
            return Err(DeserializeError::InvalidFormat(
                "entradas de tree duplicadas",
            ));
        }
        Ok(Self { entries })
    }
}



impl<H: HashStrategy> Serializable for Tree<H> {
    /// Formato: `<entry_1><entry_2>...<entry_n>`, ver [`TreeEntry`].
    ///
    /// Concatena las entradas serializadas, ordenadas por los bytes de su nombre.
    /// Un [`Tree`] vacío produce 0 bytes.
    ///
    /// El orden garantiza que el mismo directorio produzca siempre los mismos bytes,
    /// sin importar el orden en que se insertaron las entradas.
    fn serialize(&self) -> Vec<u8> {
        // Se ordenan referencias para no modificar ni clonar las entradas
        let mut sorted: Vec<&TreeEntry<H>> = self.entries.iter().collect();
        sorted.sort_unstable_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));

        let total_len: usize = sorted.iter().map(|e| e.serialized_len()).sum();
        let mut buff = Vec::with_capacity(total_len);

        for entry in sorted {
            entry.write_into(&mut buff);
        }

        buff
    }
}

impl<H: HashStrategy> Deserializable for Tree<H> {
    /// Reconstruye un [`Tree`] leyendo entradas consecutivas hasta consumir todos los bytes.
    ///
    /// Una secuencia vacía produce un [`Tree`] sin entradas.
    ///
    /// Rechaza entradas desordenadas o con nombres repetidos, ya que [`Serializable::serialize`]
    /// nunca las produciría y el *tree* leído tendría un id distinto al de sus bytes.
    fn deserialize(mut bytes: &[u8]) -> Result<Self, DeserializeError> {
        let mut entries: Vec<TreeEntry<H>> = Vec::new();
        while !bytes.is_empty() {
            let (entry, remaining) = TreeEntry::from_bytes(bytes)?;
            // Evita nombres desordenados o duplicados
            if entries
                .last()
                .is_some_and(|prev| prev.name.as_bytes() >= entry.name.as_bytes())
            {
                return Err(DeserializeError::InvalidFormat(
                    "entradas de tree desordenadas o duplicadas",
                ));
            }
            entries.push(entry);
            bytes = remaining;
        }
        Ok(Self { entries })
    }
}

impl<H: HashStrategy> GitObject for Tree<H> {
    type H = H;
    const KIND: &'static str = "tree";
}