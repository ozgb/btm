// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/ota.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! OTA map and mission descriptions: TDF text with a `GlobalHeader` section.
//!
//! The parse is [`crate::tdf`]'s, unchanged; this module reads the map-menu and game-start
//! fields out of it and keeps the whole document for mission-specific consumers.
//!
//! Differences from the Go source:
//! - Go's `OTA` held `Global *Section` and each `OTASchema` held `Section *Section`,
//!   pointers into the document. Rust cannot store a borrow of a sibling field, so they
//!   are methods instead: [`Ota::global`] and [`Ota::schema_section`]. They return the
//!   same sections Go held.
//! - Text values are byte strings, because TDF values are (the retail files are
//!   Windows-1252).
//! - Go found each `Schema N` with a linear scan of the global section, which is
//!   quadratic in the number of sections for a file that holds thousands of them. Here
//!   the sections are indexed once by folded name (first one wins, as in the scan), which
//!   gives the same projection in linear time.
//! - `LoadOTAFile` (VFS read) is not ported.

use std::collections::HashMap;
use std::fmt;

use crate::tdf::{self, Document, Limits, Section};

/// Everything that can go wrong reading an OTA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtaError {
    /// The TDF text did not parse.
    Parse(tdf::Error),
    /// The document has no `GlobalHeader` section.
    MissingGlobalHeader,
}

impl fmt::Display for OtaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OtaError::Parse(e) => write!(f, "ota: parse: {e}"),
            OtaError::MissingGlobalHeader => write!(f, "ota: missing GlobalHeader"),
        }
    }
}

impl std::error::Error for OtaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OtaError::Parse(e) => Some(e),
            OtaError::MissingGlobalHeader => None,
        }
    }
}

/// One `Schema N` section of a map or mission: the per-difficulty starting resources and
/// AI profile. Values are the authored text, not parsed numbers. A key the section does
/// not author is empty. Use [`Ota::schema_section`] for the other keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OtaSchema {
    /// The section's name as authored, e.g. `Schema 0`.
    pub name: Vec<u8>,
    pub kind: Vec<u8>,
    pub ai_profile: Vec<u8>,
    pub human_metal: Vec<u8>,
    pub human_energy: Vec<u8>,
    pub computer_metal: Vec<u8>,
    pub computer_energy: Vec<u8>,
}

/// The map metadata the menus and the game-start path need, plus the whole document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ota {
    document: Document,
    pub mission_name: Vec<u8>,
    pub mission_description: Vec<u8>,
    pub memory: Vec<u8>,
    pub num_players: Vec<u8>,
    pub size: Vec<u8>,
    /// `Schema 0`, `Schema 1`, ... up to the first index the global section lacks.
    pub schemas: Vec<OtaSchema>,
}

fn value(section: &Section, key: &str) -> Vec<u8> {
    section.string_value(key, b"").0.to_vec()
}

impl Ota {
    /// Parse a map or mission definition under the default TDF limits. A file without a
    /// `GlobalHeader` section is an error.
    pub fn parse(data: &[u8]) -> Result<Ota, OtaError> {
        Self::parse_with_limits(data, Limits::default())
    }

    /// Parse under explicit TDF limits.
    pub fn parse_with_limits(data: &[u8], limits: Limits) -> Result<Ota, OtaError> {
        let document = Document::parse_with_limits(data, limits).map_err(OtaError::Parse)?;
        let global = document
            .root()
            .section("GlobalHeader")
            .ok_or(OtaError::MissingGlobalHeader)?;
        // Go read these through `LanguageString("", key, "")`, which with no language is
        // the plain string lookup.
        let (mission_name, _) = global.language_string("", "missionname", b"");
        let (mission_description, _) = global.language_string("", "missiondescription", b"");
        let mission_name = mission_name.to_vec();
        let mission_description = mission_description.to_vec();
        let memory = value(global, "memory");
        let num_players = value(global, "numplayers");
        let size = value(global, "size");

        // The semantic projection stops at the first missing index; the document keeps
        // every raw section for tooling.
        let mut by_name: HashMap<&[u8], &Section> = HashMap::new();
        for section in global.sections() {
            by_name.entry(section.name()).or_insert(section);
        }
        let mut schemas = Vec::new();
        for index in 0usize.. {
            let wanted = format!("schema {index}");
            let Some(section) = by_name.get(wanted.as_bytes()) else {
                break;
            };
            schemas.push(OtaSchema {
                name: section.original_name().to_vec(),
                kind: value(section, "type"),
                ai_profile: value(section, "aiprofile"),
                human_metal: value(section, "humanmetal"),
                human_energy: value(section, "humanenergy"),
                computer_metal: value(section, "computermetal"),
                computer_energy: value(section, "computerenergy"),
            });
        }
        Ok(Ota {
            document,
            mission_name,
            mission_description,
            memory,
            num_players,
            size,
            schemas,
        })
    }

    /// The whole TDF document.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The `GlobalHeader` section. Present for every parsed `Ota`.
    pub fn global(&self) -> Option<&Section> {
        self.document.root().section("GlobalHeader")
    }

    /// The parsed section behind a schema, for keys [`OtaSchema`] does not carry. The
    /// section is found by the schema's name, so it is the first section of that name,
    /// the one the projection read.
    pub fn schema_section(&self, schema: &OtaSchema) -> Option<&Section> {
        self.global()?.section(&schema.name)
    }

    /// Whether any schema declares a network type.
    pub fn has_network_schema(&self) -> bool {
        self.schemas
            .iter()
            .any(|schema| network_schema_rank(&schema.kind) != 0)
    }
}

/// The exact type's preference rank (1 to 4), or zero for an unrecognized type. The
/// comparison folds ASCII case only. Whitespace is not removed: `"network 1 "` is not a
/// network schema.
pub fn network_schema_rank(kind: &[u8]) -> u8 {
    match kind.to_ascii_lowercase().as_slice() {
        b"network 1" => 1,
        b"network 2" => 2,
        b"network 3" => 3,
        b"network 4" => 4,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )]

    use super::*;

    const SAMPLE: &[u8] = b"[GlobalHeader]
{
    missionname=Test Valley;
    missiondescription=A small valley.;
    memory=64;
    numplayers=2;
    size=8 x 8;
    [Schema 0]
    {
        Type=Network 1;
        aiprofile=DEFAULT;
        humanmetal=1000;
        humanenergy=1000;
        computermetal=500;
        computerenergy=700;
        surfacemetal=2;
    }
    [schema 1]
    {
        type=network 2;
    }
    [Schema 3]
    {
        type=skipped;
    }
}
";

    #[test]
    fn projects_the_header_and_schemas() {
        let ota = Ota::parse(SAMPLE).unwrap();
        assert_eq!(ota.mission_name, b"Test Valley");
        assert_eq!(ota.mission_description, b"A small valley.");
        assert_eq!(ota.memory, b"64");
        assert_eq!(ota.num_players, b"2");
        assert_eq!(ota.size, b"8 x 8");
        // The projection stops at the first missing index: Schema 2 is absent.
        assert_eq!(ota.schemas.len(), 2);
        let s0 = &ota.schemas[0];
        assert_eq!(s0.name, b"Schema 0");
        assert_eq!(s0.kind, b"Network 1");
        assert_eq!(s0.ai_profile, b"DEFAULT");
        assert_eq!(s0.human_metal, b"1000");
        assert_eq!(s0.human_energy, b"1000");
        assert_eq!(s0.computer_metal, b"500");
        assert_eq!(s0.computer_energy, b"700");
        // Section names fold, and the authored spelling is kept.
        assert_eq!(ota.schemas[1].name, b"schema 1");
        assert_eq!(ota.schemas[1].kind, b"network 2");
        assert_eq!(ota.schemas[1].human_metal, b"");
        assert!(ota.has_network_schema());
        // The raw section stays reachable for other keys.
        let section = ota.schema_section(s0).unwrap();
        assert_eq!(section.int_value("surfacemetal", 0), 2);
        assert!(ota.global().is_some());
        assert_eq!(ota.document().root().sections().count(), 1);
    }

    #[test]
    fn missing_global_header_is_an_error() {
        assert_eq!(
            Ota::parse(b"[Other]\n{\n}\n"),
            Err(OtaError::MissingGlobalHeader)
        );
        assert_eq!(Ota::parse(b""), Err(OtaError::MissingGlobalHeader));
        assert_eq!(
            OtaError::MissingGlobalHeader.to_string(),
            "ota: missing GlobalHeader"
        );
    }

    #[test]
    fn syntax_errors_are_wrapped() {
        let err = Ota::parse(b"[GlobalHeader]\n{\nmissionname=x\n}\n").unwrap_err();
        let OtaError::Parse(inner) = &err else {
            panic!("not a parse error: {err:?}");
        };
        assert!(inner.parse_error().is_some());
        assert!(err.to_string().starts_with("ota: parse: "));
    }

    #[test]
    fn global_header_lookup_is_case_insensitive_and_first_wins() {
        let ota =
            Ota::parse(b"[globalheader]\n{\nsize=a;\n}\n[GlobalHeader]\n{\nsize=b;\n}\n").unwrap();
        assert_eq!(ota.size, b"a");
    }

    #[test]
    fn schema_with_duplicate_names_reads_the_first() {
        let ota = Ota::parse(
            b"[GlobalHeader]\n{\n[Schema 0]\n{\ntype=first;\n}\n[SCHEMA 0]\n{\ntype=second;\n}\n}\n",
        )
        .unwrap();
        assert_eq!(ota.schemas.len(), 1);
        assert_eq!(ota.schemas[0].kind, b"first");
        assert_eq!(ota.schemas[0].name, b"Schema 0");
        let section = ota.schema_section(&ota.schemas[0]).unwrap();
        assert_eq!(section.first_value("type"), Some(b"first".as_slice()));
    }

    #[test]
    fn network_schema_ranks() {
        for (kind, rank) in [
            (&b"network 1"[..], 1),
            (b"NETWORK 2", 2),
            (b"Network 3", 3),
            (b"nEtWoRk 4", 4),
            (b"network 5", 0),
            (b"network 1 ", 0),
            (b" network 1", 0),
            (b"", 0),
            (b"easy", 0),
        ] {
            assert_eq!(network_schema_rank(kind), rank, "{kind:?}");
        }
        let ota = Ota::parse(b"[GlobalHeader]\n{\n[Schema 0]\n{\ntype=Easy;\n}\n}\n").unwrap();
        assert!(!ota.has_network_schema());
    }

    /// Hostile: tens of thousands of schema sections parse in linear time and every
    /// section is found.
    #[test]
    fn many_schemas_are_linear() {
        let mut text = b"[GlobalHeader]\n{\n".to_vec();
        for i in 0..20_000 {
            text.extend_from_slice(format!("[Schema {i}]\n{{\ntype=t{i};\n}}\n").as_bytes());
        }
        text.extend_from_slice(b"}\n");
        let ota = Ota::parse(&text).unwrap();
        assert_eq!(ota.schemas.len(), 20_000);
        assert_eq!(ota.schemas[19_999].kind, b"t19999");
    }

    #[test]
    fn limits_apply() {
        let limits = Limits {
            max_bytes: 8,
            ..Limits::default()
        };
        assert!(matches!(
            Ota::parse_with_limits(SAMPLE, limits),
            Err(OtaError::Parse(tdf::Error::DocumentTooLarge { .. }))
        ));
    }

    #[test]
    fn every_truncation_is_handled_without_panic() {
        for len in 0..SAMPLE.len() {
            let _ = Ota::parse(&SAMPLE[..len]);
        }
    }
}
