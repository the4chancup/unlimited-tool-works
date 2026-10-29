//! The shared look: the fonts, the drafts' style and the `theme` setting's
//! application to the egui context. `install`
//! puts the one font set and the pinned style on a context: `run_gui` calls it
//! at window creation and every view test's harness calls it too, so a size a
//! test measures is the size a member sees. The spacing and both themes'
//! widget states are egui 0.36's, the look the drafts were reviewed in, but
//! for the light fills, which keep the dark ones' contrast with the panel;
//! the rest of the presets stays egui's own `Visuals` until the curated one
//! lands.

use crate::settings::Theme;
use crate::status::NoticeKind;

/// Every colour the suite draws beyond egui's `Visuals`, one field each, with a
/// dark and a light instance picked by the theme ([`palette`]). Both instances
/// are this one struct, so a colour added to one and forgotten in the other
/// does not compile. A light value keeps the dark one's contrast against its
/// background; a colour drawn on a surface both themes share (the pitch, a
/// fill under its own text colour) is the same in both.
pub struct Palette {
    /// The success notices' bright green (the draft's `--success`). egui's
    /// `Visuals` has a warning colour and an error colour but no success one,
    /// so the suite carries its own. On the light theme a darker green: the
    /// dark one reads at a contrast of 1.7 against egui's light panel, where
    /// the darker one reads at 5.6 (the light theme's own link and warning
    /// colours are darker for the same reason).
    pub success: egui::Color32,
    /// The boots/gloves dialog's range end ID (the draft's
    /// `.boglo input.readout`): egui's weak grey on the box is as faded as a
    /// disabled field's text, which is barely readable. Its light value keeps
    /// the dark one's contrast, 6.3, on the light theme's faint box, where the
    /// dark grey itself would read at 2.6.
    pub readout: egui::Color32,
    /// A counted label's count under its limit (the draft's `.gen .count`, a
    /// disabled card label's `#5a5a5a`): quiet, so a count draws the eye only
    /// once the field reaches its limit. Its light value keeps the same quiet
    /// contrast, 2.5, on the light panel, where the dark grey itself would read
    /// at 6.5, as loud as the text.
    pub count_grey: egui::Color32,
    /// The positions grid's chosen C rating's fill, and a held side toggle's
    /// (the draft's `.sel.on.rc` `#5a5a5a`): the quiet grey a chosen rating or
    /// side gets where the accent would pull the eye. Its light value has the
    /// same contrast with the panel, 2.5, as the dark grey has with the dark
    /// one, and the light theme's dark text reads on it at 3.05 (on the dark
    /// grey itself, at 1.2).
    pub rating_c_fill: egui::Color32,
    /// The positions grid's chosen A rating's fill (the draft's `.sel.on.ra`
    /// `#129648`): a green deep enough to carry a white letter, where the
    /// theme's bright success green needed a dark one.
    pub rating_a_fill: egui::Color32,
    /// The letter on a chosen A rating's fill.
    pub rating_a_text: egui::Color32,
    /// The dark frame drawn inside a player appearance palette entry's square
    /// (the draft's `.swatch`), around the entry's own colour.
    pub swatch_edge: egui::Color32,
    /// The Tactics pitch's dark turf band (`#2d571d`), at both ends of the
    /// pitch and every second band between.
    pub turf_dark_band: egui::Color32,
    /// The Tactics pitch's light turf band (`#35602a`).
    pub turf_light_band: egui::Color32,
    /// The pitch lines' stroke: white at 20% opacity.
    pub pitch_lines: egui::Color32,
    /// A pitch marker's badge fill (`#242b2d`), and the captain mark's outline.
    pub badge_fill: egui::Color32,
    /// A join member's badge fill while join mode is armed (`#005c80`).
    pub join_fill: egui::Color32,
    /// The keeper's position text on the pitch's badges (`#ffde00`).
    pub keeper: egui::Color32,
    /// A defender's position text (`#00d2ff`).
    pub defender: egui::Color32,
    /// A midfielder's position text (`#57e42b`).
    pub midfielder: egui::Color32,
    /// A forward's position text (`#ff2c77`).
    pub forward: egui::Color32,
    /// The keeper's position text in the lineup lists, which sit on the
    /// panel: the pitch's `keeper` on the dark theme, and on the light one
    /// its hue darkened to read at 4.5 on the light panel, where the pitch's
    /// yellow reads at 1.26. The pitch keeps its own on its badges.
    pub lineup_keeper: egui::Color32,
    /// A defender's position text in the lineup lists, as `lineup_keeper`:
    /// the pitch's `defender` on the dark theme, darkened to read at 4.5 on
    /// the light one. The pitch keeps its own on its badges.
    pub lineup_defender: egui::Color32,
    /// A midfielder's position text in the lineup lists, as
    /// `lineup_keeper`: the pitch's `midfielder` on the dark theme, darkened
    /// to read at 4.5 on the light one. The pitch keeps its own on its
    /// badges.
    pub lineup_midfielder: egui::Color32,
    /// A forward's position text in the lineup lists, as `lineup_keeper`:
    /// the pitch's `forward` on the dark theme, darkened to read at 4.5 on
    /// the light one. The pitch keeps its own on its badges.
    pub lineup_forward: egui::Color32,
    /// The corner brackets around the open popover's marker and a swap's
    /// target (`#14fbff`), the screen's selection mark.
    pub selected_corners: egui::Color32,
    /// The corner brackets on every marker an armed assignment mode can take
    /// (`#ff4e7f`).
    pub candidate_corners: egui::Color32,
    /// The pitch's own text: the formation's name and a starter's overall,
    /// white as the game's, a name's colour where its codes give none, and a
    /// position past the table's.
    pub pitch_text: egui::Color32,
    /// The outline under Live AATF's pitch cross, so the cross stays seen over
    /// a portrait of its own colour.
    pub cross_outline: egui::Color32,
    /// The captain mark's box over his portrait.
    pub captain_fill: egui::Color32,
    /// The captain mark's `C`.
    pub captain_text: egui::Color32,
    /// The pitch's darkening outside the areas a held starter is rated at:
    /// black at 60%, which leaves the turf at 40%.
    pub lit_dim: egui::Color32,
    /// A gold medal's badge outline, the game's gold name colour.
    pub medal_gold: egui::Color32,
    /// A silver medal's badge outline, its metal's colour.
    pub medal_silver: egui::Color32,
    /// A bronze medal's badge outline, darkened so a thin outline does not
    /// read as gold.
    pub medal_bronze: egui::Color32,
    /// The colour picker's marks on the colour field itself: the ring at the
    /// working colour's point and the bar at its hue.
    pub picker_marker: egui::Color32,
    /// The edge around the colour picker's marks, so they show on a light
    /// colour as on a dark one.
    pub picker_marker_edge: egui::Color32,
}

impl Palette {
    /// The dark theme's colours, the default look.
    pub const DARK: Palette = Palette {
        success: egui::Color32::from_rgb(0x4c, 0xd9, 0x64),
        readout: egui::Color32::from_gray(0xa0),
        count_grey: egui::Color32::from_rgb(0x5a, 0x5a, 0x5a),
        rating_c_fill: egui::Color32::from_rgb(0x5a, 0x5a, 0x5a),
        rating_a_fill: egui::Color32::from_rgb(0x12, 0x96, 0x48),
        rating_a_text: egui::Color32::WHITE,
        swatch_edge: egui::Color32::BLACK,
        turf_dark_band: egui::Color32::from_rgb(0x2d, 0x57, 0x1d),
        turf_light_band: egui::Color32::from_rgb(0x35, 0x60, 0x2a),
        pitch_lines: egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 51),
        badge_fill: egui::Color32::from_rgb(0x24, 0x2b, 0x2d),
        join_fill: egui::Color32::from_rgb(0x00, 0x5c, 0x80),
        keeper: egui::Color32::from_rgb(0xff, 0xde, 0x00),
        defender: egui::Color32::from_rgb(0x00, 0xd2, 0xff),
        midfielder: egui::Color32::from_rgb(0x57, 0xe4, 0x2b),
        forward: egui::Color32::from_rgb(0xff, 0x2c, 0x77),
        lineup_keeper: egui::Color32::from_rgb(0xff, 0xde, 0x00),
        lineup_defender: egui::Color32::from_rgb(0x00, 0xd2, 0xff),
        lineup_midfielder: egui::Color32::from_rgb(0x57, 0xe4, 0x2b),
        lineup_forward: egui::Color32::from_rgb(0xff, 0x2c, 0x77),
        selected_corners: egui::Color32::from_rgb(0x14, 0xfb, 0xff),
        candidate_corners: egui::Color32::from_rgb(0xff, 0x4e, 0x7f),
        pitch_text: egui::Color32::WHITE,
        cross_outline: egui::Color32::BLACK,
        captain_fill: egui::Color32::WHITE,
        captain_text: egui::Color32::BLACK,
        lit_dim: egui::Color32::from_black_alpha(153),
        medal_gold: egui::Color32::from_rgb(0xcc, 0x99, 0x00),
        medal_silver: egui::Color32::from_rgb(0xc0, 0xc0, 0xc0),
        medal_bronze: egui::Color32::from_rgb(0x8c, 0x4a, 0x20),
        picker_marker: egui::Color32::WHITE,
        picker_marker_edge: egui::Color32::BLACK,
    };

    /// The light theme's colours. Only the ones drawn on the theme's own panel
    /// differ from the dark ones; the rest sit on a surface both themes share.
    /// Each is written out rather than copied from `DARK`, so a colour added
    /// to the struct is a compile error here until it has a light value.
    pub const LIGHT: Palette = Palette {
        success: egui::Color32::from_rgb(0x13, 0x73, 0x33),
        readout: egui::Color32::from_gray(0x5f),
        count_grey: egui::Color32::from_gray(0x9f),
        rating_c_fill: egui::Color32::from_gray(0x9f),
        rating_a_fill: egui::Color32::from_rgb(0x12, 0x96, 0x48),
        rating_a_text: egui::Color32::WHITE,
        swatch_edge: egui::Color32::BLACK,
        turf_dark_band: egui::Color32::from_rgb(0x2d, 0x57, 0x1d),
        turf_light_band: egui::Color32::from_rgb(0x35, 0x60, 0x2a),
        pitch_lines: egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 51),
        badge_fill: egui::Color32::from_rgb(0x24, 0x2b, 0x2d),
        join_fill: egui::Color32::from_rgb(0x00, 0x5c, 0x80),
        keeper: egui::Color32::from_rgb(0xff, 0xde, 0x00),
        defender: egui::Color32::from_rgb(0x00, 0xd2, 0xff),
        midfielder: egui::Color32::from_rgb(0x57, 0xe4, 0x2b),
        forward: egui::Color32::from_rgb(0xff, 0x2c, 0x77),
        lineup_keeper: egui::Color32::from_rgb(0x84, 0x72, 0x00),
        lineup_defender: egui::Color32::from_rgb(0x00, 0x7d, 0x99),
        lineup_midfielder: egui::Color32::from_rgb(0x2f, 0x83, 0x14),
        lineup_forward: egui::Color32::from_rgb(0xd9, 0x24, 0x64),
        selected_corners: egui::Color32::from_rgb(0x14, 0xfb, 0xff),
        candidate_corners: egui::Color32::from_rgb(0xff, 0x4e, 0x7f),
        pitch_text: egui::Color32::WHITE,
        cross_outline: egui::Color32::BLACK,
        captain_fill: egui::Color32::WHITE,
        captain_text: egui::Color32::BLACK,
        lit_dim: egui::Color32::from_black_alpha(153),
        medal_gold: egui::Color32::from_rgb(0xcc, 0x99, 0x00),
        medal_silver: egui::Color32::from_rgb(0xc0, 0xc0, 0xc0),
        medal_bronze: egui::Color32::from_rgb(0x8c, 0x4a, 0x20),
        picker_marker: egui::Color32::WHITE,
        picker_marker_edge: egui::Color32::BLACK,
    };
}

/// The palette of the theme `visuals` draw: [`Palette::LIGHT`] on a light
/// theme, else [`Palette::DARK`].
pub fn palette(visuals: &egui::Visuals) -> &'static Palette {
    if visuals.dark_mode {
        &Palette::DARK
    } else {
        &Palette::LIGHT
    }
}

/// The tint that draws an image in its own colours (egui multiplies an
/// image's texels by it). Not a palette field: it is the same whatever the
/// theme, and nothing is drawn in it.
pub const NO_TINT: egui::Color32 = egui::Color32::WHITE;

/// The text style a modal or floating window's title draws in (the drafts'
/// `.window .title`): proportional 14, where `Heading`, a page's heading
/// (the drafts' `h2`), is 15. `install` registers it on both themes; draw a
/// title with [`window_title`] or [`window_title_text`].
fn window_title_style() -> egui::TextStyle {
    egui::TextStyle::Name("window_title".into())
}

/// `text` as a window's title, the drafts' `.window .title`: in
/// `window_title_style`'s 14 points, the draft's `var(--strong)` the strong
/// text colour. An `egui::Window` takes it as its title, which its title
/// bar would otherwise draw in `Heading`; a modal draws it with
/// [`window_title`].
pub fn window_title_text(text: impl Into<egui::RichText>) -> egui::RichText {
    text.into().strong().text_style(window_title_style())
}

/// Draws `text` as a modal window's title, [`window_title_text`] in a
/// label. A page's heading is `ui.heading`'s, `Heading`'s 15.
pub fn window_title(ui: &mut egui::Ui, text: impl Into<egui::RichText>) -> egui::Response {
    ui.label(window_title_text(text))
}

/// The colour a notice's text is drawn in: its kind's bright colour, since a
/// notice is seen, not found.
pub fn notice_color(kind: NoticeKind, visuals: &egui::Visuals) -> egui::Color32 {
    match kind {
        NoticeKind::Success => palette(visuals).success,
        NoticeKind::Info => visuals.hyperlink_color,
        NoticeKind::Warning => visuals.warn_fg_color,
    }
}

/// Applies the common `theme` setting to `ctx` through `Context::set_theme`: the setting,
/// never the system's theme, decides, so a member on a light desktop still gets the dark
/// default.
pub(crate) fn apply(ctx: &egui::Context, theme: Theme) {
    ctx.set_theme(match theme {
        Theme::Dark => egui::ThemePreference::Dark,
        Theme::Light => egui::ThemePreference::Light,
    });
}

/// Installs the suite's look on `ctx`: the fonts (`install_fonts`), the
/// drafts' style and the passes a frame may lay out in. `run_gui` calls it
/// at window creation and every view test's harness calls it too, so a
/// size a test measures is the size a member sees.
/// The spacing and both themes' widget states are egui 0.36's (the light
/// fills aside, `widgets_light`), the look the drafts the GUI is built from
/// were reviewed in: 0.37 moved them (the button
/// padding to 8 by 2, the controls to 20 high, the dark buttons to grey 50
/// with text at 215), so `install` pins them on both themes itself and an egui
/// upgrade cannot move the approved look.
#[cfg(not(target_arch = "wasm32"))]
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    // Three passes a frame at most, not egui's two: a new `Grid` discards the
    // first for its own sizing, so a view that lays out from the widths a
    // pass measured (the Save editor's centred player columns) needs a third
    // to discard the second, which was laid out on the sizing pass's widths.
    ctx.options_mut(|options| {
        options.max_passes = std::num::NonZeroUsize::new(3).expect("3 is not zero");
    });
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.style_mut_of(theme, |style| {
            style.spacing.button_padding = egui::vec2(4.0, 1.0);
            style.spacing.interact_size = egui::vec2(40.0, 18.0);
            // The drafts' text sizes (the stylesheet's `.small`, body, `.mono`,
            // `h2` and `.window .title`), where egui defaults to 9, 13, 13, 13
            // and 18: at those every approved width and row moves.
            for (text_style, font_id) in [
                (egui::TextStyle::Small, egui::FontId::proportional(11.0)),
                (egui::TextStyle::Body, egui::FontId::proportional(12.5)),
                (egui::TextStyle::Button, egui::FontId::proportional(12.5)),
                (egui::TextStyle::Monospace, egui::FontId::monospace(12.0)),
                (egui::TextStyle::Heading, egui::FontId::proportional(15.0)),
                (window_title_style(), egui::FontId::proportional(14.0)),
            ] {
                style.text_styles.insert(text_style, font_id);
            }
            style.visuals.widgets = match theme {
                egui::Theme::Dark => widgets_dark(),
                egui::Theme::Light => widgets_light(),
            };
        });
    }
}

/// egui 0.36.1's `Widgets::dark()`, field for field: the drafts' stylesheet
/// draws widgets in these colors, and 0.37's `Widgets::dark()` moved them (the
/// buttons to grey 50 with text at 215, the radii to 4 to 6 points), so it
/// cannot supply them.
#[cfg(not(target_arch = "wasm32"))]
fn widgets_dark() -> egui::style::Widgets {
    egui::style::Widgets {
        noninteractive: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(27),
            bg_fill: egui::Color32::from_gray(27),
            bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            fg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(140)),
            corner_radius: egui::CornerRadius::same(2),
            expansion: 0.0,
        },
        inactive: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(60),
            bg_fill: egui::Color32::from_gray(60),
            bg_stroke: egui::Stroke::default(),
            fg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
            corner_radius: egui::CornerRadius::same(2),
            expansion: 0.0,
        },
        hovered: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(70),
            bg_fill: egui::Color32::from_gray(70),
            bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(150)),
            fg_stroke: egui::Stroke::new(1.5, egui::Color32::from_gray(240)),
            corner_radius: egui::CornerRadius::same(3),
            expansion: 0.0,
        },
        active: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(55),
            bg_fill: egui::Color32::from_gray(55),
            bg_stroke: egui::Stroke::new(1.0, egui::Color32::WHITE),
            fg_stroke: egui::Stroke::new(2.0, egui::Color32::WHITE),
            corner_radius: egui::CornerRadius::same(2),
            expansion: 0.0,
        },
        open: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(45),
            bg_fill: egui::Color32::from_gray(27),
            bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            fg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(210)),
            corner_radius: egui::CornerRadius::same(2),
            expansion: 0.0,
        },
    }
}

/// egui 0.36.1's `Widgets::light()`, field for field but for the fills, for
/// the same reason as `widgets_dark`: 0.37's `Widgets::light()` rounded every
/// state to 6 points, which is not the look the drafts were reviewed in.
/// Each fill keeps the dark state's contrast with its panel instead of
/// 0.36's grey, whose resting 230 reads at 1.18 on the 248 panel, where the
/// dark 60 reads at 1.56 on 27, and left a resting checkbox, radio button or
/// number box all but invisible.
#[cfg(not(target_arch = "wasm32"))]
fn widgets_light() -> egui::style::Widgets {
    egui::style::Widgets {
        noninteractive: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(248),
            bg_fill: egui::Color32::from_gray(248),
            bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(190)),
            fg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            corner_radius: egui::CornerRadius::same(2),
            expansion: 0.0,
        },
        inactive: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(201),
            bg_fill: egui::Color32::from_gray(201),
            bg_stroke: egui::Stroke::default(),
            fg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            corner_radius: egui::CornerRadius::same(2),
            expansion: 0.0,
        },
        hovered: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(186),
            bg_fill: egui::Color32::from_gray(186),
            bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(105)),
            fg_stroke: egui::Stroke::new(1.5, egui::Color32::BLACK),
            corner_radius: egui::CornerRadius::same(3),
            expansion: 0.0,
        },
        active: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(208),
            bg_fill: egui::Color32::from_gray(208),
            bg_stroke: egui::Stroke::new(1.0, egui::Color32::BLACK),
            fg_stroke: egui::Stroke::new(2.0, egui::Color32::BLACK),
            corner_radius: egui::CornerRadius::same(2),
            expansion: 0.0,
        },
        open: egui::style::WidgetVisuals {
            weak_bg_fill: egui::Color32::from_gray(223),
            bg_fill: egui::Color32::from_gray(220),
            bg_stroke: egui::Stroke::new(1.0, egui::Color32::from_gray(160)),
            fg_stroke: egui::Stroke::new(1.0, egui::Color32::BLACK),
            corner_radius: egui::CornerRadius::same(2),
            expansion: 0.0,
        },
    }
}

/// Registers the suite's fonts on `ctx`: proportional text resolves from
/// Roboto Flex, the symbols font, the icon font and `egui-icons`, in that
/// order, and monospace from egui's Hack, then the same four. The body font
/// `"Roboto Flex"` (the file `resources/fonts/RobotoFlex-Regular.ttf`
/// bundles) is first because a member's text must resolve from it before
/// any other font. The symbols font `"symbols"`
/// (`resources/fonts/symbols.ttf`: the →, ✔, ⏷ and the like the views draw
/// that Roboto Flex lacks) follows it, so the body font keeps every
/// character it has. The symbols are a file of their own, not glyphs of the
/// icon font, because that file's vertical metrics are the body font's: egui
/// centres a later font's line height on the body font's, so in the icon
/// font's metrics a symbol would sit off the text's baseline. The icon font
/// `"icons"` (the subset `resources/fonts/phosphor-subset.ttf` ships;
/// `crate::icons` names its glyphs) comes ahead of egui's own icons, so the
/// icon codepoints come from it, and is its own `icons` family too. The
/// `label` family is `"Roboto Flex Semibold"`
/// (`resources/fonts/RobotoFlex-Semibold.ttf`), then the proportional chain:
/// the Tactics pitch's formation name, the one text the suite draws in a
/// heavier weight, to match the game's stroke. Every build that has
/// `studio_core` has egui's default fonts, through its `eframe` dependency's
/// `default_fonts`. Installed fonts resolve
/// before the system fallback eframe registers (`Context::set_fonts` replaces the
/// definitions only, not the context's font providers); `set_fonts` replaces the
/// whole font set, so the definitions start from egui's defaults and add ours.
/// Native only for now: Studio Web's frontend calls it when that exists.
#[cfg(not(target_arch = "wasm32"))]
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Roboto Flex".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../resources/fonts/RobotoFlex-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "icons".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../resources/fonts/phosphor-subset.ttf"
        ))),
    );
    fonts.font_data.insert(
        "symbols".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../resources/fonts/symbols.ttf"
        ))),
    );
    // The body font, then the symbols and the icons, take Ubuntu-Light's place
    // in every family, not only the proportional one: monospace falls back to
    // them for what Hack lacks, and a family naming a `font_data` key that is
    // not there panics in `fonts_for_family`.
    fonts.font_data.remove("Ubuntu-Light");
    for names in fonts.families.values_mut() {
        if let Some(light) = names.iter().position(|name| name == "Ubuntu-Light") {
            names.splice(
                light..=light,
                ["Roboto Flex", "symbols", "icons"].map(str::to_owned),
            );
        }
    }
    fonts.families.insert(
        egui::FontFamily::Name("icons".into()),
        vec!["icons".to_owned()],
    );
    fonts.font_data.insert(
        "Roboto Flex Semibold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../resources/fonts/RobotoFlex-Semibold.ttf"
        ))),
    );
    // The semibold first, then the body text's own chain for what it lacks.
    let mut label = vec!["Roboto Flex Semibold".to_owned()];
    label.extend_from_slice(
        fonts
            .families
            .get(&egui::FontFamily::Proportional)
            .expect("egui's default fonts define the proportional family"),
    );
    fonts
        .families
        .insert(egui::FontFamily::Name("label".into()), label);
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_notice_kind_gets_its_own_bright_colour() {
        for (visuals, success) in [
            (egui::Visuals::dark(), Palette::DARK.success),
            (egui::Visuals::light(), Palette::LIGHT.success),
        ] {
            assert_eq!(notice_color(NoticeKind::Success, &visuals), success);
            assert_eq!(
                notice_color(NoticeKind::Info, &visuals),
                visuals.hyperlink_color
            );
            assert_eq!(
                notice_color(NoticeKind::Warning, &visuals),
                visuals.warn_fg_color
            );
        }
    }

    // Compared field by field, not by address: two references to one constant
    // need not point at the same place.
    #[test]
    fn the_palette_follows_the_visuals_theme() {
        assert_eq!(
            named_fields(palette(&egui::Visuals::light())),
            named_fields(&Palette::LIGHT),
            "the light visuals draw the light palette"
        );
        assert_eq!(
            named_fields(palette(&egui::Visuals::dark())),
            named_fields(&Palette::DARK),
            "the dark visuals draw the dark palette"
        );
    }

    /// `palette`'s fields by name. The destructuring names every field, so a
    /// field added to the struct does not compile here until it is listed.
    fn named_fields(palette: &Palette) -> Vec<(&'static str, egui::Color32)> {
        let Palette {
            success,
            readout,
            count_grey,
            rating_c_fill,
            rating_a_fill,
            rating_a_text,
            swatch_edge,
            turf_dark_band,
            turf_light_band,
            pitch_lines,
            badge_fill,
            join_fill,
            keeper,
            defender,
            midfielder,
            forward,
            lineup_keeper,
            lineup_defender,
            lineup_midfielder,
            lineup_forward,
            selected_corners,
            candidate_corners,
            pitch_text,
            cross_outline,
            captain_fill,
            captain_text,
            lit_dim,
            medal_gold,
            medal_silver,
            medal_bronze,
            picker_marker,
            picker_marker_edge,
        } = palette;
        vec![
            ("success", *success),
            ("readout", *readout),
            ("count_grey", *count_grey),
            ("rating_c_fill", *rating_c_fill),
            ("rating_a_fill", *rating_a_fill),
            ("rating_a_text", *rating_a_text),
            ("swatch_edge", *swatch_edge),
            ("turf_dark_band", *turf_dark_band),
            ("turf_light_band", *turf_light_band),
            ("pitch_lines", *pitch_lines),
            ("badge_fill", *badge_fill),
            ("join_fill", *join_fill),
            ("keeper", *keeper),
            ("defender", *defender),
            ("midfielder", *midfielder),
            ("forward", *forward),
            ("lineup_keeper", *lineup_keeper),
            ("lineup_defender", *lineup_defender),
            ("lineup_midfielder", *lineup_midfielder),
            ("lineup_forward", *lineup_forward),
            ("selected_corners", *selected_corners),
            ("candidate_corners", *candidate_corners),
            ("pitch_text", *pitch_text),
            ("cross_outline", *cross_outline),
            ("captain_fill", *captain_fill),
            ("captain_text", *captain_text),
            ("lit_dim", *lit_dim),
            ("medal_gold", *medal_gold),
            ("medal_silver", *medal_silver),
            ("medal_bronze", *medal_bronze),
            ("picker_marker", *picker_marker),
            ("picker_marker_edge", *picker_marker_edge),
        ]
    }

    // Only the colours drawn on the theme's own panel have a light value of
    // their own; every other is on a surface both themes share.
    #[test]
    fn only_the_panel_colours_differ_between_the_themes() {
        const THEMED: [&str; 8] = [
            "success",
            "readout",
            "count_grey",
            "rating_c_fill",
            "lineup_keeper",
            "lineup_defender",
            "lineup_midfielder",
            "lineup_forward",
        ];
        for ((name, dark), (_, light)) in named_fields(&Palette::DARK)
            .into_iter()
            .zip(named_fields(&Palette::LIGHT))
        {
            if THEMED.contains(&name) {
                assert_ne!(dark, light, "{name} has a light value of its own");
            } else {
                assert_eq!(dark, light, "{name} is the same in both themes");
            }
        }
    }

    #[test]
    fn install_pins_egui_0_36s_spacing_and_widget_states() {
        let ctx = egui::Context::default();
        install(&ctx);
        for (theme, inactive_fill, inactive_text) in
            [(egui::Theme::Dark, 60, 180), (egui::Theme::Light, 201, 60)]
        {
            let style = ctx.style_of(theme);
            assert_eq!(style.spacing.button_padding, egui::vec2(4.0, 1.0));
            assert_eq!(style.spacing.interact_size, egui::vec2(40.0, 18.0));
            let widgets = &style.visuals.widgets;
            assert_eq!(
                widgets.inactive.weak_bg_fill,
                egui::Color32::from_gray(inactive_fill)
            );
            assert_eq!(
                widgets.inactive.fg_stroke.color,
                egui::Color32::from_gray(inactive_text)
            );
            assert_eq!(widgets.hovered.corner_radius, egui::CornerRadius::same(3));
        }
    }

    /// The WCAG contrast ratio of two opaque colours: the lighter one's
    /// relative luminance plus 0.05 over the darker one's plus 0.05.
    fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
        let luminance = |colour: egui::Color32| {
            let linear = |channel: u8| {
                let srgb = f64::from(channel) / 255.0;
                if srgb <= 0.04045 {
                    srgb / 12.92
                } else {
                    ((srgb + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(colour.r()) + 0.7152 * linear(colour.g()) + 0.0722 * linear(colour.b())
        };
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Each widget state's fills by name. The open state's `bg_fill` is left
    /// out: the open combo box and menu button draw as buttons, which fill
    /// with `weak_bg_fill`, so it keeps egui's light grey.
    fn state_fills(widgets: &egui::style::Widgets) -> Vec<(&'static str, egui::Color32)> {
        vec![
            ("noninteractive weak", widgets.noninteractive.weak_bg_fill),
            ("noninteractive", widgets.noninteractive.bg_fill),
            ("inactive weak", widgets.inactive.weak_bg_fill),
            ("inactive", widgets.inactive.bg_fill),
            ("hovered weak", widgets.hovered.weak_bg_fill),
            ("hovered", widgets.hovered.bg_fill),
            ("active weak", widgets.active.weak_bg_fill),
            ("active", widgets.active.bg_fill),
            ("open weak", widgets.open.weak_bg_fill),
        ]
    }

    // A light widget state's fill stands out from the light panel as the
    // dark state's does from the dark panel: a resting checkbox, radio
    // button or number box is seen on both themes.
    #[test]
    fn the_light_widget_fills_keep_the_dark_ones_contrast_with_the_panel() {
        let ctx = egui::Context::default();
        install(&ctx);
        let (dark, light) = (
            ctx.style_of(egui::Theme::Dark),
            ctx.style_of(egui::Theme::Light),
        );
        for ((name, dark_fill), (_, light_fill)) in state_fills(&dark.visuals.widgets)
            .into_iter()
            .zip(state_fills(&light.visuals.widgets))
        {
            let dark_contrast = contrast(dark_fill, dark.visuals.panel_fill);
            let light_contrast = contrast(light_fill, light.visuals.panel_fill);
            assert!(
                (light_contrast - dark_contrast).abs() <= 0.05,
                "{name}: the light fill {light_fill:?} reads at {light_contrast:.2} on the light \
                 panel, the dark one at {dark_contrast:.2} on the dark panel"
            );
        }
    }

    // The lineup lists' position colours: the pitch's on the dark theme; on
    // the light one each reads at 4.5 on the light panel (grey 248) and no
    // darker than it needs to, so it keeps its hue.
    #[test]
    fn the_lineup_colours_read_at_4_5_on_the_light_panel() {
        let panel = egui::Visuals::light().panel_fill;
        assert_eq!(
            panel,
            egui::Color32::from_gray(248),
            "premise: the light panel"
        );
        let (dark, light) = (&Palette::DARK, &Palette::LIGHT);
        for (name, dark_lineup, pitch, light_lineup) in [
            (
                "keeper",
                dark.lineup_keeper,
                dark.keeper,
                light.lineup_keeper,
            ),
            (
                "defender",
                dark.lineup_defender,
                dark.defender,
                light.lineup_defender,
            ),
            (
                "midfielder",
                dark.lineup_midfielder,
                dark.midfielder,
                light.lineup_midfielder,
            ),
            (
                "forward",
                dark.lineup_forward,
                dark.forward,
                light.lineup_forward,
            ),
        ] {
            assert_eq!(dark_lineup, pitch, "the dark {name} is the pitch's");
            let light_contrast = contrast(light_lineup, panel);
            assert!(
                (4.5..4.7).contains(&light_contrast),
                "the light {name} {light_lineup:?} reads at {light_contrast:.3} on the light panel"
            );
        }
    }

    #[test]
    fn install_pins_the_drafts_text_sizes() {
        let ctx = egui::Context::default();
        install(&ctx);
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let style = ctx.style_of(theme);
            for (text_style, font_id) in [
                (egui::TextStyle::Small, egui::FontId::proportional(11.0)),
                (egui::TextStyle::Body, egui::FontId::proportional(12.5)),
                (egui::TextStyle::Button, egui::FontId::proportional(12.5)),
                (egui::TextStyle::Monospace, egui::FontId::monospace(12.0)),
                (egui::TextStyle::Heading, egui::FontId::proportional(15.0)),
                (
                    egui::TextStyle::Name("window_title".into()),
                    egui::FontId::proportional(14.0),
                ),
            ] {
                assert_eq!(
                    style.text_styles.get(&text_style),
                    Some(&font_id),
                    "{theme:?} {text_style:?}"
                );
            }
        }
    }

    #[test]
    fn installing_fonts_chains_roboto_flex_the_symbols_and_the_icons() {
        let ctx = egui::Context::default();
        install(&ctx);
        // `Context::fonts` is populated by the first pass.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
        ctx.fonts(|fonts| {
            let definitions = fonts.definitions();
            for name in ["Roboto Flex", "symbols", "icons"] {
                assert!(definitions.font_data.contains_key(name), "{name} bundled");
            }
            for name in ["Ubuntu-Light", "Ubuntu-Regular"] {
                assert!(
                    !definitions.font_data.contains_key(name),
                    "{name} is out: the body font is Roboto Flex"
                );
            }
            let proportional = definitions
                .families
                .get(&egui::FontFamily::Proportional)
                .expect("the proportional family exists");
            assert_eq!(
                proportional,
                &["Roboto Flex", "symbols", "icons", "egui-icons"],
                "the body font resolves text first, then the symbols, the icons, egui's own"
            );
            let monospace = definitions
                .families
                .get(&egui::FontFamily::Monospace)
                .expect("the monospace family exists");
            assert_eq!(
                monospace,
                &["Hack", "Roboto Flex", "symbols", "icons", "egui-icons"],
                "monospace keeps Hack first, then the proportional chain"
            );
            assert_eq!(
                definitions
                    .families
                    .get(&egui::FontFamily::Name("icons".into())),
                Some(&vec!["icons".to_owned()]),
                "icons is its own family too"
            );
            assert!(
                definitions.font_data.contains_key("Roboto Flex Semibold"),
                "Roboto Flex Semibold bundled"
            );
            assert_eq!(
                definitions
                    .families
                    .get(&egui::FontFamily::Name("label".into())),
                Some(&vec![
                    "Roboto Flex Semibold".to_owned(),
                    "Roboto Flex".to_owned(),
                    "symbols".to_owned(),
                    "icons".to_owned(),
                    "egui-icons".to_owned(),
                ]),
                "the label family is the semibold, then the proportional chain"
            );
        });
    }

    #[test]
    fn every_icon_constant_resolves_a_glyph_after_install() {
        const ALL: &[&str] = &[
            crate::icons::ARROW_FAT_UP,
            crate::icons::BUILDINGS,
            crate::icons::CROSSHAIR,
            crate::icons::DATABASE,
            crate::icons::DOWNLOAD_SIMPLE,
            crate::icons::FLAG,
            crate::icons::GEAR,
            crate::icons::MUSIC_NOTE,
            crate::icons::MUSIC_NOTES,
            crate::icons::NOTE_PENCIL,
            crate::icons::PLUS_CIRCLE,
            crate::icons::QUESTION,
            crate::icons::SCROLL,
            crate::icons::SMILEY,
            crate::icons::SOCCER_BALL,
            crate::icons::T_SHIRT,
            crate::icons::USERS_THREE,
            crate::icons::WARNING,
            crate::icons::X,
        ];
        let ctx = egui::Context::default();
        install(&ctx);
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
        ctx.fonts_mut(|fonts| {
            for name in ALL {
                let glyph = name.chars().next().expect("a constant is one glyph");
                assert!(
                    fonts.has_glyph(&egui::FontId::proportional(14.0), glyph),
                    "no font has U+{:04X}",
                    u32::from(glyph)
                );
            }
        });
    }

    #[test]
    fn every_symbol_the_views_draw_resolves_a_glyph_after_install() {
        const SYMBOLS: &str = "→↓↻∉−≠≡≤ⓘ⏴⏵⏷▶▸▾●⚠✔✕✖⠇…›•";
        let ctx = egui::Context::default();
        install(&ctx);
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
        ctx.fonts_mut(|fonts| {
            for font_id in [
                egui::FontId::proportional(14.0),
                egui::FontId::monospace(14.0),
            ] {
                let missing: Vec<char> = SYMBOLS
                    .chars()
                    .filter(|&glyph| !fonts.has_glyph(&font_id, glyph))
                    .collect();
                assert!(
                    missing.is_empty(),
                    "no bundled {:?} font has {}",
                    font_id.family,
                    missing
                        .iter()
                        .map(|glyph| format!("U+{:04X} ({glyph})", u32::from(*glyph)))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        });
    }
}
