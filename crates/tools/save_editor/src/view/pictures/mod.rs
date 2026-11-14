//! The formation pictures: one PNG of a team's formations, a row per
//! preset and a column per formation, which organizers check a team's
//! shapes with and share (the draft's `formationsPicture`).
//!
//! A picture is painted in an egui context of its own that is never
//! shown, by the Tactics view's own pitch painter (`pitch::still_pitch`),
//! and `raster` fills the triangles epaint tessellates the frame into.
//! A second painter drawing straight into an image would have to be kept
//! in step with the view's by hand, one shape kind at a time.

mod raster;

use std::path::Path;

use image::RgbaImage;
use pes_savefile::EditFile;
use pes_savefile::model::TeamEntry;
use pes_savefile::model::names::display_name;
use pes_savefile::model::tactics::{Formation, FormationSlot, TacticsPreset};
use pes_version::PesVersion;
use team_widgets::names::append_name;
use team_widgets::pitch::{self, FORMATIONS, PITCH};
use team_widgets::tactics::TacticsView;

use crate::view::{portraits, tactics};

/// The empty room around the picture's content.
const MARGIN: f32 = 16.0;
/// The row labels' column, left of the grid.
const LABEL_WIDTH: f32 = 96.0;
/// The gap between two cells, across and down.
const GAP: f32 = 12.0;
/// The title's line.
const TITLE_HEIGHT: f32 = 30.0;
/// The column heads' line, over a grid of three columns.
const HEAD_HEIGHT: f32 = 22.0;
/// The team name's size in the title.
const TITLE_SIZE: f32 = 18.0;
/// Every other text's size, the body text's.
const TEXT_SIZE: f32 = 12.5;
/// A row label's first line's centre, below the row's top.
const LABEL_TOP: f32 = 10.0;
/// The step from a row label's line to the next.
const LINE: f32 = 15.0;
/// The gap between the team's name and its id in the title.
const ID_GAP: f32 = 8.0;
/// The opacity of a formation drawn though its preset does not field it,
/// so it reads as unused rather than as part of a fluid set.
const HIDDEN_OPACITY: f32 = 0.6;

/// What a picture draws besides the formations.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PictureOptions {
    /// The starters' portraits over their badges.
    pub(crate) portraits: bool,
    /// The turf in flat bands, without its grain, which PNG cannot
    /// compress: a smaller file.
    pub(crate) flat: bool,
}

/// The team a picture is of, and where its markers' medals, crosses and
/// portraits come from.
pub(crate) struct PictureTeam<'a> {
    /// The open save.
    pub(crate) file: &'a EditFile,
    /// The team, one of `file`'s.
    pub(crate) team: &'a TeamEntry,
    /// The active ruleset, which the badges' medal outlines and the
    /// crosses' wording come from; `None` draws neither.
    pub(crate) rules: Option<&'a aatf::Rules>,
    /// Live AATF's violations of the team under `rules`: a starter they
    /// name as fielded where he is not rated A gets a cross. Empty draws
    /// none.
    pub(crate) violations: &'a [aatf::Violation],
    /// The save's version's PES folder, the portraits' source; `None`
    /// draws every starter's placeholder portrait.
    pub(crate) pes_folder: Option<&'a Path>,
}

/// The picture of `team`'s formations, drawn in `visuals` (the theme in
/// use, whose panel colour is its background).
pub(crate) fn formations_picture(
    team: &PictureTeam<'_>,
    visuals: &egui::Visuals,
    options: PictureOptions,
) -> RgbaImage {
    paint(team, visuals, options).0
}

/// The picture's file name: the team's display name less the characters a
/// file name cannot hold (`f.png` for /f/), or `team_id` when nothing is
/// left of it, and ` settings` before the extension for a picture with the
/// presets' settings.
pub(crate) fn picture_file_name(team_id: u32, raw_name: &str, settings: bool) -> String {
    let name: String = display_name(raw_name)
        .chars()
        .filter(|c| !r#"\/:*?"<>|"#.contains(*c))
        .collect();
    let name = name.trim();
    let mut file = if name.is_empty() {
        team_id.to_string()
    } else {
        name.to_owned()
    };
    if settings {
        file.push_str(" settings");
    }
    file.push_str(".png");
    file
}

/// The formations a preset's row draws, by index into its three: the
/// ones it fields, or all three when Fluid formation is off and one of
/// the other two has a shape of its own (`same_shape`), since a team can
/// hide formations there and switch Fluid formation on in a match.
fn pictured_formations(preset: &TacticsPreset) -> &'static [usize] {
    let [kick_off, others @ ..] = &preset.formations;
    if preset.style.fluid == Some(false) && !others.iter().all(|other| same_shape(other, kick_off))
    {
        &[0, 1, 2]
    } else {
        preset.fielded_formations()
    }
}

/// How far, in stored units on each axis, a spot may stand from
/// Kick-off's and still count as Kick-off's in `same_shape`.
const SHAPE_TOLERANCE: u8 = 2;

/// Whether `formation` has `kick_off`'s shape: its eleven stored spots
/// pair one to one with Kick-off's, each within `SHAPE_TOLERANCE` on both
/// axes, whoever stands on them and whatever position they are labelled
/// with. A shape, not the whole formation, because players trading spots
/// changes no shape, and drawing such a copy would show three of one shape
/// as if the team hid another.
///
/// The pairing backtracks: with a tolerance, the first Kick-off spot in
/// reach of one slot may be the only one in reach of another, so a greedy
/// pick can miss a pairing that exists.
fn same_shape(formation: &Formation, kick_off: &Formation) -> bool {
    /// Pairs `slots` with the Kick-off spots still `free`, undoing a
    /// pick that leaves a later slot without one.
    fn pair(slots: &[FormationSlot], spots: &[FormationSlot; 11], free: &mut [bool; 11]) -> bool {
        let Some((slot, rest)) = slots.split_first() else {
            return true;
        };
        for (index, spot) in spots.iter().enumerate() {
            let near = slot.x.abs_diff(spot.x) <= SHAPE_TOLERANCE
                && slot.y.abs_diff(spot.y) <= SHAPE_TOLERANCE;
            if free[index] && near {
                free[index] = false;
                if pair(rest, spots, free) {
                    return true;
                }
                free[index] = true;
            }
        }
        false
    }
    pair(&formation.players, &kick_off.players, &mut [true; 11])
}

/// `team`'s picture painted off screen at one pixel per point and filled
/// into pixels, with the shapes its frame painted (the tests read its
/// texts off them).
fn paint(
    team: &PictureTeam<'_>,
    visuals: &egui::Visuals,
    options: PictureOptions,
) -> (RgbaImage, Vec<egui::epaint::ClippedShape>) {
    let file = team.file;
    let version = file.version();
    let presets = &team.team.tactics.presets;
    let rows = presets.each_ref().map(pictured_formations);
    // One column when every row draws Kick-off alone: a lone "Kick-off"
    // head would name a switch the team never makes.
    let columns = if rows.iter().all(|formations| formations.len() == 1) {
        1
    } else {
        FORMATIONS.len()
    };
    let head = if columns > 1 { HEAD_HEIGHT } else { 0.0 };
    let grid_width = columns as f32 * PITCH.x + (columns - 1) as f32 * GAP;
    let grid_height = rows.len() as f32 * PITCH.y + (rows.len() - 1) as f32 * GAP;
    let size = egui::vec2(
        2.0 * MARGIN + LABEL_WIDTH + grid_width,
        2.0 * MARGIN + TITLE_HEIGHT + head + grid_height,
    );

    let ctx = egui::Context::default();
    studio_core::theme::install(&ctx);
    ctx.set_visuals(visuals.clone());
    // The handles keep the textures alive until the frame is rasterized.
    let portraits = if options.portraits {
        starter_portraits(&ctx, team)
    } else {
        vec![None; team.team.roster.len()]
    };
    let medals = team.rules.map(|rules| {
        let view = tactics::medal_team_view(file, team.team.id);
        tactics::slot_medals(rules, view.as_ref(), team.team.roster.len())
    });
    let fielded = match team.rules {
        Some(rules) => tactics::fielded_marks(
            team.violations,
            rules,
            team.team.id,
            version,
            team.team.roster.len(),
        ),
        None => vec![Vec::new(); team.team.roster.len()],
    };
    let slots = tactics::roster_slots(file, team.team);
    let roster = tactics::roster_rows(
        &slots,
        portraits
            .iter()
            .map(|portrait| portrait.as_ref().map(egui::TextureHandle::id))
            .collect(),
        medals.as_deref(),
        &fielded,
    );

    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        let painter = ui.painter();
        let visuals = ui.visuals();
        painter.rect_filled(screen, 0.0, visuals.panel_fill);
        paint_title(painter, size.x, team.team, version);
        let grid = egui::pos2(MARGIN + LABEL_WIDTH, MARGIN + TITLE_HEIGHT + head);
        let cell = |row: usize, column: usize| {
            egui::Rect::from_min_size(
                grid + egui::vec2(column as f32, row as f32) * (PITCH + egui::Vec2::splat(GAP)),
                PITCH,
            )
        };
        let text = egui::FontId::proportional(TEXT_SIZE);
        if columns > 1 {
            for (column, name) in FORMATIONS.iter().enumerate() {
                painter.text(
                    egui::pos2(cell(0, column).center().x, grid.y - HEAD_HEIGHT / 2.0),
                    egui::Align2::CENTER_CENTER,
                    *name,
                    text.clone(),
                    visuals.strong_text_color(),
                );
            }
        }
        for (row, (preset, formations)) in presets.iter().zip(rows).enumerate() {
            let label = egui::pos2(MARGIN, cell(row, 0).top() + LABEL_TOP);
            painter.text(
                label,
                egui::Align2::LEFT_CENTER,
                format!("Preset {}", row + 1),
                text.clone(),
                visuals.strong_text_color(),
            );
            // PES 2015 has no switch, so no line.
            if let Some(fluid) = preset.style.fluid {
                painter.text(
                    label + egui::vec2(0.0, LINE),
                    egui::Align2::LEFT_CENTER,
                    if fluid { "Fluid On" } else { "Fluid Off" },
                    text.clone(),
                    visuals.text_color(),
                );
            }
            for (column, &formation) in formations.iter().enumerate() {
                let mut painter = painter.clone();
                if column > 0 && preset.style.fluid != Some(true) {
                    painter.multiply_opacity(HIDDEN_OPACITY);
                }
                let mut view = TacticsView::default();
                view.preset = row;
                view.formation = formation;
                view.portraits = options.portraits;
                pitch::still_pitch(
                    &painter,
                    cell(row, column),
                    version,
                    &team.team.tactics,
                    &view,
                    &roster,
                    options.flat,
                );
            }
        }
    });
    let shapes = output.shapes.clone();
    let primitives = ctx.tessellate(output.shapes, 1.0);
    let image = raster::rasterize(
        size.x as u32,
        size.y as u32,
        &primitives,
        &mut output.textures_delta,
    );
    (image, shapes)
}

/// The title: `team`'s name in its colour codes' colours (the strong text
/// colour where it has none), its id after it and `version` at the
/// picture's right, `width` wide. A folder of pictures from several saves
/// would otherwise not say which game each came from.
fn paint_title(painter: &egui::Painter, width: f32, team: &TeamEntry, version: PesVersion) {
    let visuals = &painter.ctx().global_style().visuals;
    let centre = MARGIN + TITLE_HEIGHT / 2.0;
    let mut job = egui::text::LayoutJob::default();
    append_name(
        &mut job,
        &team.name,
        &egui::FontId::proportional(TITLE_SIZE),
    );
    for section in &mut job.sections {
        if section.format.color == egui::Color32::PLACEHOLDER {
            section.format.color = visuals.strong_text_color();
        }
    }
    let galley = painter.layout_job(job);
    let name_end = MARGIN + galley.size().x;
    painter.galley(
        egui::pos2(MARGIN, centre - galley.size().y / 2.0),
        galley,
        visuals.strong_text_color(),
    );
    let text = egui::FontId::proportional(TEXT_SIZE);
    painter.text(
        egui::pos2(name_end + ID_GAP, centre),
        egui::Align2::LEFT_CENTER,
        team.id.to_string(),
        text.clone(),
        visuals.text_color(),
    );
    painter.text(
        egui::pos2(width - MARGIN, centre),
        egui::Align2::RIGHT_CENTER,
        version.to_string(),
        text,
        visuals.text_color(),
    );
}

/// The portraits of `team`'s starters loaded into `ctx`, one handle per
/// roster slot: a starter's own portrait, or the version's placeholder
/// where the game holds none (as the view shows it); `None` off the
/// eleven, whose markers the picture does not draw.
fn starter_portraits(
    ctx: &egui::Context,
    team: &PictureTeam<'_>,
) -> Vec<Option<egui::TextureHandle>> {
    let version = team.file.version();
    let starters = &team.team.tactics.starting_eleven;
    let ids: Vec<Option<u32>> = tactics::roster_player_ids(team.file, team.team)
        .into_iter()
        .enumerate()
        .map(|(slot, id)| {
            id.filter(|_| starters.iter().any(|&starter| usize::from(starter) == slot))
        })
        .collect();
    let images = portraits::read_images(team.pes_folder, version, &ids);
    let placeholder = ctx.load_texture(
        "placeholder portrait",
        portraits::placeholder_image(version),
        egui::TextureOptions::LINEAR,
    );
    ids.iter()
        .zip(images)
        .map(|(id, image)| {
            let id = (*id)?;
            Some(match image {
                Some(image) => ctx.load_texture(
                    format!("portrait {id}"),
                    image,
                    egui::TextureOptions::LINEAR,
                ),
                None => placeholder.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use egui::epaint::{ClippedShape, Shape};
    use egui::{Color32, Pos2};
    use pes_savefile::fixture_save;
    use pes_savefile::interchange::team_toml::labels::POSITIONS;

    use super::*;

    /// The PES 2021 fixture's /f/, whose three presets have Fluid
    /// formation off and their other two formations copies of Kick-off.
    const KICK_OFF_ALONE: u32 = 718;
    /// The PES 2021 fixture's /2hug/, whose three presets have Fluid
    /// formation on.
    const FLUID: u32 = 783;
    /// The PES 2021 fixture's /gbpen/: presets 1 and 3 have Fluid
    /// formation off, their other two formations holding Kick-off's spots
    /// with players traded; preset 2 has it on.
    const TRADED: u32 = 798;
    /// The PES 2021 fixture's /nepgen/: preset 1 has Fluid formation on,
    /// preset 2 has it off with a formation of a shape of its own, preset
    /// 3 has it off with two of Kick-off's shape.
    const HIDDEN: u32 = 813;

    /// `version`'s fixture save.
    fn save(version: PesVersion) -> EditFile {
        EditFile::from_bytes(&fixture_save(version)).expect("the fixture save opens")
    }

    /// The window's dark theme, as `theme::install` sets it.
    fn dark() -> egui::Visuals {
        let ctx = egui::Context::default();
        studio_core::theme::install(&ctx);
        ctx.style_of(egui::Theme::Dark).visuals.clone()
    }

    /// `team` of `file` with no ruleset, no crosses and no PES folder.
    fn bare<'a>(file: &'a EditFile, team: &'a TeamEntry) -> PictureTeam<'a> {
        PictureTeam {
            file,
            team,
            rules: None,
            violations: &[],
            pes_folder: None,
        }
    }

    /// No portraits, the turf `flat` or grained.
    fn without_portraits(flat: bool) -> PictureOptions {
        PictureOptions {
            portraits: false,
            flat,
        }
    }

    /// `team`'s picture from `file` in the dark theme, with no ruleset,
    /// no crosses and no portraits.
    fn picture(file: &EditFile, team: &TeamEntry, flat: bool) -> (RgbaImage, Vec<ClippedShape>) {
        paint(&bare(file, team), &dark(), without_portraits(flat))
    }

    /// The top-left corner of a picture's grid, under its column heads
    /// when it has them.
    fn grid(heads: bool) -> Pos2 {
        let head = if heads { HEAD_HEIGHT } else { 0.0 };
        egui::pos2(MARGIN + LABEL_WIDTH, MARGIN + TITLE_HEIGHT + head)
    }

    /// The pixel rect of the cell of `row` and `column` in a grid at
    /// `grid`.
    fn cell(grid: Pos2, row: u32, column: u32) -> egui::Rect {
        let step = PITCH + egui::Vec2::splat(GAP);
        egui::Rect::from_min_size(
            grid + egui::vec2(column as f32 * step.x, row as f32 * step.y),
            PITCH,
        )
    }

    /// The texts the frame painted outside the grid at `grid` (the title,
    /// the column heads and the row labels), in paint order, each with
    /// its first run's colour.
    fn chrome_texts(shapes: &[ClippedShape], grid: Pos2) -> Vec<(String, Color32)> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Text(text) if text.pos.x < grid.x || text.pos.y < grid.y => Some((
                    text.galley.text().to_owned(),
                    text.galley.job.sections[0].format.color,
                )),
                _ => None,
            })
            .collect()
    }

    /// `version`'s turf texture, `flat` or grained, as the still pitch
    /// loads it: read off a frame of its own.
    fn turf_texture(version: PesVersion, flat: bool) -> egui::ColorImage {
        let file = save(version);
        let team = file.team(KICK_OFF_ALONE).expect("the fixture holds /f/");
        let ctx = egui::Context::default();
        studio_core::theme::install(&ctx);
        let screen = egui::Rect::from_min_size(Pos2::ZERO, PITCH);
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let view = TacticsView::default();
            pitch::still_pitch(
                ui.painter(),
                screen,
                version,
                &team.tactics,
                &view,
                &[],
                flat,
            );
        });
        let turf = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Rect(rect) if rect.fill_texture_id() != egui::TextureId::default() => {
                    Some(rect.fill_texture_id())
                }
                _ => None,
            })
            .expect("the turf, the frame's one textured rect");
        let deltas = output
            .textures_delta
            .set
            .remove(&turf)
            .expect("the turf's delta");
        output.textures_delta.clear();
        let egui::epaint::ImageData::Color(image) = &deltas[0].image;
        (**image).clone()
    }

    /// `texture` drawn over a cell `PITCH` in size and sampled at the
    /// centre of the cell's pixel (`x`, `y`) as the GPU samples it:
    /// between the four nearest texel centres, weighted by distance.
    fn sampled(texture: &egui::ColorImage, x: f32, y: f32) -> [f32; 4] {
        let [width, height] = texture.size;
        let u = (x + 0.5) / PITCH.x * width as f32 - 0.5;
        let v = (y + 0.5) / PITCH.y * height as f32 - 0.5;
        let (left, top) = (u.floor(), v.floor());
        let texel = |x: f32, y: f32| {
            let x = (x.max(0.0) as usize).min(width - 1);
            let y = (y.max(0.0) as usize).min(height - 1);
            texture.pixels[y * width + x].to_array().map(f32::from)
        };
        let (across, down) = (u - left, v - top);
        std::array::from_fn(|channel| {
            let above = texel(left, top)[channel] * (1.0 - across)
                + texel(left + 1.0, top)[channel] * across;
            let below = texel(left, top + 1.0)[channel] * (1.0 - across)
                + texel(left + 1.0, top + 1.0)[channel] * across;
            above * (1.0 - down) + below * down
        })
    }

    /// The pixels of `area` (cell-relative, in the cell at `cell`) whose
    /// centre no shape but the background and the turf reaches: no line,
    /// marker or text, their bounds widened a pixel for the edges' soft
    /// fringe.
    fn bare_turf(shapes: &[ClippedShape], cell: egui::Rect, area: egui::Rect) -> Vec<(u32, u32)> {
        let covers: Vec<egui::Rect> = shapes
            .iter()
            .filter(|clipped| match &clipped.shape {
                Shape::Rect(rect) => rect.rect.width() < PITCH.x,
                _ => true,
            })
            .map(|clipped| clipped.shape.visual_bounding_rect().expand(1.0))
            .collect();
        let mut pixels = Vec::new();
        for y in area.top() as u32..area.bottom() as u32 {
            for x in area.left() as u32..area.right() as u32 {
                let centre = cell.min + egui::vec2(x as f32 + 0.5, y as f32 + 0.5);
                if !covers.iter().any(|cover| cover.contains(centre)) {
                    pixels.push((x, y));
                }
            }
        }
        pixels
    }

    /// Asserts `pixel` within 2 of `expected` in every channel.
    fn within_two(pixel: [u8; 4], expected: [f32; 4], what: &str) {
        for (channel, (got, wanted)) in pixel.iter().zip(expected).enumerate() {
            assert!(
                (f32::from(*got) - wanted).abs() <= 2.0,
                "{what}: channel {channel} is {got}, {wanted} expected ({pixel:?})"
            );
        }
    }

    // SE-GUI-87
    #[test]
    fn a_picture_is_a_column_per_fielded_formation_under_its_title() {
        let file = save(PesVersion::Pes21);
        let visuals = dark();
        let (strong, text) = (visuals.strong_text_color(), visuals.text_color());
        let kick_off_alone = file.team(KICK_OFF_ALONE).expect("the fixture holds /f/");
        assert!(
            kick_off_alone.tactics.presets.iter().all(|preset| {
                let [kick_off, others @ ..] = &preset.formations;
                preset.style.fluid == Some(false) && others.iter().all(|other| other == kick_off)
            }),
            "premise: /f/'s presets field Kick-off alone and copy it"
        );
        let (image, shapes) = picture(&file, kick_off_alone, false);
        assert_eq!(
            image.dimensions(),
            (32 + 96 + 320, 32 + 30 + 3 * 384 + 2 * 12),
            "one column"
        );
        let mut expected = vec![
            ("/f/".to_owned(), strong),
            ("718".to_owned(), text),
            ("PES 2021".to_owned(), text),
        ];
        for row in 1..=3 {
            expected.push((format!("Preset {row}"), strong));
            expected.push(("Fluid Off".to_owned(), text));
        }
        assert_eq!(
            chrome_texts(&shapes, grid(false)),
            expected,
            "no column heads"
        );

        let fluid = file.team(FLUID).expect("the fixture holds /2hug/");
        assert!(
            fluid
                .tactics
                .presets
                .iter()
                .all(|preset| preset.style.fluid == Some(true)),
            "premise: /2hug/'s presets are fluid"
        );
        let (image, shapes) = picture(&file, fluid, false);
        assert_eq!(
            image.dimensions(),
            (32 + 96 + 3 * 320 + 2 * 12, 32 + 30 + 22 + 3 * 384 + 2 * 12),
            "three columns, under their heads"
        );
        let mut expected = vec![
            ("/2hug/".to_owned(), strong),
            ("783".to_owned(), text),
            ("PES 2021".to_owned(), text),
        ];
        expected.extend(FORMATIONS.map(|name| (name.to_owned(), strong)));
        for row in 1..=3 {
            expected.push((format!("Preset {row}"), strong));
            expected.push(("Fluid On".to_owned(), text));
        }
        assert_eq!(chrome_texts(&shapes, grid(true)), expected);

        // A name's colour codes colour the title's runs.
        let mut coded = kick_off_alone.clone();
        coded.name = "/\x11cb7bec5fff\x11d/".to_owned();
        let (_, shapes) = picture(&file, &coded, false);
        let title = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Text(text) if text.galley.text() == "/f/" => Some(text.galley.clone()),
                _ => None,
            })
            .expect("the title's name");
        let runs: Vec<Color32> = title
            .job
            .sections
            .iter()
            .map(|section| section.format.color)
            .collect();
        assert_eq!(runs, [strong, Color32::from_rgb(0xb7, 0xbe, 0xc5), strong]);
    }

    // SE-GUI-87
    #[test]
    fn a_picture_s_turf_is_the_view_s_and_a_hidden_formation_recedes() {
        let file = save(PesVersion::Pes21);
        let team = file.team(HIDDEN).expect("the fixture holds /nepgen/");
        let [first, second, third] = &team.tactics.presets;
        assert_eq!(first.style.fluid, Some(true), "premise: preset 1 is fluid");
        assert_eq!(second.style.fluid, Some(false), "premise: preset 2 is not");
        assert!(
            !same_shape(&second.formations[2], &second.formations[0]),
            "premise: preset 2's Out-of-possession has a shape of its own"
        );
        assert_eq!(third.style.fluid, Some(false), "premise: preset 3 is not");
        assert!(
            third.formations[1..]
                .iter()
                .all(|other| same_shape(other, &third.formations[0])),
            "premise: preset 3's other two have Kick-off's shape"
        );
        let background = dark().panel_fill.to_array().map(f32::from);
        let grid = grid(true);
        // A patch of each cell clear of the penalty boxes and the halfway
        // line: the markers in it are left out pixel by pixel.
        let patch = egui::Rect::from_min_max(egui::pos2(4.0, 100.0), egui::pos2(316.0, 160.0));
        for flat in [false, true] {
            let turf = turf_texture(PesVersion::Pes21, flat);
            let (image, shapes) = picture(&file, team, flat);
            // Preset 3's row draws Kick-off alone: the room beside it is
            // the background's.
            let empty = cell(grid, 2, 1);
            for y in empty.top() as u32..empty.bottom() as u32 {
                for x in empty.left() as u32..empty.right() as u32 {
                    assert_eq!(
                        image.get_pixel(x, y).0,
                        dark().panel_fill.to_array(),
                        "preset 3's In-possession room, flat {flat}, ({x}, {y})"
                    );
                }
            }
            for (row, column, opacity, what) in [
                (0, 0, 1.0, "preset 1's Kick-off"),
                (0, 1, 1.0, "preset 1's fluid In-possession"),
                (1, 1, HIDDEN_OPACITY, "preset 2's hidden In-possession"),
                (1, 2, HIDDEN_OPACITY, "preset 2's hidden Out-of-possession"),
            ] {
                let cell = cell(grid, row, column);
                let pixels = bare_turf(&shapes, cell, patch);
                assert!(pixels.len() > 1000, "{what}: {} bare pixels", pixels.len());
                for (x, y) in pixels {
                    let expected: [f32; 4] = std::array::from_fn(|channel| {
                        sampled(&turf, x as f32, y as f32)[channel] * opacity
                            + background[channel] * (1.0 - opacity)
                    });
                    let pixel = image
                        .get_pixel(cell.left() as u32 + x, cell.top() as u32 + y)
                        .0;
                    within_two(pixel, expected, &format!("{what}, flat {flat}, ({x}, {y})"));
                }
            }
        }
    }

    // SE-GUI-87
    #[test]
    fn a_flat_picture_paints_each_band_one_colour_in_a_smaller_file() {
        let file = save(PesVersion::Pes21);
        let team = file.team(TRADED).expect("the fixture holds /gbpen/");
        let cell = cell(grid(true), 0, 0);
        // Inside PES 2021's second band (34.9 to 69.8), left of the penalty
        // box.
        let band = egui::Rect::from_min_max(egui::pos2(2.0, 38.0), egui::pos2(76.0, 66.0));
        let colours = |image: &RgbaImage, pixels: &[(u32, u32)]| {
            let mut colours: Vec<[u8; 4]> = pixels
                .iter()
                .map(|&(x, y)| {
                    image
                        .get_pixel(cell.left() as u32 + x, cell.top() as u32 + y)
                        .0
                })
                .collect();
            colours.sort_unstable();
            colours.dedup();
            colours
        };
        let png = |flat: bool| {
            let image = formations_picture(&bare(&file, team), &dark(), without_portraits(flat));
            let mut bytes = std::io::Cursor::new(Vec::new());
            image
                .write_to(&mut bytes, image::ImageFormat::Png)
                .expect("a picture encodes");
            bytes.into_inner().len()
        };
        let (grained, shapes) = picture(&file, team, false);
        let pixels = bare_turf(&shapes, cell, band);
        assert!(pixels.len() > 500, "{} bare pixels", pixels.len());
        assert!(
            colours(&grained, &pixels).len() > 1,
            "premise: the grained band's pixels differ"
        );
        let (flat, shapes) = picture(&file, team, true);
        assert_eq!(bare_turf(&shapes, cell, band), pixels, "the same markers");
        assert_eq!(colours(&flat, &pixels).len(), 1, "one colour");
        let (grained, flat) = (png(false), png(true));
        assert!(
            flat < grained,
            "the flat PNG, {flat} bytes, under {grained}"
        );
    }

    // SE-GUI-87
    #[test]
    fn a_pes_2015_picture_draws_kick_off_alone_with_no_fluid_line() {
        let file = save(PesVersion::Pes15);
        let team = file.team(KICK_OFF_ALONE).expect("the fixture holds /f/");
        let (image, shapes) = picture(&file, team, false);
        assert_eq!(
            image.dimensions(),
            (32 + 96 + 320, 32 + 30 + 3 * 384 + 2 * 12),
            "one column: every row draws Kick-off alone"
        );
        let texts: Vec<String> = chrome_texts(&shapes, grid(false))
            .into_iter()
            .map(|(text, _)| text)
            .collect();
        assert_eq!(
            texts,
            ["/f/", "718", "PES 2015", "Preset 1", "Preset 2", "Preset 3"]
        );
    }

    // SE-GUI-87
    #[test]
    fn a_formation_of_kick_off_s_shape_is_not_drawn() {
        let file = save(PesVersion::Pes21);
        let traded = file.team(TRADED).expect("the fixture holds /gbpen/");
        let [first, _, third] = &traded.tactics.presets;
        for (preset, which) in [(first, "preset 1"), (third, "preset 3")] {
            assert_eq!(
                preset.style.fluid,
                Some(false),
                "premise: {which} is not fluid"
            );
            assert!(
                preset.formations[1..]
                    .iter()
                    .any(|other| *other != preset.formations[0]),
                "premise: {which} holds a formation unequal to Kick-off"
            );
            assert_eq!(pictured_formations(preset), [0], "{which}");
        }
        let [kick_off, in_possession, _] = &first.formations;
        assert!(
            in_possession.players.iter().any(|slot| {
                kick_off.players.iter().any(|spot| {
                    (spot.x, spot.y) == (slot.x, slot.y) && spot.position != slot.position
                })
            }),
            "premise: preset 1 labels one of Kick-off's spots otherwise"
        );

        // One spot moved: 2 across and 2 down keeps the shape, 3 down
        // makes one of its own.
        let amf = in_possession.players[8];
        assert_eq!(
            (POSITIONS[usize::from(amf.position)], amf.x, amf.y),
            ("AMF", 37, 30),
            "premise: slot 8 of preset 1's In-possession"
        );
        for (x, y, expected) in [(39, 28, &[0][..]), (37, 27, &[0, 1, 2][..])] {
            let mut moved = first.clone();
            moved.formations[1].players[8].x = x;
            moved.formations[1].players[8].y = y;
            assert_eq!(pictured_formations(&moved), expected, "AMF at ({x}, {y})");
        }

        // A pairing a greedy pick misses: In-possession's slot 9 stands
        // within 2 of both Kick-off spots 9 and 10, its slot 10 within 2 of
        // spot 9 alone.
        let mut crowded = file
            .team(KICK_OFF_ALONE)
            .expect("the fixture holds /f/")
            .tactics
            .presets[0]
            .clone();
        let corner = |x: u8| FormationSlot {
            position: 9,
            x,
            y: 47,
        };
        assert!(
            crowded.formations[0].players[..9]
                .iter()
                .all(|slot| slot.x.abs_diff(92) > 4 || slot.y.abs_diff(47) > 4),
            "premise: no other Kick-off spot near the corner"
        );
        crowded.formations[0].players[9] = corner(92);
        crowded.formations[0].players[10] = corner(90);
        crowded.formations[1] = crowded.formations[0].clone();
        crowded.formations[2] = crowded.formations[0].clone();
        crowded.formations[1].players[10] = corner(94);
        assert_eq!(pictured_formations(&crowded), [0]);
    }

    #[test]
    fn a_picture_s_file_name_is_the_name_a_file_can_hold() {
        for (raw_name, settings, expected) in [
            ("/f/", false, "f.png"),
            ("/f/", true, "f settings.png"),
            // No team's name is made of these alone.
            (r#" /\:*?"<>| "#, false, "718.png"),
        ] {
            assert_eq!(
                picture_file_name(KICK_OFF_ALONE, raw_name, settings),
                expected
            );
        }
    }
}
