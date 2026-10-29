//! A compiled CPK's record of which shared boots or gloves folder of a team took which of the
//! team's block's shared IDs: `unlimited-tool-works/shared_ids/<team_id>.toml`, a path no game
//! reads. A `Midcup` export numbered on its own would give a new folder the ID an
//! absent folder holds in the main CPK, so every compiled CPK carries the record its team's
//! IDs came from and a `Midcup` compiled over it keeps them.

use std::collections::BTreeMap;

use aesthetics_export::SharedKind;
use anyhow::Context;

/// One team's shared-IDs record: per kind, the shared ID each folder holds, the folder's name
/// under its own spelling.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SharedIds {
    /// The shared boots IDs taken.
    pub(crate) boots: BTreeMap<u16, String>,
    /// The shared gloves IDs taken.
    pub(crate) gloves: BTreeMap<u16, String>,
}

impl SharedIds {
    /// `bytes` as a team's record: UTF-8 TOML with a `[boots]` and a `[gloves]` table of
    /// shared ID to folder name, each table read as empty when absent. A file that does not
    /// parse is as unreadable as a bin that does not (the walk's `installed_bin_unreadable`).
    pub(crate) fn read(bytes: &[u8]) -> anyhow::Result<SharedIds> {
        let text = String::from_utf8(bytes.to_vec()).context("not UTF-8")?;
        let table: toml::Table = text.parse().context("not TOML")?;
        Ok(SharedIds {
            boots: table_of(&table, "boots")?,
            gloves: table_of(&table, "gloves")?,
        })
    }

    /// The record's bytes in the file's layout: the comment line, then the `[boots]` and
    /// `[gloves]` tables, each written even when empty.
    pub(crate) fn write(&self) -> Vec<u8> {
        let mut text = "# Written by the Team compiler; do not edit.\n".to_owned();
        for (name, table) in [("boots", &self.boots), ("gloves", &self.gloves)] {
            text.push_str(&format!("[{name}]\n"));
            for (id, folder) in table {
                text.push_str(&format!("{id} = {}\n", toml::Value::from(folder.as_str())));
            }
        }
        text.into_bytes()
    }

    /// Whether neither kind's table names a folder: a `Midcup` over no installed record
    /// writes none rather than an empty one, which a later `Midcup` would read as "no shared
    /// folders".
    pub(crate) fn is_empty(&self) -> bool {
        self.boots.is_empty() && self.gloves.is_empty()
    }

    /// The record's table of `kind`, boots or gloves; a shared face takes no shared ID, so
    /// the record has none for it.
    pub(crate) fn of(&self, kind: SharedKind) -> &BTreeMap<u16, String> {
        match kind {
            SharedKind::Boots => &self.boots,
            SharedKind::Gloves => &self.gloves,
            SharedKind::Face => unreachable!("a shared face takes no shared ID"),
        }
    }

    /// The record's table of `kind`, to be written into.
    fn of_mut(&mut self, kind: SharedKind) -> &mut BTreeMap<u16, String> {
        match kind {
            SharedKind::Boots => &mut self.boots,
            SharedKind::Gloves => &mut self.gloves,
            SharedKind::Face => unreachable!("a shared face takes no shared ID"),
        }
    }

    /// This record as the export's compile writes it, `assigned` (shared ID, the folder's
    /// name under its own spelling) merged into `kind`'s table: a folder named there already,
    /// names compared case-folded, is rewritten under the export's spelling, and every other
    /// entry, an absent folder's included, kept.
    pub(crate) fn extended(&self, kind: SharedKind, assigned: &[(u16, String)]) -> SharedIds {
        let mut record = self.clone();
        let table = record.of_mut(kind);
        for (id, name) in assigned {
            let folded = vtree::fold_name(name);
            table.retain(|_, held| vtree::fold_name(held) != folded);
            table.insert(*id, name.clone());
        }
        record
    }
}

/// `table`'s `[name]` table as shared ID to folder name: empty when absent, an error when it
/// is no table, a key is no shared ID or a value no folder name.
fn table_of(table: &toml::Table, name: &str) -> anyhow::Result<BTreeMap<u16, String>> {
    let Some(value) = table.get(name) else {
        return Ok(BTreeMap::new());
    };
    let entries = value
        .as_table()
        .with_context(|| format!("[{name}] is not a list of shared IDs and folder names"))?;
    entries
        .iter()
        .map(|(key, value)| {
            let id: u16 = key
                .parse()
                .with_context(|| format!("in [{name}], {key} is not a shared ID"))?;
            let folder = value
                .as_str()
                .with_context(|| format!("in [{name}], {key}'s value is not a folder name"))?;
            Ok((id, folder.to_owned()))
        })
        .collect()
}
