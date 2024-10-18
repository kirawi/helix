use std::{
    fs::File,
    io::{Seek, Write},
    time::SystemTime,
};

use crate::{
    hash::Digest,
    history::{History, Revision},
};

/// TODO: Memorymap
/// ```
/// A -> (A->B) -> (B->C) -> (C->D)
///      \
///       (B->B1) -> (B1->B2)
///                   \
///                    (B2->B2a)
/// ```
/// Each state diff is represented via a hash of the file
/// Hashes are not guaranteed to be unique relative to each other.
/// ```
/// A[hash1] -> B[hash2] -> C[hash3] -> D[hash4]
///              \
///               B1[hash5] -> B2[hash6]
///                             \
///                              B2a[hash7]
/// ```
pub struct UndoStorage {
    pub timestamp: SystemTime,
    // Current is always most recent (last)
    pub nodes: Vec<UndoStorageNode>,
}

pub struct UndoStorageNode {
    // Hash of file
    pub hash: Digest,
    // Needed to disambiguate order, e.g. if one client older than the undofile writes its changes
    pub parent: Option<usize>,
    pub diff: UndoStateDiff,
}

/// Analagous to [`History`]
/// Unlike `History`, the changes here are not contiguous nor necessarily valid. The expectation is that the actual `History` will be reconstructed from continuously applying these diffs.
pub struct UndoStateDiff {
    revisions: Vec<Revision>,
    current: usize,
}

/// A map to each Node based on offsets in the format
/// Excludes revisions for memory savings
// TODO: All clients maintain this
// Required for merging by diffing this against the UndoStorage
pub struct UndoMap {
    pub timestamp: SystemTime,
    pub nodes: Vec<UndoMapNode>,
}

pub struct UndoMapNode {
    /// Hash of file
    pub hash: Digest,
    /// Parent node
    pub parent: Option<usize>,
    /// Number of revisions
    pub changes: usize,
}

// Interface for serializing/deserializing into storage or map,etc.
pub struct UndoStorageHandle<'a> {
    map: &'a UndoMap,
    file: File,
}

/*
    - This should work without needing to load the entire undofile into memory
    - The UndoFile will be an interface singleton on all clients. It will help to merge client histories too. It will store the parent hash/idx too.
*/
impl UndoMap {
    // TODO: Add fast path when undofile state is unchanged. Maybe a UUID?
    // TODO: Make panic-free
    pub fn commit(&mut self, history: &History, file_hash: Digest) {
        // First, I need to construct the diff
        // - Traverse up the undo tree to find the offset
        if let Some((mut parent_idx, parent_hash)) = history.undofile_parent {
            // TODO: Check if the parent's hash matches the one at the idx
            // Sum number of revisions in each
            if self.nodes[parent_idx].hash != parent_hash {
                todo!()
            }

            let mut offset = 0;
            loop {
                let node = &self.nodes[parent_idx];
                offset += node.diff.revisions.len();
                if let Some(ancestor_idx) = node.parent {
                    parent_idx = ancestor_idx;
                } else {
                    break;
                }
            }
            let revisions = history.get_revisions()[offset..].to_vec();
            let diff = UndoStateDiff {
                revisions,
                current: history.current_revision(),
            };
            self.nodes.push(UndoStorageNode {
                hash: file_hash,
                parent: Some(self.nodes.len() - 1),
                diff,
            });
        } else {
            let diff = UndoStateDiff {
                revisions: history.get_revisions().to_vec(),
                current: history.current_revision(),
            };
            self.nodes.push(UndoStorageNode {
                hash: file_hash,
                parent: None,
                diff,
            });
        }
    }
}

// Serializable impl
impl UndoStorage {
    pub fn serialize<W: Write + Seek>(&self, writer: &mut W) {}
}
