//! Red parity over one library export: the CPKs the Team compiler wrote for an upgraded
//! export, against the CPKs Red wrote for the old one, one report line per entry of Red's
//! output (and per entry of ours that no Red entry accounts for), each classified under the
//! tiers and the intentional differences of
//! `docs/spec/team_compiler/testing.md` "Testing: parity against Red (adapted from Blue)":
//! textures matched by content across the relocated locations, Red's kit path magic
//! (`u0XXXp1`) read as the compiler's `kit1`, boots and gloves packages matched by content
//! across the IDs, models decoded and compared with their texture directories normalized,
//! FTEX and DDS compared by what they decode to, the dropped `*.fpk.xml` named as such.
//!
//! A parity test fixes the expected class of every entry; this tool only names each entry's
//! class, so a script can tally the classes over the library and the lead can diagnose each.
//!
//! How an entry of Red's output is paired with ours, in order: the same path, Red's kit
//! magic read as ours; a texture (`.ftex`, `.dds`) by file name among our textures, in the
//! player's own folder when Red's path names his slot (`face/real/73905`), the ones not yet
//! paired first, the first whose content compares equal, else the closest compared (fewest
//! differing blocks under the same headers; `relocated_differs`); a boots or gloves
//! package (`boots/k<id>/`, `glove/g<id>/`) under the ID the optional map names for Red's
//! (the compiler plans its own IDs), else by content among our packages of the same shape,
//! the nearest unpaired one once every equal pair is made; a `*.fpk.xml` is `dropped_fpk_xml`
//! without a lookup; anything else unpaired is `red_only`, and every entry of ours no Red
//! entry pairs with is `ours_only` (`studio_record` for the compiler's own `unlimited-tool-works/`
//! records, which no game reads).
//!
//! How a paired entry is compared, by what it is:
//! - `exact` / `bytes_differ`: the same bytes, else the sizes and the first differing offset
//!   (`.skl`, `.fclo`, `.fpkd`, `face_diff.bin`, `.model`, PNG logos, anything unlisted);
//! - `fmdl_equal` / `fmdl_differs`: both decoded (`fmdl::Model`), every texture's directory
//!   dropped and its name read with the compiler's kit spelling, float attributes compared
//!   bit for bit (a NaN vertex equals itself); the detail names the differing parts, per mesh
//!   the differing attributes and how far positions moved; `fmdl_texture_names_differ` when
//!   the models differ in their texture names alone, read as stems (`hatboots_u0XXXp1` is
//!   `hatboots_kitN.dds`);
//! - `ftex_equal` / `ftex_differs`: the headers (`ftex::info`) and the pixel payload each
//!   converts back to (`ftex_to_dds`); the detail names the differing header fields and the
//!   share and place of the differing 16-byte blocks;
//! - `dds_equal` / `dds_differs` / `dds_reencoded_equal` / `dds_reencoded`: the dimensions,
//!   codec and pixel payload (a WESYS-wrapped file unwrapped); with the codec differing, both
//!   decoded (`dds_convert`) and the mips compared;
//! - `mtl_equal` / `mtl_differs`: the material set (`pes_model`), every sampler path reduced
//!   to its file name in the compiler's kit spelling;
//! - `xml_equal` / `xml_differs`: a `face.xml`'s `<model>` entries (type, path and material
//!   as file names) as sets, and its `<dif>` decoded;
//! - `kitconfig_differs`: a kit config decoded on both sides, the differing TOML lines named;
//! - `bin_records_differ`: `UniColor.bin` and `TeamColor.bin` by team record, the teams
//!   whose records differ named; `uniparam_equal` / `uniparam_differs`:
//!   `UniformParameter.bin` by entry name and bytes;
//! - `fpk_equal` / `fpk_differs`: an FPK by its member names, each member compared by its
//!   kind as above on its own line (`<fpk path>/<member>`; `fpk_member_missing`,
//!   `fpk_member_ours_only`);
//! - `cpk_equal` / `cpk_differs`: a nested CPK (a pre-Fox face) by its member paths, then
//!   file names (`renamed_model` for a model packed under another name with the same
//!   bytes), a texture missing from ours looked for among our outer textures as above
//!   (`relocated`, `relocated_differs`; `cpk_member_missing`, `cpk_member_ours_only`).
//!
//! A paired package or texture found equal is `relocated` or `relocated_id` with the
//! comparison's class as its detail; one found differing is `relocated_differs` or
//! `relocated_id_differs` with the class and detail.
//!
//! Usage: `red_parity <pes version> <folder of Red's CPKs> <folder of our CPKs> <report.tsv>
//! [<ids.tsv>]`; a folder holding no CPK is read as one extracted tree (the committed tracer
//! fixtures); `ids.tsv` maps Red's boots and gloves folder IDs to the compiler's, one
//! `k1859<TAB>k3392` per line. The report's columns: side (`team`/`refs`), Red's path, our
//! path, class, detail.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, bail};
use base64::Engine as _;
use fmdl::Model;
use fpk::FpkFile;
use kit_config::KitConfig;
use pes_model::format::mtl::{MaterialEntry, MaterialSet};
use pes_version::PesVersion;
use uniparam::UniformParameter;

/// The entries of every CPK of one folder, the referee CPK's apart from the team CPKs'.
struct Side {
    team: BTreeMap<String, Vec<u8>>,
    refs: BTreeMap<String, Vec<u8>>,
}

/// One report line.
struct Line {
    side: &'static str,
    red: String,
    ours: String,
    class: &'static str,
    detail: String,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 && args.len() != 6 {
        bail!(
            "usage: red_parity <pes version> <red folder> <ours folder> <report.tsv> [<ids.tsv>]"
        );
    }
    // The boots and gloves IDs the compiler planned for Red's folder IDs (`k1859<TAB>k3392` per
    // line), from the upgrade's inferred wearers: a package is paired by its ID first, and
    // by its content only when the map does not name it.
    let mut ids: BTreeMap<String, String> = BTreeMap::new();
    if let Some(path) = args.get(5) {
        for line in fs::read_to_string(path)
            .with_context(|| path.clone())?
            .lines()
        {
            if let Some((red_id, our_id)) = line.split_once('\t') {
                ids.insert(red_id.to_owned(), our_id.to_owned());
            }
        }
    }
    let version: PesVersion = args[1]
        .parse()
        .map_err(|error| anyhow::anyhow!("{}: {error}", args[1]))?;
    let red = load_side(Path::new(&args[2]))?;
    let ours = load_side(Path::new(&args[3]))?;
    let mut lines = Vec::new();
    compare_side("team", version, &red.team, &ours.team, &ids, &mut lines);
    compare_side("refs", version, &red.refs, &ours.refs, &ids, &mut lines);
    let mut text = String::new();
    for line in &lines {
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            line.side,
            line.red,
            line.ours,
            line.class,
            line.detail.replace(['\t', '\n'], " ")
        ));
    }
    fs::write(&args[4], text).with_context(|| args[4].clone())?;
    Ok(())
}

/// A folder's CPKs, the referee CPK (`referee` in its name) apart; a folder holding no CPK is
/// read as one extracted tree (the committed tracer fixtures' `red/`), the referee tracer's
/// (`refs` in the folder's path) as the referee side.
fn load_side(folder: &Path) -> anyhow::Result<Side> {
    let mut side = Side {
        team: BTreeMap::new(),
        refs: BTreeMap::new(),
    };
    let mut saw_cpk = false;
    for entry in fs::read_dir(folder).with_context(|| folder.display().to_string())? {
        let path = entry?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.to_ascii_lowercase().ends_with(".cpk") {
            continue;
        }
        saw_cpk = true;
        let into = if name.to_ascii_lowercase().contains("referee") {
            &mut side.refs
        } else {
            &mut side.team
        };
        let file = fs::File::open(&path).with_context(|| path.display().to_string())?;
        let mut cpk = cpk::CpkArchive::open(file).with_context(|| path.display().to_string())?;
        let entries = cpk.entries().to_vec();
        for entry in &entries {
            let bytes = cpk.read(entry).with_context(|| entry.path.clone())?;
            into.insert(entry.path.clone(), bytes);
        }
    }
    if !saw_cpk {
        let into = if folder.to_string_lossy().contains("refs") {
            &mut side.refs
        } else {
            &mut side.team
        };
        read_tree(folder, folder, into)?;
    }
    Ok(side)
}

fn read_tree(root: &Path, dir: &Path, into: &mut BTreeMap<String, Vec<u8>>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let path = entry?.path();
        if path.is_dir() {
            read_tree(root, &path, into)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        into.insert(
            relative,
            fs::read(&path).with_context(|| path.display().to_string())?,
        );
    }
    Ok(())
}

/// The compiler's spelling of Red's kit path magic: `u0XXXp1` or `u0792p1` to `kit1`,
/// `...p0` to `kitN`, wherever it sits in `text`.
fn normalize_kit_magic(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 7 <= bytes.len()
            && &bytes[i..i + 2] == b"u0"
            && (&bytes[i + 2..i + 5] == b"XXX"
                || bytes[i + 2..i + 5].iter().all(u8::is_ascii_digit))
            && bytes[i + 5] == b'p'
            && bytes[i + 6].is_ascii_digit()
        {
            let digit = bytes[i + 6] as char;
            out.push_str("kit");
            out.push(if digit == '0' { 'N' } else { digit });
            i += 7;
            continue;
        }
        out.push(text[i..].chars().next().unwrap_or(' '));
        i += text[i..].chars().next().map_or(1, char::len_utf8);
    }
    out
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn lower_name(path: &str) -> String {
    normalize_kit_magic(file_name(path)).to_ascii_lowercase()
}

fn is_texture(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".ftex") || lower.ends_with(".dds")
}

/// `boots/k2180/#Win/boots.fpk` without its ID: `boots/k*/#Win/boots.fpk`; the folder of a
/// boots or gloves package is named by its ID, which the compiler plans anew.
fn without_id(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = path.split('/').collect();
    let kind = parts
        .iter()
        .position(|part| *part == "boots" || *part == "glove")?;
    let id = parts.get(kind + 1)?;
    let letter = id.chars().next()?;
    if !(letter == 'k' || letter == 'g') || !id[1..].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    parts[kind + 1] = "*";
    Some(parts.join("/"))
}

fn compare_side(
    side: &'static str,
    version: PesVersion,
    red: &BTreeMap<String, Vec<u8>>,
    ours: &BTreeMap<String, Vec<u8>>,
    ids: &BTreeMap<String, String>,
    lines: &mut Vec<Line>,
) {
    // Our paths as Red would spell them, for the same-path lookup.
    let mut by_normalized: BTreeMap<String, &String> = BTreeMap::new();
    let mut textures_by_name: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    let mut packages_by_shape: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for path in ours.keys() {
        by_normalized.insert(normalize_kit_magic(path), path);
        if is_texture(path) {
            textures_by_name
                .entry(lower_name(path))
                .or_default()
                .push(path);
        }
        if let Some(shape) = without_id(path) {
            packages_by_shape.entry(shape).or_default().push(path);
        }
    }
    let mut consumed: BTreeSet<&String> = BTreeSet::new();
    let mut deferred: Vec<(&String, &Vec<u8>, String)> = Vec::new();
    for (red_path, red_bytes) in red {
        if red_path.to_ascii_lowercase().ends_with(".fpk.xml") {
            lines.push(make_line(
                side,
                red_path,
                "",
                "dropped_fpk_xml",
                String::new(),
            ));
            continue;
        }
        // Same path, the kit magic read either way.
        if let Some(our_path) = by_normalized.get(&normalize_kit_magic(red_path)) {
            consumed.insert(our_path);
            let (class, detail) = compare_entry(
                version,
                red_path,
                red_bytes,
                &ours[*our_path],
                ours,
                lines,
                side,
            );
            lines.push(make_line(side, red_path, our_path, class, detail));
            continue;
        }
        // A texture relocated to a per-player common folder: matched by name and content,
        // and in the player's own folder when the path names his slot (names repeat across
        // players: `hair_parts_col`, `shirt`).
        if is_texture(red_path) {
            let slot = slot_of(red_path);
            let mut candidates: Vec<_> = textures_by_name
                .get(&lower_name(red_path))
                .into_iter()
                .flatten()
                .filter(|path| slot.as_ref().is_none_or(|slot| in_slot(path, slot)))
                .collect();
            // Unpaired ones first: equal copies of one texture (one per package) pair one to
            // one, rather than every Red copy with our first.
            candidates.sort_by_key(|path| consumed.contains(*path));
            let mut found = None;
            let mut nearest = None;
            for candidate in candidates {
                let (class, detail) = compare_entry(
                    version,
                    red_path,
                    red_bytes,
                    &ours[*candidate],
                    ours,
                    lines,
                    side,
                );
                if class.ends_with("_equal") || class == "exact" {
                    found = Some((*candidate, class));
                    break;
                }
                if is_closer(&detail, nearest.as_ref().map(|(_, _, best)| best)) {
                    nearest = Some((*candidate, class, detail));
                }
            }
            match (found, nearest) {
                (Some((our_path, class)), _) => {
                    consumed.insert(our_path);
                    lines.push(make_line(
                        side,
                        red_path,
                        our_path,
                        "relocated",
                        class.to_owned(),
                    ));
                }
                (None, Some((our_path, class, detail))) => {
                    consumed.insert(our_path);
                    lines.push(make_line(
                        side,
                        red_path,
                        our_path,
                        "relocated_differs",
                        format!("{class}: {detail}"),
                    ));
                }
                (None, None) => {
                    lines.push(make_line(side, red_path, "", "red_only", String::new()))
                }
            }
            continue;
        }
        // A boots or gloves package under the ID the compiler planned for its folder's.
        if let Some((red_id, our_id)) = ids
            .iter()
            .find(|(red_id, _)| red_path.contains(&format!("/{red_id}/")))
        {
            let our_path = red_path.replace(&format!("/{red_id}/"), &format!("/{our_id}/"));
            if let Some(our_bytes) = ours.get(&our_path) {
                let (class, detail) =
                    compare_entry(version, red_path, red_bytes, our_bytes, ours, lines, side);
                let equal = class.ends_with("_equal") || class == "exact";
                consumed.insert(
                    ours.get_key_value(&our_path)
                        .map(|(key, _)| key)
                        .unwrap_or(red_path),
                );
                lines.push(make_line(
                    side,
                    red_path,
                    &our_path,
                    if equal {
                        "relocated_id"
                    } else {
                        "relocated_id_differs"
                    },
                    if equal {
                        class.to_owned()
                    } else {
                        format!("{class}: {detail}")
                    },
                ));
                continue;
            }
        }
        // A boots or gloves package under another ID: matched by shape and content.
        if let Some(shape) = without_id(red_path) {
            let candidates: Vec<&String> = packages_by_shape
                .get(&shape)
                .map(|paths| {
                    paths
                        .iter()
                        .copied()
                        .filter(|path| !consumed.contains(path))
                        .collect()
                })
                .unwrap_or_default();
            let mut found = None;
            for candidate in candidates {
                let mut probe = Vec::new();
                let (class, _) = compare_entry(
                    version,
                    red_path,
                    red_bytes,
                    &ours[candidate],
                    ours,
                    &mut probe,
                    side,
                );
                if class.ends_with("_equal") || class == "exact" {
                    found = Some((candidate, class, probe));
                    break;
                }
            }
            match found {
                Some((our_path, class, probe)) => {
                    consumed.insert(our_path);
                    lines.extend(probe);
                    lines.push(make_line(
                        side,
                        red_path,
                        our_path,
                        "relocated_id",
                        class.to_owned(),
                    ));
                }
                // Paired with the nearest of the packages left once every equal pair is
                // made, so a package differing in one model is not taken by another
                // player's boots first.
                None => deferred.push((red_path, red_bytes, shape)),
            }
            continue;
        }
        lines.push(make_line(side, red_path, "", "red_only", String::new()));
    }
    for (red_path, red_bytes, shape) in deferred {
        let candidates: Vec<&String> = packages_by_shape
            .get(&shape)
            .map(|paths| {
                paths
                    .iter()
                    .copied()
                    .filter(|path| !consumed.contains(path))
                    .collect()
            })
            .unwrap_or_default();
        let mut nearest = None;
        for candidate in candidates {
            let mut probe = Vec::new();
            let (class, detail) = compare_entry(
                version,
                red_path,
                red_bytes,
                &ours[candidate],
                ours,
                &mut probe,
                side,
            );
            // The nearest candidate has the fewest differing members; a model differing in
            // its meshes alone (the same bones and materials) is the same boots re-encoded,
            // a tenth of a differing member.
            let score: f64 = probe
                .iter()
                .filter(|line| !(line.class.ends_with("_equal") || line.class == "exact"))
                .map(|line| {
                    if line.class == "fmdl_differs" && line.detail.starts_with("meshes") {
                        0.1
                    } else {
                        1.0
                    }
                })
                .sum();
            let nearer = nearest
                .as_ref()
                .is_none_or(|(_, _, _, _, best): &(_, _, _, _, f64)| score < *best);
            if nearer {
                nearest = Some((candidate, class, detail, probe, score));
            }
        }
        match nearest {
            Some((our_path, class, detail, probe, _)) => {
                consumed.insert(our_path);
                lines.extend(probe);
                lines.push(make_line(
                    side,
                    red_path,
                    our_path,
                    "relocated_id_differs",
                    format!("{class}: {detail}"),
                ));
            }
            None => lines.push(make_line(side, red_path, "", "red_only", String::new())),
        }
    }
    // An outer texture a nested CPK's member relocated to is accounted for by that member's
    // line, which names it as ours.
    let named: BTreeSet<String> = lines
        .iter()
        .filter(|line| line.side == side)
        .map(|line| line.ours.clone())
        .collect();
    for path in ours.keys() {
        if consumed.contains(path) || named.contains(path) {
            continue;
        }
        let class = if path.starts_with("unlimited-tool-works/") {
            "studio_record"
        } else {
            "ours_only"
        };
        lines.push(Line {
            side,
            red: String::new(),
            ours: path.clone(),
            class,
            detail: String::new(),
        });
    }
}

fn make_line(
    side: &'static str,
    red: &str,
    ours: &str,
    class: &'static str,
    detail: String,
) -> Line {
    Line {
        side,
        red: red.to_owned(),
        ours: ours.to_owned(),
        class,
        detail,
    }
}

/// One Red entry against our entry at a matched path: the class and a detail. A container
/// (an FPK, a nested CPK) also pushes one line per member.
fn compare_entry(
    version: PesVersion,
    red_path: &str,
    red: &[u8],
    ours: &[u8],
    our_outer: &BTreeMap<String, Vec<u8>>,
    lines: &mut Vec<Line>,
    side: &'static str,
) -> (&'static str, String) {
    let lower = red_path.to_ascii_lowercase();
    let name = file_name(&lower);
    if lower.ends_with(".fpk") {
        return compare_fpk(version, red_path, red, ours, our_outer, lines, side);
    }
    if lower.ends_with(".cpk") {
        return compare_nested_cpk(version, red_path, red, ours, our_outer, lines, side);
    }
    if lower.ends_with(".fmdl") {
        return compare_fmdl(red, ours);
    }
    if lower.ends_with(".ftex") {
        return compare_ftex(red, ours);
    }
    if lower.ends_with(".dds") {
        return compare_dds(red, ours);
    }
    if lower.ends_with(".mtl") {
        return compare_mtl(red, ours);
    }
    if lower.ends_with(".xml") {
        return compare_face_xml(red, ours);
    }
    if name == "unicolor.bin" {
        return compare_records(red, ours, 85, 4);
    }
    if name == "teamcolor.bin" {
        return compare_records(red, ours, 16, 2);
    }
    if name == "uniformparameter.bin" {
        return compare_uniform_parameter(red, ours);
    }
    if lower.ends_with(".bin") && lower.contains("/uniform/team/") {
        return compare_kit_config(version, red, ours);
    }
    compare_bytes(red, ours)
}

fn compare_bytes(red: &[u8], ours: &[u8]) -> (&'static str, String) {
    if red == ours {
        return ("exact", String::new());
    }
    let offset = red
        .iter()
        .zip(ours)
        .position(|(a, b)| a != b)
        .unwrap_or(red.len().min(ours.len()));
    (
        "bytes_differ",
        format!(
            "red {} bytes, ours {}, first difference at {offset}",
            red.len(),
            ours.len()
        ),
    )
}

/// The model decoded, every texture's directory dropped and its name read with the compiler's
/// kit spelling: the directories are the relocation the tiers allow, checked by the texture
/// rows instead.
fn decoded_model(bytes: &[u8]) -> Result<Model, String> {
    let file = fmdl::FmdlFile::read(bytes).map_err(|error| error.to_string())?;
    let mut model = Model::from_file(&file).map_err(|error| error.to_string())?;
    for material in &mut model.materials {
        for (_, texture) in &mut material.textures {
            texture.directory = String::new();
            texture.file_name = normalize_kit_magic(&texture.file_name);
        }
    }
    Ok(model)
}

fn compare_fmdl(red: &[u8], ours: &[u8]) -> (&'static str, String) {
    let (red_model, our_model) = match (decoded_model(red), decoded_model(ours)) {
        (Ok(red_model), Ok(our_model)) => (red_model, our_model),
        (red_result, our_result) => {
            return (
                "fmdl_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    if red_model == our_model {
        return ("fmdl_equal", String::new());
    }
    let mut parts = Vec::new();
    if red_model.bones != our_model.bones {
        parts.push(format!(
            "bones {} vs {}",
            red_model.bones.len(),
            our_model.bones.len()
        ));
    }
    let textures = |model: &Model| -> Vec<String> {
        model
            .materials
            .iter()
            .flat_map(|m| m.textures.iter().map(|(_, t)| t.file_name.clone()))
            .collect()
    };
    let mut texture_names_only = false;
    if red_model.materials != our_model.materials {
        let red_names: Vec<&str> = red_model
            .materials
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        let our_names: Vec<&str> = our_model
            .materials
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        // The same materials once each texture name is read as its stem in the `kitN`
        // spelling: Red keeps the member's `hatboots_u0XXXp1`, no extension and the kit
        // number baked in; the compiler writes `hatboots_kitN.dds`.
        let mut red_stems = red_model.clone();
        let mut our_stems = our_model.clone();
        for model in [&mut red_stems, &mut our_stems] {
            for material in &mut model.materials {
                for (_, texture) in &mut material.textures {
                    texture.file_name = texture_stem(&texture.file_name);
                }
            }
        }
        texture_names_only = red_stems.materials == our_stems.materials;
        parts.push(format!(
            "materials {red_names:?} vs {our_names:?}, textures {:?} vs {:?}",
            textures(&red_model),
            textures(&our_model)
        ));
    }
    if red_model.meshes != our_model.meshes {
        let mut fields = Vec::new();
        for (index, (red_mesh, our_mesh)) in
            red_model.meshes.iter().zip(&our_model.meshes).enumerate()
        {
            if red_mesh == our_mesh {
                continue;
            }
            let (red_vertices, our_vertices) = (&red_mesh.vertices, &our_mesh.vertices);
            let differing: Vec<&str> = [
                (
                    "positions",
                    !same_floats(&red_vertices.positions, &our_vertices.positions),
                ),
                (
                    "normals",
                    !same_optional_floats(&red_vertices.normals, &our_vertices.normals),
                ),
                (
                    "tangents",
                    !same_optional_floats(&red_vertices.tangents, &our_vertices.tangents),
                ),
                ("colors", red_vertices.colors != our_vertices.colors),
                ("uvs", red_vertices.uvs != our_vertices.uvs),
                (
                    "uv_high_precision",
                    red_vertices.uv_high_precision != our_vertices.uv_high_precision,
                ),
                (
                    "bone_weights",
                    red_vertices.bone_weights != our_vertices.bone_weights,
                ),
                (
                    "bone_indices",
                    red_vertices.bone_indices != our_vertices.bone_indices,
                ),
                ("faces", red_mesh.faces != our_mesh.faces),
                ("bone_group", red_mesh.bone_group != our_mesh.bone_group),
                ("material", red_mesh.material != our_mesh.material),
                ("alpha_flags", red_mesh.alpha_flags != our_mesh.alpha_flags),
                (
                    "shadow_flags",
                    red_mesh.shadow_flags != our_mesh.shadow_flags,
                ),
                (
                    "antiblur flags",
                    (red_mesh.has_antiblur_meshes, red_mesh.is_antiblur_mesh)
                        != (our_mesh.has_antiblur_meshes, our_mesh.is_antiblur_mesh),
                ),
                (
                    "custom_bounding_box",
                    red_mesh.custom_bounding_box != our_mesh.custom_bounding_box,
                ),
            ]
            .into_iter()
            .filter(|(_, differs)| *differs)
            .map(|(field, _)| field)
            .collect();
            if differing.is_empty() {
                continue;
            }
            let mut summary = differing.join("+");
            if red_vertices.positions != our_vertices.positions
                && red_vertices.positions.len() == our_vertices.positions.len()
            {
                let moved = red_vertices
                    .positions
                    .iter()
                    .zip(&our_vertices.positions)
                    .filter(|(a, b)| a != b)
                    .count();
                let largest = red_vertices
                    .positions
                    .iter()
                    .zip(&our_vertices.positions)
                    .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x - y).abs()))
                    .fold(0.0_f32, f32::max);
                summary.push_str(&format!(
                    " ({moved} of {} vertices moved, at most {largest})",
                    red_vertices.positions.len()
                ));
            }
            fields.push(format!("#{index} {summary}"));
            if fields.len() == 3 {
                break;
            }
        }
        if red_model.meshes.len() != our_model.meshes.len() {
            fields.push("count".to_owned());
        }
        if !fields.is_empty() {
            parts.push(format!(
                "meshes {} vs {}, differing: {}",
                red_model.meshes.len(),
                our_model.meshes.len(),
                fields.join(", ")
            ));
        }
    }
    if texture_names_only && parts.len() == 1 {
        return ("fmdl_texture_names_differ", parts.remove(0));
    }
    if red_model.mesh_groups != our_model.mesh_groups {
        parts.push("mesh groups".to_owned());
    }
    if red_model.extensions != our_model.extensions {
        parts.push("extensions".to_owned());
    }
    if red_model.bone_matrices != our_model.bone_matrices {
        parts.push("bone matrices".to_owned());
    }
    if parts.is_empty() {
        // `Model`'s equality compares floats as values, so a NaN vertex (a few community
        // models carry them) never equals itself; compared bit for bit above, it does.
        return ("fmdl_equal", "equal bit for bit (NaN vertices)".to_owned());
    }
    ("fmdl_differs", parts.join("; "))
}

/// Float attributes compared bit for bit, so a NaN equals the same NaN.
fn same_floats<const N: usize>(red: &[[f32; N]], ours: &[[f32; N]]) -> bool {
    red.len() == ours.len()
        && red
            .iter()
            .zip(ours)
            .all(|(a, b)| a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits()))
}

fn same_optional_floats<const N: usize>(
    red: &Option<Vec<[f32; N]>>,
    ours: &Option<Vec<[f32; N]>>,
) -> bool {
    match (red, ours) {
        (None, None) => true,
        (Some(red), Some(ours)) => same_floats(red, ours),
        _ => false,
    }
}

fn compare_ftex(red: &[u8], ours: &[u8]) -> (&'static str, String) {
    let (red_info, our_info) = match (ftex::info(red), ftex::info(ours)) {
        (Ok(red_info), Ok(our_info)) => (red_info, our_info),
        (red_result, our_result) => {
            return (
                "ftex_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    let decoded = |bytes: &[u8]| {
        ftex::ftex_to_dds(bytes)
            .map_err(|error| error.to_string())
            .and_then(|dds| dds_content(&dds))
    };
    let (red_content, our_content) = match (decoded(red), decoded(ours)) {
        (Ok(red_content), Ok(our_content)) => (red_content, our_content),
        (red_result, our_result) => {
            return (
                "ftex_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    let pixels = if red_content.3 == our_content.3 {
        "pixels equal".to_owned()
    } else {
        format!(
            "pixels differ in {}",
            differing_blocks(&red_content.3, &our_content.3)
        )
    };
    if red_info != our_info {
        return (
            "ftex_differs",
            format!("headers red {red_info:?}, ours {our_info:?}; {pixels}"),
        );
    }
    if red_content.3 == our_content.3 {
        return ("ftex_equal", String::new());
    }
    ("ftex_differs", format!("same headers, {pixels}"))
}

/// Whether a differing texture comparison's `detail` came closer than `best`'s (or there is
/// none yet): fewer differing blocks, "pixels equal" closest of all. A detail with differing
/// headers is the farthest, its block count meaningless (two payloads of one size but other
/// dimensions or codecs). The candidates share a file name, so the first one compared is
/// often another player's copy: without this the line would name it rather than the
/// wearer's own, as it did for a texture Red changed in its last two blocks.
fn is_closer(detail: &str, best: Option<&String>) -> bool {
    let distance = |detail: &str| -> Option<usize> {
        if detail.ends_with("pixels equal") {
            return Some(0);
        }
        let rest = detail.strip_prefix("same headers, ")?;
        let (_, rest) = rest.split_once(" blocks (")?;
        let (count, _) = rest.split_once(" blocks,")?;
        count.parse().ok()
    };
    match (best.map(|best| distance(best)), distance(detail)) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(None), Some(_)) => true,
        (Some(Some(best)), Some(new)) => new < best,
    }
}

/// The share of 16-byte blocks (one BC2/BC3/BC7 block, or four BC1 blocks) that differ
/// between two pixel payloads, or their sizes when they differ.
fn differing_blocks(red: &[u8], ours: &[u8]) -> String {
    if red.len() != ours.len() {
        return format!("size: {} vs {} bytes", red.len(), ours.len());
    }
    let blocks = red.len().div_ceil(16);
    let differing: Vec<usize> = red
        .chunks(16)
        .zip(ours.chunks(16))
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    format!(
        "{:.1}% of {blocks} blocks ({} blocks, first #{} at byte {}, last #{})",
        100.0 * differing.len() as f64 / blocks.max(1) as f64,
        differing.len(),
        differing.first().copied().unwrap_or(0),
        differing.first().copied().unwrap_or(0) * 16,
        differing.last().copied().unwrap_or(0)
    )
}

/// A texture name as its stem in the compiler's kit spelling: the extension dropped, Red's
/// `u0XXXp1` read as `kit1` and every `kit<digit>` as `kitN`, the kit-dependent reference
/// the compiler writes (`spec/model_format.md` "Kit-dependent assets").
fn texture_stem(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let stem = normalize_kit_magic(stem);
    let mut out = String::with_capacity(stem.len());
    let bytes = stem.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 4 <= bytes.len() && &bytes[i..i + 3] == b"kit" && bytes[i + 3].is_ascii_digit() {
            out.push_str("kitN");
            i += 4;
            continue;
        }
        out.push(stem[i..].chars().next().unwrap_or(' '));
        i += stem[i..].chars().next().map_or(1, char::len_utf8);
    }
    out
}

/// A DDS's width, height, four-character code and pixel payload, a WESYS-wrapped file
/// unwrapped first.
fn dds_content(bytes: &[u8]) -> Result<(u32, u32, [u8; 4], Vec<u8>), String> {
    let bytes = wezlib::decompress_if_wrapped(bytes).map_err(|error| error.to_string())?;
    if bytes.len() < 128 || &bytes[..4] != b"DDS " {
        return Err("not a DDS".to_owned());
    }
    let field = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]));
    let fourcc = bytes[84..88].try_into().unwrap_or([0; 4]);
    Ok((field(16), field(12), fourcc, bytes[128..].to_vec()))
}

fn compare_dds(red: &[u8], ours: &[u8]) -> (&'static str, String) {
    if red == ours {
        return ("exact", String::new());
    }
    let (red_content, our_content) = match (dds_content(red), dds_content(ours)) {
        (Ok(red_content), Ok(our_content)) => (red_content, our_content),
        (red_result, our_result) => {
            return (
                "dds_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    if red_content == our_content {
        return ("dds_equal", "header differs, pixels equal".to_owned());
    }
    let (red_width, red_height, red_codec, _) = &red_content;
    let (our_width, our_height, our_codec, _) = &our_content;
    if (red_width, red_height) != (our_width, our_height) {
        return (
            "dds_differs",
            format!("{red_width}x{red_height} vs {our_width}x{our_height}"),
        );
    }
    let codec = |code: &[u8; 4]| String::from_utf8_lossy(code).to_string();
    if red_codec != our_codec {
        let decoded = |bytes: &[u8]| {
            dds_convert::decode(bytes, dds_convert::SourceFormat::Dds).map(|image| image.mips)
        };
        return match (decoded(red), decoded(ours)) {
            (Ok(red_mips), Ok(our_mips)) if red_mips == our_mips => (
                "dds_reencoded_equal",
                format!("codec {} vs {}", codec(red_codec), codec(our_codec)),
            ),
            (Ok(red_mips), Ok(our_mips)) => (
                "dds_reencoded",
                format!(
                    "codec {} vs {}, {} vs {} mips, pixels differ",
                    codec(red_codec),
                    codec(our_codec),
                    red_mips.len(),
                    our_mips.len()
                ),
            ),
            (red_result, our_result) => (
                "dds_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            ),
        };
    }
    (
        "dds_differs",
        format!(
            "same {}x{} {}, pixels differ in {}",
            red_width,
            red_height,
            codec(red_codec),
            differing_blocks(&red_content.3, &our_content.3)
        ),
    )
}

/// The two-digit slot of a player's face path (`face/real/73905.cpk`, `face/real/79205/`),
/// `None` for a referee (`referee005`) or any other path.
fn slot_of(path: &str) -> Option<String> {
    let after = path.split("face/real/").nth(1)?;
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 3 {
        return None;
    }
    Some(digits[digits.len() - 2..].to_owned())
}

/// Whether `path` sits in slot `slot`'s folder, `<NN - name>/` or `<NN>/`, the per-player
/// common folder the compiler relocates his textures to.
fn in_slot(path: &str, slot: &str) -> bool {
    path.contains(&format!("/{slot} - ")) || path.contains(&format!("/{slot}/"))
}

/// The material set, every sampler path reduced to its file name in the compiler's kit
/// spelling: the paths are the relocation the tiers allow.
fn materials_by_name(bytes: &[u8]) -> Result<MaterialSet, String> {
    let mut set = MaterialSet::read(bytes).map_err(|error| error.to_string())?;
    for material in &mut set.materials {
        for entry in &mut material.entries {
            if let MaterialEntry::Sampler(sampler) = entry {
                sampler.path = lower_name(&sampler.path);
            }
        }
    }
    Ok(set)
}

fn compare_mtl(red: &[u8], ours: &[u8]) -> (&'static str, String) {
    if red == ours {
        return ("exact", String::new());
    }
    let (red_set, our_set) = match (materials_by_name(red), materials_by_name(ours)) {
        (Ok(red_set), Ok(our_set)) => (red_set, our_set),
        (red_result, our_result) => {
            return (
                "mtl_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    if red_set.materials == our_set.materials {
        return ("mtl_equal", String::new());
    }
    let names = |set: &MaterialSet| -> Vec<String> {
        set.materials.iter().map(|m| m.name.clone()).collect()
    };
    let differing: Vec<String> = red_set
        .materials
        .iter()
        .filter(|red_material| !our_set.materials.contains(red_material))
        .map(|material| format!("{}: {:?}", material.name, material.entries))
        .collect();
    (
        "mtl_differs",
        format!(
            "materials {:?} vs {:?}; red's unmatched: {}",
            names(&red_set),
            names(&our_set),
            differing.join(" | ")
        ),
    )
}

/// A `face.xml` read for comparison: its `<model>` entries (type, path, material, the
/// paths reduced to file names) and its `<dif>` decoded.
struct FaceXml {
    entries: Vec<(String, String, String)>,
    dif: Vec<u8>,
}

fn face_xml(bytes: &[u8]) -> Result<FaceXml, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let document = roxmltree::Document::parse(text).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    let mut dif = Vec::new();
    for node in document
        .root_element()
        .children()
        .filter(|node| node.is_element())
    {
        match node.tag_name().name() {
            "model" => entries.push((
                node.attribute("type").unwrap_or("").to_owned(),
                lower_name(node.attribute("path").unwrap_or("")),
                lower_name(node.attribute("material").unwrap_or("")),
            )),
            "dif" => {
                let encoded: String = node
                    .text()
                    .unwrap_or("")
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                dif = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|error| error.to_string())?;
            }
            _ => {}
        }
    }
    Ok(FaceXml { entries, dif })
}

fn compare_face_xml(red: &[u8], ours: &[u8]) -> (&'static str, String) {
    if red == ours {
        return ("exact", String::new());
    }
    let (red_xml, our_xml) = match (face_xml(red), face_xml(ours)) {
        (Ok(red_xml), Ok(our_xml)) => (red_xml, our_xml),
        (red_result, our_result) => {
            return (
                "xml_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    let mut red_entries = red_xml.entries.clone();
    let mut our_entries = our_xml.entries.clone();
    red_entries.sort();
    our_entries.sort();
    if red_entries == our_entries && red_xml.dif == our_xml.dif {
        return ("xml_equal", String::new());
    }
    let mut parts = Vec::new();
    if red_entries != our_entries {
        parts.push(format!("entries {red_entries:?} vs {our_entries:?}"));
    }
    if red_xml.dif != our_xml.dif {
        parts.push(format!(
            "dif {} vs {} bytes",
            red_xml.dif.len(),
            our_xml.dif.len()
        ));
    }
    ("xml_differs", parts.join("; "))
}

/// Fixed-size team records: the records that differ, named by the team ID each starts with
/// (`id_bytes` wide).
fn compare_records(
    red: &[u8],
    ours: &[u8],
    size: usize,
    id_bytes: usize,
) -> (&'static str, String) {
    if red == ours {
        return ("exact", String::new());
    }
    if red.len() != ours.len() || !red.len().is_multiple_of(size) {
        return (
            "bin_differs",
            format!("red {} bytes, ours {}", red.len(), ours.len()),
        );
    }
    let mut teams = Vec::new();
    for (red_record, our_record) in red.chunks(size).zip(ours.chunks(size)) {
        if red_record != our_record {
            let mut id = 0u32;
            for (i, byte) in red_record[..id_bytes].iter().enumerate() {
                id |= u32::from(*byte) << (8 * i);
            }
            teams.push(id);
        }
    }
    ("bin_records_differ", format!("teams {teams:?}"))
}

fn compare_uniform_parameter(red: &[u8], ours: &[u8]) -> (&'static str, String) {
    if red == ours {
        return ("exact", String::new());
    }
    let (red_table, our_table) = match (UniformParameter::read(red), UniformParameter::read(ours)) {
        (Ok(red_table), Ok(our_table)) => (red_table, our_table),
        (red_result, our_result) => {
            return (
                "uniparam_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    let red_names: BTreeSet<&str> = red_table.entries().map(|(name, _)| name).collect();
    let our_names: BTreeSet<&str> = our_table.entries().map(|(name, _)| name).collect();
    let mut parts = Vec::new();
    let missing: Vec<&&str> = red_names.difference(&our_names).collect();
    let extra: Vec<&&str> = our_names.difference(&red_names).collect();
    if !missing.is_empty() {
        parts.push(format!("missing in ours {missing:?}"));
    }
    if !extra.is_empty() {
        parts.push(format!("extra in ours {extra:?}"));
    }
    let differing: Vec<&&str> = red_names
        .intersection(&our_names)
        .filter(|name| red_table.get(name) != our_table.get(name))
        .collect();
    if !differing.is_empty() {
        parts.push(format!("entries differ {differing:?}"));
    }
    if parts.is_empty() {
        return (
            "uniparam_equal",
            "same entries, container bytes differ".to_owned(),
        );
    }
    ("uniparam_differs", parts.join("; "))
}

/// A kit config decoded on both sides, the differing lines of its TOML rendering named.
fn compare_kit_config(version: PesVersion, red: &[u8], ours: &[u8]) -> (&'static str, String) {
    if red == ours {
        return ("exact", String::new());
    }
    let (red_config, our_config) = match (
        KitConfig::decode(red, version),
        KitConfig::decode(ours, version),
    ) {
        (Ok(red_config), Ok(our_config)) => (red_config, our_config),
        (red_result, our_result) => {
            return (
                "kitconfig_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    let red_text = red_config.to_toml();
    let our_text = our_config.to_toml();
    let red_lines: BTreeSet<&str> = red_text.lines().collect();
    let our_lines: BTreeSet<&str> = our_text.lines().collect();
    let differing: Vec<String> = red_lines
        .symmetric_difference(&our_lines)
        .map(|line| (*line).to_owned())
        .collect();
    if differing.is_empty() {
        return (
            "kitconfig_differs",
            "decoded fields equal, undecoded bytes differ".to_owned(),
        );
    }
    ("kitconfig_differs", differing.join(" | "))
}

fn compare_fpk(
    version: PesVersion,
    red_path: &str,
    red: &[u8],
    ours: &[u8],
    our_outer: &BTreeMap<String, Vec<u8>>,
    lines: &mut Vec<Line>,
    side: &'static str,
) -> (&'static str, String) {
    let (red_fpk, our_fpk) = match (FpkFile::read(red), FpkFile::read(ours)) {
        (Ok(red_fpk), Ok(our_fpk)) => (red_fpk, our_fpk),
        (red_result, our_result) => {
            return (
                "fpk_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    let red_names: BTreeSet<&str> = red_fpk.entries().map(|(name, _)| name).collect();
    let our_names: BTreeSet<&str> = our_fpk.entries().map(|(name, _)| name).collect();
    let mut all_equal = true;
    for (name, red_bytes) in red_fpk.entries() {
        let member = format!("{red_path}/{name}");
        let (class, detail) = match our_fpk.get(name) {
            None => ("fpk_member_missing", String::new()),
            Some(our_bytes) => compare_entry(
                version, &member, red_bytes, our_bytes, our_outer, lines, side,
            ),
        };
        if !(class.ends_with("_equal") || class == "exact") {
            all_equal = false;
        }
        lines.push(Line {
            side,
            red: member,
            ours: String::new(),
            class,
            detail,
        });
    }
    for name in our_names.difference(&red_names) {
        all_equal = false;
        lines.push(Line {
            side,
            red: String::new(),
            ours: format!("{red_path}/{name}"),
            class: "fpk_member_ours_only",
            detail: String::new(),
        });
    }
    if red_names != our_names {
        return (
            "fpk_differs",
            format!("members red {red_names:?}, ours {our_names:?}"),
        );
    }
    if all_equal {
        ("fpk_equal", String::new())
    } else {
        ("fpk_differs", "a member differs".to_owned())
    }
}

/// A nested CPK (pre-Fox: a player's face CPK): entries by path, a texture missing from ours
/// looked for by name and content among our outer textures (the relocation to the player's
/// common folder).
fn compare_nested_cpk(
    version: PesVersion,
    red_path: &str,
    red: &[u8],
    ours: &[u8],
    our_outer: &BTreeMap<String, Vec<u8>>,
    lines: &mut Vec<Line>,
    side: &'static str,
) -> (&'static str, String) {
    let read = |bytes: &[u8]| -> Result<BTreeMap<String, Vec<u8>>, String> {
        let mut cpk = cpk::CpkArchive::open(std::io::Cursor::new(bytes))
            .map_err(|error| error.to_string())?;
        let entries = cpk.entries().to_vec();
        let mut files = BTreeMap::new();
        for entry in &entries {
            files.insert(
                entry.path.clone(),
                cpk.read(entry).map_err(|error| error.to_string())?,
            );
        }
        Ok(files)
    };
    let (red_entries, our_entries) = match (read(red), read(ours)) {
        (Ok(red_entries), Ok(our_entries)) => (red_entries, our_entries),
        (red_result, our_result) => {
            return (
                "cpk_unreadable",
                format!("red {:?}, ours {:?}", red_result.err(), our_result.err()),
            );
        }
    };
    // A member is matched by its path inside the CPK first (`realkit/apc.mtl` is not the
    // root's `apc.mtl`), then by its file name (the face model packs under the name the
    // `face.xml` gives).
    let mut our_by_path: BTreeMap<String, &String> = BTreeMap::new();
    let mut our_by_name: BTreeMap<String, &String> = BTreeMap::new();
    for path in our_entries.keys() {
        our_by_path.insert(lower(path), path);
        our_by_name.entry(lower_name(path)).or_insert(path);
    }
    let mut outer_textures: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for path in our_outer.keys() {
        if is_texture(path) {
            outer_textures
                .entry(lower_name(path))
                .or_default()
                .push(path);
        }
    }
    let slot = slot_of(red_path);
    let mut consumed: BTreeSet<&String> = BTreeSet::new();
    let mut all_equal = true;
    for (path, red_bytes) in &red_entries {
        let member = format!("{red_path}/{}", file_name(path));
        let name = lower_name(path);
        let mut line = Line {
            side,
            red: member.clone(),
            ours: String::new(),
            class: "cpk_member_missing",
            detail: String::new(),
        };
        let matched = our_by_path.get(&lower(path)).or_else(|| {
            our_by_name
                .get(&name)
                .filter(|our_path| !consumed.contains(*our_path))
        });
        if let Some(our_path) = matched {
            consumed.insert(our_path);
            let (class, detail) = compare_entry(
                version,
                &member,
                red_bytes,
                &our_entries[*our_path],
                our_outer,
                lines,
                side,
            );
            line.ours = (*our_path).clone();
            line.class = class;
            line.detail = detail;
        } else if is_texture(path) {
            let mut found = None;
            let mut nearest = None;
            for candidate in outer_textures
                .get(&name)
                .into_iter()
                .flatten()
                .filter(|path| slot.as_ref().is_none_or(|slot| in_slot(path, slot)))
            {
                let (class, detail) = compare_entry(
                    version,
                    &member,
                    red_bytes,
                    &our_outer[*candidate],
                    our_outer,
                    lines,
                    side,
                );
                if class.ends_with("_equal") || class == "exact" {
                    found = Some((*candidate, class));
                    break;
                }
                if is_closer(&detail, nearest.as_ref().map(|(_, _, best)| best)) {
                    nearest = Some((*candidate, class, detail));
                }
            }
            match (found, nearest) {
                (Some((our_path, class)), _) => {
                    line.ours = our_path.clone();
                    line.class = "relocated";
                    line.detail = class.to_owned();
                }
                (None, Some((our_path, class, detail))) => {
                    line.ours = our_path.clone();
                    line.class = "relocated_differs";
                    line.detail = format!("{class}: {detail}");
                }
                (None, None) => {}
            }
        } else if lower(path).ends_with(".model") {
            // The face model packs under the name the member's own `face.xml` lists or the
            // generated `oral_` one: matched by content among the unconsumed models.
            let equal = our_entries.iter().find(|(our_path, our_bytes)| {
                !consumed.contains(our_path)
                    && lower(our_path).ends_with(".model")
                    && *our_bytes == red_bytes
            });
            if let Some((our_path, _)) = equal {
                consumed.insert(our_path);
                line.ours = our_path.clone();
                line.class = "renamed_model";
            }
        }
        if !(line.class.ends_with("_equal")
            || line.class == "exact"
            || line.class == "relocated"
            || line.class == "renamed_model")
        {
            all_equal = false;
        }
        lines.push(line);
    }
    for path in our_entries.keys() {
        if !consumed.contains(path) {
            all_equal = false;
            lines.push(Line {
                side,
                red: String::new(),
                ours: format!("{red_path}/{}", file_name(path)),
                class: "cpk_member_ours_only",
                detail: String::new(),
            });
        }
    }
    if all_equal {
        ("cpk_equal", String::new())
    } else {
        ("cpk_differs", "a member differs".to_owned())
    }
}

fn lower(path: &str) -> String {
    path.to_ascii_lowercase()
}
