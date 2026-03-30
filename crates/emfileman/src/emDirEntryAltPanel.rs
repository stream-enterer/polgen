//! Alternative content view for directory entries.
//!
//! Port of C++ `emDirEntryAltPanel`. Creates content via
//! `CreateFilePanel(..., alternative)` with incrementing alternative index.
//! Full panel rendering deferred to panel integration phase.

use crate::emDirEntry::emDirEntry;

/// Data for an alternative content view panel.
pub struct emDirEntryAltPanelData {
    pub dir_entry: emDirEntry,
    pub alternative: i32,
}

impl emDirEntryAltPanelData {
    pub fn new(dir_entry: emDirEntry, alternative: i32) -> Self {
        Self {
            dir_entry,
            alternative,
        }
    }
}
