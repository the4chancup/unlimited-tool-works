//! The `config.toml` form: a fresh document carries the predefined per-field
//! comments; `update_toml` rewrites values inside an existing document while
//! preserving its comments and formatting.

use toml_edit::{DocumentMut, Item, Table};

use crate::KitConfigError;
use crate::binary::template;
use crate::model::*;

fn rgb_string(rgb: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2)
}

fn parse_rgb(key: &'static str, value: &toml::Value) -> Result<Rgb, KitConfigError> {
    let text = value.as_str().ok_or_else(|| KitConfigError::InvalidValue {
        key,
        value: value.to_string(),
    })?;
    let hex = text.strip_prefix('#').unwrap_or(text);
    let bad = || KitConfigError::InvalidValue {
        key,
        value: text.to_owned(),
    };
    if hex.len() != 6 || !hex.is_ascii() {
        return Err(bad());
    }
    let parse = |pair: &str| u8::from_str_radix(pair, 16);
    Ok(Rgb(
        parse(&hex[0..2]).map_err(|_| bad())?,
        parse(&hex[2..4]).map_err(|_| bad())?,
        parse(&hex[4..6]).map_err(|_| bad())?,
    ))
}

fn name_hex(name: &[u8; 16]) -> String {
    name.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse_name_hex(key: &'static str, value: &toml::Value) -> Result<[u8; 16], KitConfigError> {
    let text = value.as_str().ok_or_else(|| KitConfigError::InvalidValue {
        key,
        value: value.to_string(),
    })?;
    let bad = || KitConfigError::InvalidValue {
        key,
        value: text.to_owned(),
    };
    if text.len() != 32 || !text.is_ascii() {
        return Err(bad());
    }
    let mut name = [0u8; 16];
    for (i, byte) in name.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
    }
    Ok(name)
}

struct Reader<'a> {
    root: &'a toml::Table,
}

impl<'a> Reader<'a> {
    /// A table by name: absent is `Ok(None)`, a table `Ok(Some)`, a value of
    /// any other type an error — a wrong-typed `shirt = 144` is not "no
    /// shirt section".
    fn table(&self, name: &'static str) -> Result<Option<&toml::Table>, KitConfigError> {
        let Some(value) = self.root.get(name) else {
            return Ok(None);
        };
        value
            .as_table()
            .map(Some)
            .ok_or_else(|| KitConfigError::InvalidValue {
                key: name,
                value: value.to_string(),
            })
    }

    /// A table nested inside another (`number.back` style), same contract as
    /// [`Reader::table`].
    fn sub_table<'v>(
        &self,
        key: &'static str,
        table: &'v toml::Table,
        name: &str,
    ) -> Result<Option<&'v toml::Table>, KitConfigError> {
        let Some(value) = table.get(name) else {
            return Ok(None);
        };
        value
            .as_table()
            .map(Some)
            .ok_or_else(|| KitConfigError::InvalidValue {
                key,
                value: value.to_string(),
            })
    }

    fn uint(
        &self,
        key: &'static str,
        table: Option<&toml::Table>,
        name: &str,
        default: u8,
    ) -> Result<u8, KitConfigError> {
        let Some(value) = table.and_then(|t| t.get(name)) else {
            return Ok(default);
        };
        value
            .as_integer()
            .and_then(|v| u8::try_from(v).ok())
            .ok_or_else(|| KitConfigError::InvalidValue {
                key,
                value: value.to_string(),
            })
    }

    fn boolean(
        &self,
        key: &'static str,
        table: Option<&toml::Table>,
        name: &str,
        default: bool,
    ) -> Result<bool, KitConfigError> {
        let Some(value) = table.and_then(|t| t.get(name)) else {
            return Ok(default);
        };
        value.as_bool().ok_or_else(|| KitConfigError::InvalidValue {
            key,
            value: value.to_string(),
        })
    }

    fn text<'v>(
        &self,
        key: &'static str,
        table: Option<&'v toml::Table>,
        name: &str,
    ) -> Result<Option<&'v str>, KitConfigError> {
        let Some(value) = table.and_then(|t| t.get(name)) else {
            return Ok(None);
        };
        value
            .as_str()
            .map(Some)
            .ok_or_else(|| KitConfigError::InvalidValue {
                key,
                value: value.to_string(),
            })
    }
}

/// Builds a config from TOML text: missing sections and keys take the
/// template's values.
pub fn from_toml(text: &str) -> Result<KitConfig, KitConfigError> {
    let root: toml::Table = text.parse().map_err(KitConfigError::Toml)?;
    let mut config = template();
    let reader = Reader { root: &root };

    if let Some(shirt) = reader.table("shirt")? {
        config.shirt.model =
            reader.uint("shirt.model", Some(shirt), "model", config.shirt.model)?;
        config.shirt.collar =
            reader.uint("shirt.collar", Some(shirt), "collar", config.shirt.collar)?;
        config.shirt.winter_collar = reader.uint(
            "shirt.winter_collar",
            Some(shirt),
            "winter_collar",
            config.shirt.winter_collar,
        )?;
        config.shirt.tight =
            reader.boolean("shirt.tight", Some(shirt), "tight", config.shirt.tight)?;
        config.shirt.pattern = reader.uint(
            "shirt.pattern",
            Some(shirt),
            "pattern",
            config.shirt.pattern,
        )?;
        if let Some(value) = shirt.get("long_sleeves") {
            config.shirt.long_sleeves = match value {
                toml::Value::String(text) => match text.as_str() {
                    "normal" => LongSleeves::Normal,
                    "undershirt-only" => LongSleeves::UndershirtOnly,
                    _ => {
                        return Err(KitConfigError::InvalidValue {
                            key: "shirt.long_sleeves",
                            value: text.clone(),
                        });
                    }
                },
                toml::Value::Integer(number) => {
                    match u8::try_from(*number).map_err(|_| KitConfigError::InvalidValue {
                        key: "shirt.long_sleeves",
                        value: number.to_string(),
                    })? {
                        0x3E => LongSleeves::Normal,
                        0xBB => LongSleeves::UndershirtOnly,
                        raw => LongSleeves::Raw(raw),
                    }
                }
                _ => {
                    return Err(KitConfigError::InvalidValue {
                        key: "shirt.long_sleeves",
                        value: value.to_string(),
                    });
                }
            };
        }
        if let Some(value) = shirt.get("short_sleeves") {
            config.shirt.short_sleeves = match value {
                toml::Value::String(text) => match text.as_str() {
                    "normal" => ShortSleeves::Normal,
                    "cut-out" => ShortSleeves::CutOut,
                    _ => {
                        return Err(KitConfigError::InvalidValue {
                            key: "shirt.short_sleeves",
                            value: text.clone(),
                        });
                    }
                },
                toml::Value::Integer(number) => {
                    match u8::try_from(*number).map_err(|_| KitConfigError::InvalidValue {
                        key: "shirt.short_sleeves",
                        value: number.to_string(),
                    })? {
                        1 => ShortSleeves::Normal,
                        2 => ShortSleeves::CutOut,
                        raw => ShortSleeves::Raw(raw),
                    }
                }
                _ => {
                    return Err(KitConfigError::InvalidValue {
                        key: "shirt.short_sleeves",
                        value: value.to_string(),
                    });
                }
            };
        }
    }
    if let Some(shorts) = reader.table("shorts")? {
        config.shorts.model =
            reader.uint("shorts.model", Some(shorts), "model", config.shorts.model)?;
    }
    if let Some(colors) = reader.table("colors")? {
        for (name, rgb) in [
            ("shirt1", &mut config.colors.shirt1),
            ("shirt2", &mut config.colors.shirt2),
            ("undershirt", &mut config.colors.undershirt),
            ("shorts", &mut config.colors.shorts),
            ("socks", &mut config.colors.socks),
        ] {
            if let Some(value) = colors.get(name) {
                *rgb = parse_rgb("colors", value)?;
            }
        }
    }
    if let Some(name) = reader.table("name")? {
        config.name.show = reader.boolean("name.show", Some(name), "show", config.name.show)?;
        if let Some(value) = reader.text("name.shape", Some(name), "shape")? {
            config.name.shape = match value {
                "straight" => NameShape::Straight,
                "light-curve" => NameShape::LightCurve,
                "medium-curve" => NameShape::MediumCurve,
                "extreme-curve" => NameShape::ExtremeCurve,
                _ => {
                    return Err(KitConfigError::InvalidValue {
                        key: "name.shape",
                        value: value.to_owned(),
                    });
                }
            };
        }
        config.name.y = reader.uint("name.y", Some(name), "y", config.name.y)?;
        config.name.size = reader.uint("name.size", Some(name), "size", config.name.size)?;
    }
    if let Some(number) = reader.table("number")? {
        if let Some(back) = reader.sub_table("number.back", number, "back")? {
            config.numbers.back.y =
                reader.uint("number.back.y", Some(back), "y", config.numbers.back.y)?;
            config.numbers.back.size = reader.uint(
                "number.back.size",
                Some(back),
                "size",
                config.numbers.back.size,
            )?;
            config.numbers.back.spacing = reader.uint(
                "number.back.spacing",
                Some(back),
                "spacing",
                config.numbers.back.spacing,
            )?;
        }
        if let Some(chest) = reader.sub_table("number.chest", number, "chest")? {
            config.numbers.chest.x =
                reader.uint("number.chest.x", Some(chest), "x", config.numbers.chest.x)?;
            config.numbers.chest.y =
                reader.uint("number.chest.y", Some(chest), "y", config.numbers.chest.y)?;
            config.numbers.chest.size = reader.uint(
                "number.chest.size",
                Some(chest),
                "size",
                config.numbers.chest.size,
            )?;
        }
        if let Some(shorts) = reader.sub_table("number.shorts", number, "shorts")? {
            if let Some(value) = reader.text("number.shorts.side", Some(shorts), "side")? {
                config.numbers.shorts.side = match value {
                    "left" => Side::Left,
                    "right" => Side::Right,
                    _ => {
                        return Err(KitConfigError::InvalidValue {
                            key: "number.shorts.side",
                            value: value.to_owned(),
                        });
                    }
                };
            }
            config.numbers.shorts.x = reader.uint(
                "number.shorts.x",
                Some(shorts),
                "x",
                config.numbers.shorts.x,
            )?;
            config.numbers.shorts.y = reader.uint(
                "number.shorts.y",
                Some(shorts),
                "y",
                config.numbers.shorts.y,
            )?;
            config.numbers.shorts.size = reader.uint(
                "number.shorts.size",
                Some(shorts),
                "size",
                config.numbers.shorts.size,
            )?;
        }
    }
    if let Some(badge) = reader.table("badge")? {
        for (name, badge_key, x_key, y_key, position) in [
            (
                "right_short",
                "badge.right_short",
                "badge.right_short.x",
                "badge.right_short.y",
                &mut config.badges.right_short,
            ),
            (
                "left_short",
                "badge.left_short",
                "badge.left_short.x",
                "badge.left_short.y",
                &mut config.badges.left_short,
            ),
            (
                "right_long",
                "badge.right_long",
                "badge.right_long.x",
                "badge.right_long.y",
                &mut config.badges.right_long,
            ),
            (
                "left_long",
                "badge.left_long",
                "badge.left_long.x",
                "badge.left_long.y",
                &mut config.badges.left_long,
            ),
        ] {
            if let Some(badge_value) = badge.get(name) {
                if badge_value.as_table().is_none() {
                    return Err(KitConfigError::InvalidValue {
                        key: badge_key,
                        value: badge_value.to_string(),
                    });
                }
                for (coordinate, key, field) in
                    [("x", x_key, &mut position.x), ("y", y_key, &mut position.y)]
                {
                    if let Some(value) = badge_value.get(coordinate) {
                        *field = value
                            .as_integer()
                            .and_then(|v| u8::try_from(v).ok())
                            .ok_or_else(|| KitConfigError::InvalidValue {
                                key,
                                value: value.to_string(),
                            })?;
                    }
                }
            }
        }
    }
    if let Some(unknown) = reader.table("unknown")? {
        let mut seen = std::collections::BTreeSet::new();
        for (key, value) in unknown {
            let bad_key = || KitConfigError::InvalidValue {
                key: "unknown",
                value: key.clone(),
            };
            let offset = key
                .strip_prefix("0x")
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
                .ok_or_else(bad_key)?;
            // Only the codec's unknown-bearing offsets are valid, and two
            // spellings of one offset ("0x1C"/"0x1c") are a duplicate.
            let Some(&(_, mask)) = UNKNOWN_MASKS.iter().find(|(o, _)| *o == offset) else {
                return Err(bad_key());
            };
            if !seen.insert(offset) {
                return Err(bad_key());
            }
            let remainder = value
                .as_integer()
                .and_then(|v| u8::try_from(v).ok())
                .ok_or_else(|| KitConfigError::InvalidValue {
                    key: "unknown",
                    value: value.to_string(),
                })?;
            if remainder & !mask != 0 {
                return Err(KitConfigError::InvalidValue {
                    key: "unknown",
                    value: value.to_string(),
                });
            }
            if remainder == 0 {
                config.unknown.remove(&offset);
            } else {
                config.unknown.insert(offset, remainder);
            }
        }
    }
    if let Some(source) = reader.table("source_texture_names")? {
        let mut names = [[0u8; 16]; 5];
        for (i, field) in TEXTURE_NAME_FIELDS.iter().enumerate() {
            if let Some(value) = source.get(*field) {
                names[i] = parse_name_hex("source_texture_names", value)?;
            }
        }
        config.source_texture_names = Some(names);
    } else {
        // The table is the only carrier of texture names in TOML; without
        // it the config does not inherit the template's.
        config.source_texture_names = None;
    }

    Ok(config)
}

/// The remainder at an offset, zero when absent.
fn remainder(map: &std::collections::BTreeMap<u8, u8>, offset: u8) -> u8 {
    map.get(&offset).copied().unwrap_or(0)
}

fn long_sleeves_repr(value: LongSleeves) -> String {
    match value {
        LongSleeves::Normal => "\"normal\"".to_owned(),
        LongSleeves::UndershirtOnly => "\"undershirt-only\"".to_owned(),
        LongSleeves::Raw(raw) => format!("{raw}"),
    }
}

fn short_sleeves_repr(value: ShortSleeves) -> String {
    match value {
        ShortSleeves::Normal => "\"normal\"".to_owned(),
        ShortSleeves::CutOut => "\"cut-out\"".to_owned(),
        ShortSleeves::Raw(raw) => format!("{raw}"),
    }
}

fn shape_repr(value: NameShape) -> &'static str {
    match value {
        NameShape::Straight => "straight",
        NameShape::LightCurve => "light-curve",
        NameShape::MediumCurve => "medium-curve",
        NameShape::ExtremeCurve => "extreme-curve",
    }
}

/// A fresh document with the predefined per-field comments.
pub fn to_toml(config: &KitConfig) -> String {
    let template = template();
    let c = config;
    let mut out = String::new();
    let mut line = |text: &str| {
        out.push_str(text);
        out.push('\n');
    };

    line("[shirt]");
    line(&format!(
        "model = {}                 # 144 / 160 / 176",
        c.shirt.model
    ));
    line(&format!(
        "collar = {}                # 1-131 (PES 15: 1-101, 16: 1-105, 17-18: 1-116, 19: 1-124, 20: 1-127)",
        c.shirt.collar
    ));
    line(&format!(
        "winter_collar = {}         # 1-131 (PES 15: 1-101, 16: 1-105, 17-18: 1-116, 19: 1-124, 20: 1-127)",
        c.shirt.winter_collar
    ));
    line(&format!(
        "tight = {}               # 144/160 only",
        c.shirt.tight
    ));
    line(&format!(
        "pattern = {}                 # 0-13 (PES 15: 0-11)",
        c.shirt.pattern
    ));
    line(&format!(
        "long_sleeves = {}     # \"normal\" / \"undershirt-only\" (144/160 only)",
        long_sleeves_repr(c.shirt.long_sleeves)
    ));
    line(&format!(
        "short_sleeves = {}    # \"normal\" / \"cut-out\" (144/160 only)",
        short_sleeves_repr(c.shirt.short_sleeves)
    ));
    line("");
    line("[shorts]");
    line(&format!(
        "model = {}                  # 0-17",
        c.shorts.model
    ));
    line("");
    line("[colors]                    # the kit's own colors; the menu's are in colors.txt");
    line(&format!("shirt1 = \"{}\"", rgb_string(c.colors.shirt1)));
    line(&format!("shirt2 = \"{}\"", rgb_string(c.colors.shirt2)));
    line(&format!(
        "undershirt = \"{}\"",
        rgb_string(c.colors.undershirt)
    ));
    line(&format!("shorts = \"{}\"", rgb_string(c.colors.shorts)));
    line(&format!("socks = \"{}\"", rgb_string(c.colors.socks)));
    line("");
    line("[name]");
    line(&format!("show = {}", c.name.show));
    line(&format!(
        "shape = \"{}\"          # \"straight\" / \"light-curve\" / \"medium-curve\" / \"extreme-curve\"",
        shape_repr(c.name.shape)
    ));
    line(&format!(
        "y = {}                       # 0-39 (PES 15-20: 0-33)",
        c.name.y
    ));
    line(&format!("size = {}                   # 0-20", c.name.size));
    line("");
    line("[number.back]");
    line(&format!(
        "y = {}                      # 0-29",
        c.numbers.back.y
    ));
    line(&format!(
        "size = {}                   # 0-13",
        c.numbers.back.size
    ));
    line(&format!(
        "spacing = {}                 # 0-2",
        c.numbers.back.spacing
    ));
    line("");
    line("[number.chest]");
    line(&format!(
        "x = {}                       # 0-14",
        c.numbers.chest.x
    ));
    line(&format!(
        "y = {}                       # 0-15",
        c.numbers.chest.y
    ));
    line(&format!(
        "size = {}                   # 0-15",
        c.numbers.chest.size
    ));
    line("");
    line("[number.shorts]");
    line(&format!(
        "side = \"{}\"               # \"left\" / \"right\"",
        match c.numbers.shorts.side {
            Side::Left => "left",
            Side::Right => "right",
        }
    ));
    line(&format!(
        "x = {}                       # 0-14",
        c.numbers.shorts.x
    ));
    line(&format!(
        "y = {}                       # 0-14",
        c.numbers.shorts.y
    ));
    line(&format!(
        "size = {}                    # 0-13",
        c.numbers.shorts.size
    ));
    line("");
    line("[badge]                     # sleeve badge positions; x 0-14, y 0-31");
    for (name, position) in [
        ("right_short", c.badges.right_short),
        ("left_short", c.badges.left_short),
        ("right_long", c.badges.right_long),
        ("left_long", c.badges.left_long),
    ] {
        line(&format!(
            "{name} = {{ x = {}, y = {} }}",
            position.x, position.y
        ));
    }

    let differing: Vec<(u8, u8)> = (0u8..120)
        .map(|offset| (offset, remainder(&c.unknown, offset)))
        .filter(|(offset, value)| *value != remainder(&template.unknown, *offset))
        .collect();
    if !differing.is_empty() {
        line("");
        line("[unknown]                   # bits not yet decoded, carried over unchanged;");
        line("# listed only where they differ from the default kit config");
        for (offset, value) in differing {
            line(&format!("\"0x{offset:02X}\" = {value}"));
        }
    }

    if let Some(names) = &c.source_texture_names {
        line("");
        line("[source_texture_names]");
        for (field, name) in TEXTURE_NAME_FIELDS.iter().zip(names.iter()) {
            line(&format!("{field} = \"{}\"", name_hex(name)));
        }
    }

    out
}

/// Every section or subsection `update_toml` writes must be a table when
/// present — checked before any edit so a malformed document is left
/// untouched rather than panicking on `toml_edit`'s indexing.
fn check_update_shape(document: &DocumentMut) -> Result<(), KitConfigError> {
    fn check(item: Option<&Item>, key: &'static str) -> Result<(), KitConfigError> {
        match item {
            None | Some(Item::None) => Ok(()),
            Some(item) if item.as_table_like().is_some() => Ok(()),
            _ => Err(KitConfigError::NotATable { key }),
        }
    }
    for key in [
        "shirt",
        "shorts",
        "colors",
        "name",
        "number",
        "badge",
        "unknown",
        "source_texture_names",
    ] {
        check(document.get(key), key)?;
    }
    if let Some(number) = document.get("number").and_then(|item| item.as_table_like()) {
        for (sub, key) in [
            ("back", "number.back"),
            ("chest", "number.chest"),
            ("shorts", "number.shorts"),
        ] {
            check(number.get(sub), key)?;
        }
    }
    if let Some(badge) = document.get("badge").and_then(|item| item.as_table_like()) {
        for (sub, key) in [
            ("right_short", "badge.right_short"),
            ("left_short", "badge.left_short"),
            ("right_long", "badge.right_long"),
            ("left_long", "badge.left_long"),
        ] {
            check(badge.get(sub), key)?;
        }
    }
    Ok(())
}

/// Sets every value in an existing document, preserving its comments and
/// formatting; `[unknown]` keys are added, updated or removed as needed.
pub fn update_toml(config: &KitConfig, document: &mut DocumentMut) -> Result<(), KitConfigError> {
    check_update_shape(document)?;

    fn set(item: &mut Item, value: toml_edit::Value) {
        let decor = item.as_value().map(|old| old.decor().clone());
        *item = Item::Value(value);
        if let (Some(decor), Item::Value(new)) = (decor, item) {
            *new.decor_mut() = decor;
        }
    }

    set(
        &mut document["shirt"]["model"],
        i64::from(config.shirt.model).into(),
    );
    set(
        &mut document["shirt"]["collar"],
        i64::from(config.shirt.collar).into(),
    );
    set(
        &mut document["shirt"]["winter_collar"],
        i64::from(config.shirt.winter_collar).into(),
    );
    set(&mut document["shirt"]["tight"], config.shirt.tight.into());
    set(
        &mut document["shirt"]["pattern"],
        i64::from(config.shirt.pattern).into(),
    );
    set(
        &mut document["shirt"]["long_sleeves"],
        match config.shirt.long_sleeves {
            LongSleeves::Normal => "normal".into(),
            LongSleeves::UndershirtOnly => "undershirt-only".into(),
            LongSleeves::Raw(raw) => i64::from(raw).into(),
        },
    );
    set(
        &mut document["shirt"]["short_sleeves"],
        match config.shirt.short_sleeves {
            ShortSleeves::Normal => "normal".into(),
            ShortSleeves::CutOut => "cut-out".into(),
            ShortSleeves::Raw(raw) => i64::from(raw).into(),
        },
    );
    set(
        &mut document["shorts"]["model"],
        i64::from(config.shorts.model).into(),
    );
    set(
        &mut document["colors"]["shirt1"],
        rgb_string(config.colors.shirt1).into(),
    );
    set(
        &mut document["colors"]["shirt2"],
        rgb_string(config.colors.shirt2).into(),
    );
    set(
        &mut document["colors"]["undershirt"],
        rgb_string(config.colors.undershirt).into(),
    );
    set(
        &mut document["colors"]["shorts"],
        rgb_string(config.colors.shorts).into(),
    );
    set(
        &mut document["colors"]["socks"],
        rgb_string(config.colors.socks).into(),
    );
    set(&mut document["name"]["show"], config.name.show.into());
    set(
        &mut document["name"]["shape"],
        shape_repr(config.name.shape).into(),
    );
    set(&mut document["name"]["y"], i64::from(config.name.y).into());
    set(
        &mut document["name"]["size"],
        i64::from(config.name.size).into(),
    );
    set(
        &mut document["number"]["back"]["y"],
        i64::from(config.numbers.back.y).into(),
    );
    set(
        &mut document["number"]["back"]["size"],
        i64::from(config.numbers.back.size).into(),
    );
    set(
        &mut document["number"]["back"]["spacing"],
        i64::from(config.numbers.back.spacing).into(),
    );
    set(
        &mut document["number"]["chest"]["x"],
        i64::from(config.numbers.chest.x).into(),
    );
    set(
        &mut document["number"]["chest"]["y"],
        i64::from(config.numbers.chest.y).into(),
    );
    set(
        &mut document["number"]["chest"]["size"],
        i64::from(config.numbers.chest.size).into(),
    );
    set(
        &mut document["number"]["shorts"]["side"],
        match config.numbers.shorts.side {
            Side::Left => "left",
            Side::Right => "right",
        }
        .into(),
    );
    set(
        &mut document["number"]["shorts"]["x"],
        i64::from(config.numbers.shorts.x).into(),
    );
    set(
        &mut document["number"]["shorts"]["y"],
        i64::from(config.numbers.shorts.y).into(),
    );
    set(
        &mut document["number"]["shorts"]["size"],
        i64::from(config.numbers.shorts.size).into(),
    );
    for (name, position) in [
        ("right_short", config.badges.right_short),
        ("left_short", config.badges.left_short),
        ("right_long", config.badges.right_long),
        ("left_long", config.badges.left_long),
    ] {
        let entry = &mut document["badge"][name];
        if entry.is_none() {
            // Missing position: emit the inline form `to_toml` writes.
            let mut inline = toml_edit::InlineTable::new();
            inline.insert("x", i64::from(position.x).into());
            inline.insert("y", i64::from(position.y).into());
            set(entry, toml_edit::Value::InlineTable(inline));
        } else {
            // A present table (standard or inline, preflight-checked) keeps
            // its form: write x/y into it.
            set(&mut entry["x"], i64::from(position.x).into());
            set(&mut entry["y"], i64::from(position.y).into());
        }
    }

    let template = template();
    let needed: Vec<(u8, u8)> = (0u8..120)
        .map(|offset| (offset, remainder(&config.unknown, offset)))
        .filter(|(offset, value)| *value != remainder(&template.unknown, *offset))
        .collect();
    if needed.is_empty() {
        document.remove("unknown");
    } else {
        // Preflight made `unknown` a table — standard or inline — or absent.
        let entry = document["unknown"].or_insert(Item::Table(Table::new()));
        if let Some(table) = entry.as_table_like_mut() {
            // Update still-needed keys in place (keeping each key's spelling
            // and decor), drop keys whose remainder returned to the
            // template's, then add the missing ones.
            let existing: Vec<(String, Option<u8>)> = table
                .iter()
                .map(|(key, _)| {
                    (
                        key.to_owned(),
                        key.strip_prefix("0x")
                            .and_then(|hex| u8::from_str_radix(hex, 16).ok()),
                    )
                })
                .collect();
            let mut written = std::collections::BTreeSet::new();
            for (key, offset) in existing {
                match offset.and_then(|offset| needed.iter().find(|(o, _)| *o == offset)) {
                    Some(&(offset, value)) => {
                        written.insert(offset);
                        if let Some(item) = table.get_mut(&key) {
                            set(item, i64::from(value).into());
                        }
                    }
                    None => {
                        table.remove(&key);
                    }
                }
            }
            for (offset, value) in needed {
                if !written.contains(&offset) {
                    table
                        .entry(format!("0x{offset:02X}").as_str())
                        .or_insert(toml_edit::value(i64::from(value)));
                }
            }
        }
    }

    match &config.source_texture_names {
        Some(names) => {
            for (field, name) in TEXTURE_NAME_FIELDS.iter().zip(names.iter()) {
                set(
                    &mut document["source_texture_names"][field],
                    name_hex(name).into(),
                );
            }
        }
        None => {
            document.remove("source_texture_names");
        }
    }
    Ok(())
}
