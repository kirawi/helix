use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    num::NonZeroUsize,
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime},
};

use super::{error::StateError, History, Revision};
use crate::{combinators::*, ChangeSet, Operation, Range, Selection, Transaction};

const HASH_DIGEST_LENGTH: usize = 20;
fn get_hash<R: Read>(reader: &mut R) -> io::Result<[u8; HASH_DIGEST_LENGTH]> {
    let mut hasher = tenthash::TentHasher::new();
    let mut buf = [0u8; 8192];

    // Read until empty
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hasher.finalize())
}

impl Selection {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> io::Result<()> {
        write_usize(writer, self.primary_index)?;
        write_vec(writer, self.ranges(), |writer, range| {
            write_usize(writer, range.anchor)?;
            write_usize(writer, range.head)?;
            write_option(writer, range.old_visual_position.as_ref(), |writer, pos| {
                write_u32(writer, pos.0)?;
                write_u32(writer, pos.1)?;
                Ok(())
            })?;
            Ok(())
        })?;

        Ok(())
    }

    fn deserialize<R: std::io::Read>(reader: &mut R) -> io::Result<Self> {
        let primary_index = read_usize(reader)?;
        let ranges = read_vec(reader, |reader| {
            let anchor = read_usize(reader)?;
            let head = read_usize(reader)?;
            let old_visual_position = read_option(reader, |reader| {
                let res = (read_u32(reader)?, read_u32(reader)?);
                Ok(res)
            })?;
            Ok(Range {
                anchor,
                head,
                old_visual_position,
            })
        })?;
        Ok(Self {
            ranges: ranges.into(),
            primary_index,
        })
    }
}

impl Transaction {
    pub fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_option(writer, self.selection.as_ref(), |writer, selection| {
            selection.serialize(writer)
        })?;

        write_usize(writer, self.changes.len)?;
        write_usize(writer, self.changes.len_after)?;
        write_vec(writer, self.changes.changes(), |writer, operation| {
            let variant = match operation {
                Operation::Retain(_) => 0,
                Operation::Delete(_) => 1,
                Operation::Insert(_) => 2,
            };
            write_byte(writer, variant)?;
            match operation {
                Operation::Retain(n) | Operation::Delete(n) => {
                    write_usize(writer, *n)?;
                }

                Operation::Insert(tendril) => {
                    write_string(writer, tendril.as_str())?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    pub fn deserialize<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let selection = read_option(reader, Selection::deserialize)?;

        let len = read_usize(reader)?;
        let len_after = read_usize(reader)?;
        let changes = read_vec(reader, |reader| {
            let res = match read_byte(reader)? {
                0 => Operation::Retain(read_usize(reader)?),
                1 => Operation::Delete(read_usize(reader)?),
                2 => Operation::Insert(read_string(reader)?.into()),
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "invalid variant",
                    ))
                }
            };
            Ok(res)
        })?;
        let changes = ChangeSet {
            changes,
            len,
            len_after,
        };

        Ok(Transaction { changes, selection })
    }
}

impl Revision {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), StateError> {
        write_usize(writer, self.parent)?;
        self.transaction.serialize(writer)?;
        self.inversion.serialize(writer)?;
        write_time(writer, self.timestamp)?;
        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, StateError> {
        let parent = read_usize(reader)?;
        let transaction = Arc::new(Transaction::deserialize(reader)?);
        let inversion = Arc::new(Transaction::deserialize(reader)?);
        let timestamp = read_time(reader)?;
        Ok(Revision {
            parent,
            last_child: None,
            transaction,
            inversion,
            timestamp,
        })
    }
}

const UNDO_FILE_HEADER_TAG: &[u8] = b"Helix";
const UNDO_FILE_HEADER_LEN: usize = UNDO_FILE_HEADER_TAG.len();
const UNDO_FILE_VERSION: u8 = 1;

impl History {
    /// It is the responsibility of the caller to ensure the undofile is valid before serializing.
    /// This function performs no checks.
    // Serializes:
    // - Header:
    //     - UNDO_FILE_HEADER_TAG
    //     - UNDO_FILE_VERSION
    //     - Current revision at time of write
    //     - Hash of the file
    // - Revisions contiguously
    pub fn serialize<W: Write + Seek>(
        &self,
        writer: &mut W,
        path: &Path,
        // The offset after which to append new revisions
        offset: usize,
    ) -> Result<(), StateError> {
        // Header
        writer.write_all(UNDO_FILE_HEADER_TAG)?;
        write_byte(writer, UNDO_FILE_VERSION)?;

        // We save the current revision so that we reload at that revision later
        write_usize(writer, self.current)?;
        writer.write_all(&get_hash(&mut std::fs::File::open(path)?)?)?;

        // Append new revisions to the end of the file.
        write_usize(writer, self.revisions.len())?;
        writer.seek(SeekFrom::End(0))?;
        for rev in &self.revisions[offset..] {
            rev.serialize(writer)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Returns the deserialized [`History`] and the last_saved_revision.
    // Deserializes:
    // - Header
    // - Revisions
    pub fn deserialize<R: Read>(reader: &mut R, path: &Path) -> Result<(usize, Self), StateError> {
        let current = Self::read_header(reader, path)?;

        // Read the revisions and construct the tree.
        let len = read_usize(reader)?;
        let mut revisions: Vec<Revision> = Vec::with_capacity(len);
        for _ in 0..len {
            let rev = Revision::deserialize(reader)?;
            let len = revisions.len();

            // Check that order of revisions is correct before inserting
            match revisions.get_mut(rev.parent) {
                Some(r) => r.last_child = NonZeroUsize::new(len),
                None if len != 0 => {
                    return Err(StateError::InvalidData(format!(
                        "non-contiguous history: {} >= {}",
                        rev.parent, len
                    )));
                }
                None => {
                    // Starting revision check
                    let default_rev = History::default().revisions.pop().unwrap();
                    if rev != default_rev {
                        return Err(StateError::InvalidData(String::from(
                            "Missing 0th revision",
                        )));
                    }
                }
            }
            revisions.push(rev);
        }

        let history = History { current, revisions };
        Ok((current, history))
    }

    /// If `self.revisions = [A, B, C, D]` and `other.revisions = `[A, B, E, F]`, then
    /// they are merged into `[A, B, E, F, C, D]` where the tree can be represented as:
    /// ```md
    /// A -> B -> C -> D
    ///       \  
    ///        E -> F
    /// ```
    pub fn merge(&mut self, mut other: History) -> Result<(), StateError> {
        let n = self
            .revisions
            .iter()
            .zip(other.revisions.iter())
            .take_while(|(a, b)| {
                a.parent == b.parent && a.transaction == b.transaction && a.inversion == b.inversion
            })
            .count();

        let new_revs = self.revisions.split_off(n);
        if new_revs.is_empty() {
            return Ok(());
        }
        other.revisions.reserve_exact(n);

        // Only unique revisions in `self` matter, so saturating_sub(1) is sound as it going to 0 means there are no new revisions in the other history that aren't in `self`
        let offset = (other.revisions.len() - n).saturating_sub(1);
        for mut r in new_revs {
            // Update parents of new revisions
            if r.parent >= n {
                r.parent += offset;
            }
            debug_assert!(r.parent < other.revisions.len());

            // Update the corresponding parent.
            other.revisions.get_mut(r.parent).unwrap().last_child =
                NonZeroUsize::new(other.revisions.len());
            other.revisions.push(r);
        }

        if self.current >= n {
            self.current += offset;
        }
        self.revisions = other.revisions;

        Ok(())
    }

    pub fn is_valid<R: Read>(reader: &mut R, path: &Path) -> bool {
        Self::read_header(reader, path).is_ok()
    }

    // Deserializes:
    // - Checks for UNDO_FILE_HEADER
    // - Validates UNDO_FILE_VERSION
    // - Current revision
    // - Validates hash
    pub fn read_header<R: Read>(reader: &mut R, path: &Path) -> Result<usize, StateError> {
        let header: [u8; UNDO_FILE_HEADER_LEN] = read_many_bytes(reader)?;
        let version = read_byte(reader)?;
        if header != UNDO_FILE_HEADER_TAG || version != UNDO_FILE_VERSION {
            Err(StateError::InvalidHeader)
        } else {
            let current = read_usize(reader)?;
            let mut hash = [0u8; HASH_DIGEST_LENGTH];
            reader.read_exact(&mut hash)?;

            if hash != get_hash(&mut std::fs::File::open(path)?)? {
                return Err(StateError::Outdated);
            }

            Ok(current)
        }
    }
}
