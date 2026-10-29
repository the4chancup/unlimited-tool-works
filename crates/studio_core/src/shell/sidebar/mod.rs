//! The sidebar, a 56-point rail that opens the full sidebar over the tool's left
//! edge when the pointer rests on it, as the draft `docs/prototypes/index.html` draws
//! it: the logo doubling as the About click target, the
//! PES version selector with its live exe check, its blocker and the once-per-life
//! auto-switch to a running PES, the tool list with the planned tools grayed, and
//! the help `?` with the settings gear pinned at the bottom. There is no collapse
//! button and no expanded state to persist: the widgets are drawn once per frame,
//! in the rail while the overlay is closed, in the overlay while it is open.

use std::path::{Path, PathBuf};
use std::time::Duration;

use pes_version::PesVersion;

use crate::help::HelpTarget;
use crate::pes_process::RunningPes;
use crate::settings::CommonSettings;
use crate::status::{ActionEffect, Notice, NoticeAction, NoticeKind};
use crate::tool::{PlannedTool, ShellRequest, StudioTool, ToolContext};
use crate::widgets::{step_choice, wheel_steps};

#[cfg(test)]
mod tests;

use super::about;

/// The rail's width: the shell's `Panel::left` exact size. The rail is always there,
/// icons only.
pub(super) const RAIL_WIDTH: f32 = 56.0;
/// The rail's inner margin on every side, the draft's `.rail` padding 6. egui's side
/// panel margin (8 across, 2 down) would start the logo 4 points high and narrow the
/// selector and the lines by 4.
const RAIL_PADDING: i8 = 6;
/// The tool icons', the help's and the gear's point size: about twice the body text, so
/// the rail alone tells the tools apart.
const ICON_SIZE: f32 = 24.0;
/// One entry row: the 24-point icon and its padding. With egui's item spacing of 3
/// between them, fourteen rows take 459 points of the 756-point rail at the default
/// window, leaving the logo, the version dropdown, the help and the gear their room.
const ENTRY_HEIGHT: f32 = 30.0;
/// The height a rail line takes: its 1-point stroke with the draft's `.sidebar .sep`
/// margin 4 above and below it.
const SEPARATOR_SPACING: f32 = 9.0;
/// The side of the logo's square, the draft's `.logo` 32 square, and so its row's
/// height.
const LOGO_SIZE: f32 = 32.0;
/// The 4cc clover's ring, a private-use glyph of the symbols font. It is one em
/// across and centred on the line, so the font size is the clover's diameter.
const CLOVER_RING: &str = "\u{F100}";
/// The 4cc clover's four leaves, the symbols font's glyph drawn on the ring's place
/// in a green of its own: one glyph takes one colour.
const CLOVER_LEAVES: &str = "\u{F101}";
/// The clover's diameter, the draft's `.clover` 28 inside the logo's 32 square.
const CLOVER_SIZE: f32 = 28.0;
/// Seconds the pointer rests on the rail before the overlay opens: a short delay.
const HOVER_DELAY: f64 = 0.15;
/// The accent bar on the active entry's left edge (draft: 3 px).
const ACTIVE_BAR_WIDTH: f32 = 3.0;
/// The radius of the accent bar's two left corners: the draft entry's 2 px
/// corner radius, which its left border follows. The right corners stay square
/// against the row's fill.
const ACTIVE_BAR_CORNER: u8 = 2;
/// Seconds between running-PES polls of the auto-switch.
const PES_POLL_SECONDS: f64 = 5.0;

/// What one frame of the sidebar draws from: the registered tools in sidebar
/// order, the active one's index, the planned entries, the CLI-only ids and the
/// tools' context.
struct SidebarFrame<'a> {
    tools: &'a [Box<dyn StudioTool>],
    active: usize,
    planned: &'a [PlannedTool],
    cli_only: &'a [&'static str],
    ctx: &'a ToolContext,
}

/// The overlay's geometry for one frame: the outer rect the `Area` takes, the
/// `egui::Frame` it draws with, and the content size inside it.
struct OverlayGeom {
    outer: egui::Rect,
    frame: egui::Frame,
    inner: egui::Vec2,
}

/// What a click in the sidebar asks the shell to do.
pub(super) enum SidebarClick {
    /// Make the tool at this index the active view.
    SwitchTool(usize),
    /// Open the settings menu.
    OpenSettings,
    /// Open the help window at the target (the `?` targets the active tool's chapter,
    /// the About popup's Help link `setting-up`).
    OpenHelp(HelpTarget),
}

/// The sidebar's state across frames: whether the overlay is open, when the
/// pointer started resting on the rail, the sidebar widget last holding focus,
/// the version combo's popup id, the exe check's last verdict, the window's
/// last focus (the check re-runs when focus returns), the logo's About popup
/// and the once-per-life auto-switch to a running PES.
pub(super) struct Sidebar {
    /// The overlay is drawn while set.
    open: bool,
    /// egui time the pointer started resting on the rail's rect; `None` while it
    /// is elsewhere (leaving restarts the wait).
    hover_since: Option<f64>,
    /// The label of the sidebar widget that held keyboard focus at the end of
    /// the last drawn frame, `None` while none did: the keyboard rule keeps the
    /// overlay open while one holds the focus a Tab gave it — a pointer click
    /// gives a widget no focus, so a click never keeps it open.
    focus_label: Option<&'static str>,
    /// The version combo's popup id (the same in the rail and the overlay): its
    /// open state holds the overlay open, and its layer is pinned above the
    /// overlay's each frame.
    combo_popup_id: Option<egui::Id>,
    /// What the last exe check ran against and found: the version, the resolved PES folder
    /// and the verdict. `None` until the first listing.
    pub(super) exe_check: Option<(PesVersion, PathBuf, ExeCheck)>,
    /// How many directory listings the check has run; the tests read it (the check never
    /// re-lists per frame).
    pub(super) exe_listings: u64,
    /// The window's focus last frame, for the gained-focus re-check.
    was_focused: bool,
    /// The logo's About popup.
    pub(super) about_open: bool,
    /// The auto-switch is once per program life: spent by the switch itself or by a manual
    /// pick, never by a blocker (ST-SIDE-05).
    pub(super) auto_switch_used: bool,
    /// The running-PES probe: `pes_process::running_pes` in the app, replaced by the tests.
    /// A plain `fn`, not a trait: there is one real probe.
    pub(super) probe: fn() -> Vec<RunningPes>,
    /// egui time of the next poll; `None` until `poll_running_pes` first runs.
    next_poll: Option<f64>,
}

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            open: false,
            hover_since: None,
            focus_label: None,
            combo_popup_id: None,
            exe_check: None,
            exe_listings: 0,
            was_focused: false,
            about_open: false,
            auto_switch_used: false,
            probe: crate::pes_process::running_pes,
            next_poll: None,
        }
    }
}

/// What the exe check's last directory listing found (ST-SIDE-03): the selector is red on
/// `OtherVersion`, orange on `NoExe` and `NoFolder`, uncolored on `Found`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExeCheck {
    /// The resolved PES folder holds the selected version's exe.
    Found,
    /// It exists but holds a different version's exe (wrong selection or wrong path).
    OtherVersion(PesVersion),
    /// It exists but holds no `PES20NN.exe` at all.
    NoExe,
    /// It does not exist (or cannot be listed).
    NoFolder,
}

impl Sidebar {
    /// Draws the sidebar and returns the click's ask, if any. `tools` are in sidebar
    /// order, `active` the index of the one on screen, `cli_only` the registered tools'
    /// ids drawn as planned entries.
    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        tools: &[Box<dyn StudioTool>],
        active: usize,
        planned: &[PlannedTool],
        cli_only: &[&'static str],
        ctx: &ToolContext,
    ) -> Option<SidebarClick> {
        let args = SidebarFrame {
            tools,
            active,
            planned,
            cli_only,
            ctx,
        };
        let egui_ctx = ui.ctx().clone();
        // `max_rect` is the panel's content rect; the clip is the panel's visible
        // outer rect, shrunk onto it by `egui::Panel`.
        let rail_content = ui.max_rect();
        let rail_outer = ui.clip_rect();
        // The icon cell is as wide as the rail's inner width, in both places, so
        // an icon sits at the same position in the rail and in the overlay.
        let cell = rail_content.width();
        let inner_width = cell + ui.spacing().item_spacing.x + widest_label(ui, &args);
        let frame = overlay_frame(ui.style());
        let geom = OverlayGeom {
            outer: egui::Rect::from_min_size(
                rail_outer.min,
                egui::vec2(
                    inner_width + frame.total_margin().sum().x,
                    rail_outer.height(),
                ),
            ),
            frame,
            inner: egui::vec2(inner_width, rail_content.height()),
        };

        // Open and close: the pointer resting on the rail for HOVER_DELAY opens it,
        // and a pointer that leaves restarts the wait. It stays open while the
        // pointer is inside the overlay's rect, while a popup it opened is open, or
        // under the keyboard rule, and closes on the first frame none of those holds.
        let now = egui_ctx.input(|input| input.time);
        let pointer = egui_ctx.input(|input| input.pointer.latest_pos());
        let on_rail = pointer.is_some_and(|pos| rail_outer.contains(pos));
        let on_overlay = pointer.is_some_and(|pos| geom.outer.contains(pos));
        let mut rested = false;
        if on_rail {
            let since = *self.hover_since.get_or_insert(now);
            let left = HOVER_DELAY - (now - since);
            if left <= 0.0 {
                rested = true;
            } else {
                // Open without further input: repaint when the delay runs out.
                egui_ctx.request_repaint_after(Duration::from_secs_f64(left));
            }
        } else {
            self.hover_since = None;
        }
        // `focus_label` is refreshed during the draw: last frame's value decides here.
        self.open = self.focus_label.is_some()
            || self.about_open
            || self
                .combo_popup_id
                .is_some_and(|id| egui::Popup::is_id_open(&egui_ctx, id))
            || rested
            || (self.open && on_overlay);

        let order = overlay_order(&egui_ctx);
        let clicked = if self.open {
            // The rail panel draws only its own frame: the widgets move into the
            // overlay, so the panel and the view keep their rects.
            self.show_overlay(&egui_ctx, order, geom, cell, &args)
        } else {
            self.contents(ui, cell, true, &args)
        };
        if let Some(popup_id) = self.combo_popup_id {
            // `move_to_top` sends the overlay to the front of the Foreground order
            // on a re-open or a click on it, which would drop it over the combo's
            // popup: `set_sublayer` splices the popup directly above it each frame.
            egui_ctx.set_sublayer(
                egui::LayerId::new(egui::Order::Foreground, overlay_id()),
                egui::LayerId::new(egui::Order::Foreground, popup_id),
            );
        }
        if self.about_open {
            let help = about::show(
                ui.ctx(),
                ctx.paths(),
                ctx.common().pes_version,
                &mut self.about_open,
            );
            if help {
                return Some(SidebarClick::OpenHelp(HelpTarget {
                    tool: "setting-up".to_owned(),
                    topic: None,
                    anchor: None,
                }));
            }
        }
        clicked
    }

    /// The overlay: the full sidebar as a floating layer over the view, at the
    /// rail's top-left and as tall as it, in `order` (`overlay_order`).
    fn show_overlay(
        &mut self,
        egui_ctx: &egui::Context,
        order: egui::Order,
        geom: OverlayGeom,
        cell: f32,
        args: &SidebarFrame,
    ) -> Option<SidebarClick> {
        let OverlayGeom {
            outer,
            frame,
            inner,
        } = geom;
        let layer = egui::LayerId::new(order, overlay_id());
        if egui_ctx.memory(|m| m.area_rect(overlay_id())).is_none() {
            // An `egui::Area`'s first `show` is an invisible sizing pass; seeding
            // its state draws the overlay already on the opening frame — the rail
            // must never flash empty for one frame.
            egui_ctx.memory_mut(|m| {
                m.areas_mut().set_state(
                    layer,
                    egui::AreaState {
                        pivot_pos: Some(outer.min),
                        pivot: egui::Align2::LEFT_TOP,
                        size: Some(outer.size()),
                        interactable: true,
                        last_became_visible_at: None,
                    },
                );
            });
        }
        egui::Area::new(overlay_id())
            .order(order)
            .fixed_pos(outer.min)
            // No fade-in: the rail's widgets leave on the opening frame, so a fading
            // overlay would flash a dim, half-empty sidebar on every open.
            .fade_in(false)
            .show(egui_ctx, |ui| {
                let shown = frame.show(ui, |ui| {
                    ui.set_min_size(inner);
                    self.contents(ui, cell, false, args)
                });
                let line = ui.visuals().widgets.noninteractive.bg_stroke;
                let rect = shown.response.rect;
                ui.painter()
                    .vline(rect.right() - line.width / 2.0, rect.y_range(), line);
                shown.inner
            })
            .inner
    }

    /// The items, top to bottom as the draft lays them out, drawn once per frame:
    /// in the rail's panel while the overlay is closed (`rail`: the icons alone,
    /// no labels) or in the overlay while it is open (the labels join the icons).
    /// The `scope_id` gives the contents one id root in both places
    /// (`UiBuilder::scope_id` makes the scope's id independent of the parent's),
    /// so a widget keeps its egui id across the move: a keyboard focus or an open
    /// version popup survives the overlay opening and closing. `cell` is the icon
    /// cell's width — the rail's inner width — in both places (item 3: "nothing
    /// moves when it opens").
    fn contents(
        &mut self,
        ui: &mut egui::Ui,
        cell: f32,
        rail: bool,
        args: &SidebarFrame,
    ) -> Option<SidebarClick> {
        let SidebarFrame {
            tools,
            active,
            cli_only,
            ..
        } = args;
        self.focus_label = None;
        ui.scope_builder(
            egui::UiBuilder::new().scope_id(egui::Id::unique("sidebar-contents")),
            |ui| {
                // The row width is measured before any widget's min size can
                // stretch the layout region (the combo's atom minimum is wider
                // than the rail's content): every row takes the same width — the
                // icon cell's in the rail, the overlay's inner width when open.
                let row_width = ui.available_width();
                // The logo doubles as the About click target, centered in the icon
                // cell in both places: it gets no label, so the hover opens nothing
                // that comes and goes (item 1).
                let logo = logo(ui, row_width, cell);
                logo.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "4cc"));
                if logo.has_focus() {
                    self.focus_label = Some("4cc");
                }
                if logo
                    .on_hover_text(format!("About {}", crate::APP_NAME))
                    .clicked()
                {
                    self.about_open = true;
                }
                self.version_selector(ui, args, rail);
                ui.add(egui::Separator::default().spacing(SEPARATOR_SPACING));

                let mut clicked = None;
                for (index, tool) in tools.iter().enumerate() {
                    // A registered tool marked CLI-only is no tool entry: it is drawn
                    // with the planned entries below (ST-SIDE-08).
                    if cli_only.contains(&tool.id()) {
                        continue;
                    }
                    let active_tool = index == *active;
                    let response = entry_row(
                        ui,
                        row_width,
                        cell,
                        egui::RichText::new(tool.icon()).size(ICON_SIZE),
                        (!rail).then_some(tool.label()),
                        active_tool,
                        true,
                    );
                    if response.has_focus() {
                        self.focus_label = Some(tool.label());
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::Role::Button,
                            ui.is_enabled(),
                            active_tool,
                            tool.label(),
                        )
                    });
                    if response.clicked() {
                        clicked = Some(SidebarClick::SwitchTool(index));
                    }
                }
                for entry in drawn_planned(args) {
                    // In the registry's order: a CLI-only tool's entry is here,
                    // its tooltip saying it runs from the command line in this
                    // version (ST-SIDE-08).
                    let tooltip = if cli_only.contains(&entry.id) {
                        format!(
                            "{} has no window in this version, so it runs from the command line.",
                            entry.label
                        )
                    } else {
                        format!("{} is not in this build", entry.label)
                    };
                    let response = ui
                        .add_enabled_ui(false, |ui| {
                            entry_row(
                                ui,
                                row_width,
                                cell,
                                egui::RichText::new(entry.icon).size(ICON_SIZE),
                                (!rail).then_some(entry.label),
                                false,
                                false,
                            )
                        })
                        .inner;
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Label, false, entry.label)
                    });
                    response.on_disabled_hover_text(tooltip);
                }

                // The bottom rows, pinned to the sidebar's foot: the help `?` above
                // the settings gear (the first bottom-up child is the lowest), under
                // a line like the one under the selector.
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    let gear = entry_row(
                        ui,
                        row_width,
                        cell,
                        egui::RichText::new(crate::icons::GEAR).size(ICON_SIZE),
                        (!rail).then_some("Settings"),
                        false,
                        true,
                    );
                    gear.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Button, true, "Settings")
                    });
                    if gear.has_focus() {
                        self.focus_label = Some("Settings");
                    }
                    if gear.clicked() {
                        clicked = Some(SidebarClick::OpenSettings);
                    }
                    let help = entry_row(
                        ui,
                        row_width,
                        cell,
                        egui::RichText::new(crate::icons::QUESTION).size(ICON_SIZE),
                        (!rail).then_some("Help"),
                        false,
                        true,
                    );
                    help.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Button, true, "Help")
                    });
                    if help.has_focus() {
                        self.focus_label = Some("Help");
                    }
                    if help.clicked()
                        && let Some(tool) = tools.get(*active)
                    {
                        clicked = Some(SidebarClick::OpenHelp(HelpTarget {
                            tool: tool.id().to_owned(),
                            topic: None,
                            anchor: None,
                        }));
                    }
                    ui.add(egui::Separator::default().spacing(SEPARATOR_SPACING));
                });
                clicked
            },
        )
        .inner
    }

    /// The PES version selector: a compact combo bound to the common `pes_version`, disabled
    /// with the blocking tool's reason while a `version_change_blocker` reports one, colored
    /// by the live exe check. A change writes the common setting and asks the shell to save.
    /// `rail` picks the short labels (item 4: `21` in the rail, `PES 2021` in the overlay).
    fn version_selector(&mut self, ui: &mut egui::Ui, args: &SidebarFrame, rail: bool) {
        let common = args.ctx.common();
        let blocker = version_blocker(args.tools);
        let folder = common.pes_folder();
        let check = self.exe_check(&common, ui.input(|input| input.focused));
        let tooltip = exe_tooltip(check, &folder);

        let mut picked = common.pes_version;
        let combo_response = ui
            .add_enabled_ui(blocker.is_none(), |ui| {
                ui.scope(|ui| {
                    ui.visuals_mut().override_text_color = selector_color(check, ui.visuals());
                    // The combo spans the sidebar's width: the default `combo_width` (~100)
                    // overflows the rail and widens the layout region past its edge.
                    let response = egui::ComboBox::from_id_salt("pes-version")
                        .width(ui.available_width())
                        // Extend, not wrap: the rail's narrow button would lay the
                        // dropdown arrow on a second row, and the selector row's
                        // height must be the same open and closed (item 3).
                        .wrap_mode(egui::TextWrapMode::Extend)
                        .selected_text(version_label(picked, rail))
                        .show_ui(ui, |ui| {
                            for version in PesVersion::ALL {
                                ui.selectable_value(
                                    &mut picked,
                                    version,
                                    version_label(version, rail),
                                );
                            }
                        })
                        .response;
                    if response.has_focus() {
                        self.focus_label = Some("pes-version");
                    }
                    response
                })
                .inner
            })
            .inner;
        let steps = wheel_steps(ui, &combo_response);
        if let Some(next) = step_choice(PesVersion::ALL, &picked, steps) {
            picked = next;
        }
        self.combo_popup_id = Some(combo_response.id.with("popup"));
        // A disabled widget keeps no hover response of its own: the reason goes on the
        // selector's own response, which still reports the pointer over its rect (ST-SIDE-04).
        match blocker {
            Some((label, reason)) => {
                combo_response.on_disabled_hover_text(format!("{label}: {reason}"));
            }
            None => {
                combo_response.on_hover_text(tooltip);
            }
        }

        self.pick_version(picked, args.ctx);
    }

    /// A manual pick of the PES version, shared by the sidebar's selector and the settings
    /// menu's row: a change writes the common setting, spends the once-per-life
    /// auto-switch so the running-PES poll never fights it (ST-SIDE-05), and asks the
    /// shell to save.
    pub(super) fn pick_version(&mut self, version: PesVersion, ctx: &ToolContext) {
        if version == ctx.common().pes_version {
            return;
        }
        ctx.settings().lock().unwrap().common.pes_version = version;
        self.auto_switch_used = true;
        ctx.request(ShellRequest::SettingsChanged);
    }

    /// The once-per-life auto-switch to a running PES (ST-SIDE-05), called from the shell's
    /// `logic` every `PES_POLL_SECONDS` on egui's clock, and only while the switch is
    /// unspent, so a spent switch polls nothing. Each call asks for the repaint at the next
    /// poll so it runs while no input arrives. Of the running games the first detected
    /// wins: the poll switches only when that game's
    /// version differs from the selection, and a `version_change_blocker` defers the
    /// switch without spending it. A switch writes `pes_version` and saves like a manual
    /// pick, and announces itself in the status bar, with a "Fix PES path" action when the
    /// running exe's folder exists on this system and differs from `pes_folder_path`
    /// resolved for the new version.
    pub(super) fn poll_running_pes(
        &mut self,
        egui_ctx: &egui::Context,
        tools: &[Box<dyn StudioTool>],
        ctx: &ToolContext,
    ) {
        if self.auto_switch_used {
            return;
        }
        let now = egui_ctx.input(|input| input.time);
        let next = self.next_poll.get_or_insert(now + PES_POLL_SECONDS);
        if now < *next {
            egui_ctx.request_repaint_after(Duration::from_secs_f64(*next - now));
            return;
        }
        *next = now + PES_POLL_SECONDS;
        egui_ctx.request_repaint_after(Duration::from_secs_f64(PES_POLL_SECONDS));
        // The blocker check is cheaper than the process snapshot, so it runs first.
        if version_blocker(tools).is_some() {
            return;
        }
        let Some(game) = (self.probe)().into_iter().next() else {
            return;
        };
        if game.version == ctx.common().pes_version {
            return;
        }
        ctx.settings().lock().unwrap().common.pes_version = game.version;
        ctx.request(ShellRequest::SettingsChanged);
        self.auto_switch_used = true;
        // The notice offers the path fix only when the running exe sits elsewhere than the
        // folder the setting resolves for the version just switched to, and that folder
        // exists on this system: under Wine or Proton the probe reports the game's Windows
        // path, which `Path::parent` cannot reduce to a usable folder here.
        let action = game
            .exe
            .as_deref()
            .and_then(Path::parent)
            .and_then(|folder| {
                (folder.is_dir() && folder != ctx.common().pes_folder()).then(|| NoticeAction {
                    label: "Fix PES path".to_owned(),
                    effect: ActionEffect::FixPesPath {
                        folder: folder.to_path_buf(),
                        version: game.version,
                    },
                })
            });
        ctx.notify(Notice {
            text: format!("Switched to PES {}", game.version.year()),
            // The fix action is offered only for an exe in another folder's
            // install, the switch worth a warning; a plain switch informs.
            kind: if action.is_some() {
                NoticeKind::Warning
            } else {
                NoticeKind::Info
            },
            action,
        });
    }

    /// The exe check's verdict, listing the resolved PES folder once per change of the
    /// version or the path and once per return of the window's focus, never per frame
    /// (ST-SIDE-03: "a cheap single directory listing"). The settings menu's version row
    /// shares it: the check is one cache for the shell.
    pub(super) fn exe_check(&mut self, common: &CommonSettings, focused: bool) -> ExeCheck {
        let folder = common.pes_folder();
        let stale = match &self.exe_check {
            Some((version, checked_folder, _)) => {
                *version != common.pes_version || *checked_folder != folder
            }
            None => true,
        };
        let gained_focus = focused && !self.was_focused;
        self.was_focused = focused;
        if stale || gained_focus {
            self.exe_listings += 1;
            let found = check_pes_folder(common.pes_version, &folder);
            self.exe_check = Some((common.pes_version, folder, found));
        }
        self.exe_check.as_ref().expect("a check ran above").2
    }
}

/// The frame of the rail's panel and of the overlay: the side panel's fill inside the
/// draft's `.rail` padding.
pub(super) fn rail_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::side_top_panel(style).inner_margin(RAIL_PADDING)
}

/// The opened sidebar's frame: the rail's, so its content starts at the same point, plus
/// an outer margin on the right as wide as the rail's line, which `show_overlay` draws
/// there as `egui::Panel` draws the rail's. The line, not a shadow, marks where the layer
/// ends over the view, both being in the panel fill.
fn overlay_frame(style: &egui::Style) -> egui::Frame {
    let line_width = style.visuals.widgets.noninteractive.bg_stroke.width.round() as i8;
    rail_frame(style).outer_margin(egui::Margin {
        right: line_width,
        ..Default::default()
    })
}

/// The overlay `egui::Area`'s id.
fn overlay_id() -> egui::Id {
    egui::Id::unique("sidebar-overlay")
}

/// The overlay's layer order this frame. `Foreground` puts it above the `Middle` help
/// window: the open sidebar is where the pointer is. While a modal is open (egui's top
/// modal layer, whatever opened it) it is `Middle`, under every modal's dimmed backdrop,
/// and egui gives a layer below the top modal no input, as the view gets none. In
/// `Foreground` it would not stay there: egui sends an area to the front of its order
/// each time it opens or is clicked, which put it over the modal with its entries live.
fn overlay_order(egui_ctx: &egui::Context) -> egui::Order {
    if egui_ctx.memory(|memory| memory.top_modal_layer()).is_some() {
        egui::Order::Middle
    } else {
        egui::Order::Foreground
    }
}

/// The planned entries the sidebar draws, in registry order: every entry whose
/// id no registered window tool has (a window tool's own entry is drawn above;
/// a CLI-only tool's is a planned entry, ST-SIDE-08).
fn drawn_planned<'a>(args: &'a SidebarFrame<'a>) -> impl Iterator<Item = &'a PlannedTool> {
    args.planned.iter().filter(|entry| {
        !args
            .tools
            .iter()
            .any(|tool| tool.id() == entry.id && !args.cli_only.contains(&tool.id()))
    })
}

/// The widest body-text galley among the labels the open sidebar shows, since the overlay
/// is as wide as the longest label needs on one line: the registered tools', the planned
/// entries' as `contents` draws them, `Help` and `Settings`. Measured each frame with the
/// fonts; no constant.
fn widest_label(ui: &egui::Ui, args: &SidebarFrame) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut widest: f32 = 0.0;
    let mut measure = |label: &str| {
        widest = widest.max(
            ui.painter()
                .layout_no_wrap(label.to_owned(), font.clone(), egui::Color32::PLACEHOLDER)
                .size()
                .x,
        );
    };
    for tool in args.tools {
        if !args.cli_only.contains(&tool.id()) {
            measure(tool.label());
        }
    }
    for entry in drawn_planned(args) {
        measure(entry.label);
    }
    measure("Help");
    measure("Settings");
    widest
}

/// The logo, the About click target: the draft's `.logo` square, centred in the icon
/// cell `cell` wide at the row's left, the same in the rail and in the overlay. The
/// row is `width` wide like the entries', but only the square senses the pointer and
/// fills while in use. The 4cc clover sits centred on it, its ring and its leaves
/// in the palette's two clover greens, hovered or not.
fn logo(ui: &mut egui::Ui, width: f32, cell: f32) -> egui::Response {
    let (id, row) = ui.allocate_space(egui::vec2(width, LOGO_SIZE));
    let square = egui::Rect::from_center_size(
        egui::pos2(row.min.x + cell / 2.0, row.center().y),
        egui::Vec2::splat(LOGO_SIZE),
    );
    let response = ui.interact(square, id, egui::Sense::click());
    paint_in_use_frame(ui, square, &response, false);
    let palette = crate::theme::palette(ui.visuals());
    for (glyph, color) in [
        (CLOVER_RING, palette.clover_ring),
        (CLOVER_LEAVES, palette.clover_leaf),
    ] {
        let galley = ui.painter().layout_no_wrap(
            glyph.to_owned(),
            egui::FontId::proportional(CLOVER_SIZE),
            color,
        );
        ui.painter()
            .galley(square.center() - galley.size() / 2.0, galley, color);
    }
    response
}

/// One sidebar entry row `width` wide, the same in the rail and in the overlay
/// (item 3: "nothing moves when it opens"): `icon` centred in the icon cell, the
/// row's first `cell` points less the accent bar's width at its left (the draft's
/// entries carry that border on every row, transparent but on the active one), then
/// `label` in body text after the cell and one `item_spacing.x` gap, vertically
/// centred and never wrapped (`None` in the rail, which shows the icons alone and
/// needs no tooltip: the overlay shows the label). `active` gets the filled
/// selection background plus the accent left bar; `clickable` selects
/// `Sense::click` over `Sense::hover`.
fn entry_row(
    ui: &mut egui::Ui,
    width: f32,
    cell: f32,
    icon: impl Into<egui::WidgetText>,
    label: Option<&str>,
    active: bool,
    clickable: bool,
) -> egui::Response {
    let sense = if clickable {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, ENTRY_HEIGHT), sense);
    let text_color = paint_in_use_frame(ui, rect, &response, active);
    if active && ui.is_enabled() {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(ACTIVE_BAR_WIDTH, rect.height())),
            egui::CornerRadius {
                nw: ACTIVE_BAR_CORNER,
                ne: 0,
                sw: ACTIVE_BAR_CORNER,
                se: 0,
            },
            ui.visuals().selection.stroke.color,
        );
    }
    let galley = icon.into().into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    );
    let cell_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + ACTIVE_BAR_WIDTH, rect.min.y),
        egui::pos2(rect.min.x + cell, rect.max.y),
    );
    ui.painter()
        .galley(cell_rect.center() - galley.size() / 2.0, galley, text_color);
    if let Some(label) = label {
        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            text_color,
        );
        ui.painter().galley(
            egui::pos2(
                cell_rect.max.x + ui.spacing().item_spacing.x,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            text_color,
        );
    }
    response
}

/// Paints a sidebar widget's button frame over `rect` while it is in use and returns
/// the colour its text is drawn in: the widget's own, or the weak text colour while
/// disabled. `active` marks the active entry, whose frame stays filled.
fn paint_in_use_frame(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    active: bool,
) -> egui::Color32 {
    let enabled = ui.is_enabled();
    let visuals = ui.style().interact_selectable(response, active);
    // A row paints its button frame only when it is in use — active, hovered,
    // pressed or focused — as the `Button::selectable` it replaced did
    // (`frame_when_inactive`): an inactive, unhovered row is a plain row.
    let in_use = active
        || response.hovered()
        || response.is_pointer_button_down_on()
        || response.has_focus();
    if enabled && in_use {
        ui.painter().rect(
            rect.expand(visuals.expansion),
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    if enabled {
        visuals.fg_stroke.color
    } else {
        ui.visuals().weak_text_color()
    }
}

/// The first `version_change_blocker` any registered tool reports, as the blocking tool's
/// label and its reason for the disabled tooltip (ST-SIDE-04); `None` while nothing
/// blocks.
pub(super) fn version_blocker(tools: &[Box<dyn StudioTool>]) -> Option<(&'static str, &str)> {
    tools.iter().find_map(|tool| {
        tool.version_change_blocker()
            .map(|reason| (tool.label(), reason))
    })
}

/// The version selector's enabled-state tooltip for an exe-check verdict (ST-SIDE-03):
/// what the last listing of the resolved PES folder found. The settings menu's version
/// row carries the same one (the draft's `pesSelectorTip`).
pub(super) fn exe_tooltip(check: ExeCheck, folder: &Path) -> String {
    match check {
        ExeCheck::Found => format!(
            "{}\nThe PES version every tool works with",
            folder.display()
        ),
        ExeCheck::OtherVersion(found) => format!(
            "{} holds PES{}.exe, which is a different version from the one selected",
            folder.display(),
            found.year()
        ),
        ExeCheck::NoExe => format!("There is no PES exe in {}", folder.display()),
        ExeCheck::NoFolder => format!("{} does not exist", folder.display()),
    }
}

/// The combo's item label: `PES 2021` in the open sidebar and the settings menu,
/// `21` on the rail, as the draft shows it.
pub(super) fn version_label(version: PesVersion, rail: bool) -> String {
    if rail {
        version.number().to_string()
    } else {
        version.to_string()
    }
}

/// The selector's colour for an exe-check verdict (ST-SIDE-03): red on a different
/// version's exe, orange on no exe or no folder, none when the selection is installed.
pub(super) fn selector_color(check: ExeCheck, visuals: &egui::Visuals) -> Option<egui::Color32> {
    match check {
        ExeCheck::OtherVersion(_) => Some(visuals.error_fg_color),
        ExeCheck::NoExe | ExeCheck::NoFolder => Some(visuals.warn_fg_color),
        ExeCheck::Found => None,
    }
}

/// One directory listing of `folder` for `PES20NN.exe` names, in any letter case: the
/// selected version's own exe wins when several are present (ST-SIDE-03). Only files count:
/// the condition mirrors the compiler's preflight (`is_file` on the exe's path).
fn check_pes_folder(version: PesVersion, folder: &Path) -> ExeCheck {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(_) => return ExeCheck::NoFolder,
    };
    let mut other = None;
    for entry in entries.flatten() {
        // `Path::is_file` follows a link the same way the compiler's preflight does.
        if !entry.path().is_file() {
            continue;
        }
        let Some(found) = entry
            .file_name()
            .to_str()
            .and_then(crate::pes_process::version_of_exe)
        else {
            continue;
        };
        if found == version {
            return ExeCheck::Found;
        }
        if other.is_none() {
            other = Some(found);
        }
    }
    other.map_or(ExeCheck::NoExe, ExeCheck::OtherVersion)
}
