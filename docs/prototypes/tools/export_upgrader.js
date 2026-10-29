// The Export upgrader's draft (plans/export_upgrader.md "GUI"), a view of the
// drafts' host page, index.html: the two drop targets (an old export, or a
// folder of old model folders), the run's arguments (the discovered save or
// the loose switch), the Upgrade run and its report - the log panel, the
// left-behind list, the done line, the notice and Move to exports and compile.
// The samples are real recorded runs (.tmp/8_17/runs/): the egg export end to
// end, a zip, a referee export, a not-clean, a refused and an aborted run, and
// both model-folder outcomes, with the stand-in pickers and the close prompt.
// The wrapper keeps the view's names out of the host's scope; its body is left
// unindented, as in tools/save_editor.js.
// Names this script uses without declaring them (render, h, row, weak, strong,
// button, combo, cb, collapsing, window_, closeWindow, publishNotice,
// shellPesVersion, switchToTool, toolActivity, folderOf, mmss,
// EXE_FOLDER, appendLog, logPanel, openOutputStub, named, registerTool) are the
// host's, index.html.
(() => {
"use strict";

// The view's own rules the shared stylesheet does not already hold: the two
// equal columns the targets sit in, a target's minimum height and filled
// border, the left-behind list's cap and scroll.
const EXPORT_UPGRADER_CSS = `
  .eu-cols { display: flex; gap: 8px; }
  .eu-col { flex: 1; min-width: 0; }
  .eu-target { min-height: 70px; display: flex; flex-direction: column; }
  .eu-target .grow { flex: 1; }
  .eu-target.filled { border-color: var(--sel-fg); }
  .eu-left { max-height: 160px; overflow: auto; }
  .eu-done { margin: 6px 0; }
  .eu-args { width: 100%; }
  .eu-args td:first-child { white-space: nowrap; }
  .eu-args td.fld { width: 100%; }
  .eu-args td.fld > span { display: block; }
  .eu-args td.fld input { box-sizing: border-box; }
  .eu-out { flex: 1; min-width: 0; }
  .eu-out input { box-sizing: border-box; }
`;

// ---- Sample data ----
// The discovered saves: the Save editor's DISCOVERED sample with the Documents
// root under "you". The savefile field is prefilled with the selected version's.
const DOCS = "C:\\Users\\you\\Documents\\KONAMI";
const DISCOVERED = [
  { version: "2015", path: DOCS + "\\Pro Evolution Soccer 2015\\save\\EDIT.bin" },
  { version: "2016", path: DOCS + "\\Pro Evolution Soccer 2016\\save\\EDIT00000000" },
  { version: "2017", path: DOCS + "\\Pro Evolution Soccer 2017\\save\\EDIT00000000" },
  { version: "2018", path: DOCS + "\\PRO EVOLUTION SOCCER 2018\\save\\EDIT00000000" },
  { version: "2019", path: DOCS + "\\PRO EVOLUTION SOCCER 2019\\292733975847239680\\save\\EDIT00000000" },
  { version: "2020", path: DOCS + "\\eFootball PES 2020\\76561199320748656\\save\\EDIT00000000" },
  { version: "2021", path: DOCS + "\\eFootball PES 2021 SEASON UPDATE\\292733975847239680\\save\\EDIT00000000" },
];
// The runs, generated from the recorded CLI output's stdout verbatim
// (.tmp/8_17/runs/ by extract_runs.py); the label replacements are named in
// extract_runs.py's table.
const EGG_SAVE21 = { exit: 0, lines: [
  "- EGG Aesthetics Export VGL26: Info logo_small_dropped at Logo/emblem_0XXX_r.png (share=7%)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX19 - M1 Bradley/face_high.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/m_bradley_dummy_nrm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX19 - M1 Bradley/face_high.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/m_bradley_dummy_srm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX19 - M1 Bradley/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/m_bradley_dummy_nrm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX19 - M1 Bradley/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/m_bradley_dummy_srm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX20 - Godforsaken Astronaut/face_high.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/spec.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX23 - Lemonhulk/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/kit_texture_nrm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX23 - Lemonhulk/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/kit_texture_srm.dds)",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2176 - EMPTY/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/body_u0XXXp1.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/body_u0XXXp2.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/body_u0XXXp3.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/body_u0XXXp4.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/corpse_u0XXXp1.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/corpse_u0XXXp2.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/corpse_u0XXXp3.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/corpse_u0XXXp4.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/ghost_u0XXXp1.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/ghost_u0XXXp2.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/ghost_u0XXXp3.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/ghost_u0XXXp4.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/sign_u0XXXp1.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/sign_u0XXXp2.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/sign_u0XXXp3.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2177 - THIS BEAVER IS CONTAMINATED/sign_u0XXXp4.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2178 - DEFAULT SPENGIE HEADLESS/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2178 - DEFAULT SPENGIE HEADLESS/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2178 - DEFAULT SPENGIE HEADLESS/m.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2178 - DEFAULT SPENGIE HEADLESS/srm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2179 - aeiou/aeiou.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2179 - aeiou/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2179 - aeiou/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2179 - aeiou/m.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2179 - aeiou/srm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2180 - The Chad Stormworks Player/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2180 - The Chad Stormworks Player/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2180 - The Chad Stormworks Player/shirt.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2182 - Chaos Engine/ball.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2182 - Chaos Engine/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2182 - Chaos Engine/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2182 - Chaos Engine/chaos.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2182 - Chaos Engine/engine.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2182 - Chaos Engine/fire.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2183 - Landing Gear On Rotors On Pistons/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2183 - Landing Gear On Rotors On Pistons/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2183 - Landing Gear On Rotors On Pistons/m.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2183 - Landing Gear On Rotors On Pistons/srm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2183 - Landing Gear On Rotors On Pistons/visor.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2183 - Landing Gear On Rotors On Pistons/visor_nrm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2184 - Thomasformer/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2184 - Thomasformer/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2184 - Thomasformer/thomas.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/floor.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/m.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/srm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/step.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/visor.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/visor_nrm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2185 - Mechanically Engineered Autism/visor_srm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2186 - CLANG/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2186 - CLANG/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2186 - CLANG/m.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2186 - CLANG/metal.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2186 - CLANG/srm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2187 - Square-chan/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2187 - Square-chan/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2187 - Square-chan/metal_roughness.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2187 - Square-chan/square.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2188 - Structural Integrity/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2188 - Structural Integrity/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2190 - Jebediah Kerman/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2190 - Jebediah Kerman/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2190 - Jebediah Kerman/kerbal_bsm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2190 - Jebediah Kerman/kerbal_nrm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2191 - SPIDERTRON/body.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2191 - SPIDERTRON/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2191 - SPIDERTRON/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2191 - SPIDERTRON/helmet.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2191 - SPIDERTRON/kit_a.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2191 - SPIDERTRON/kit_b.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2192 - FAILURE/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2192 - FAILURE/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2192 - FAILURE/nonkit.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2192 - FAILURE/pack.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2193 - BLUE HURT/blink.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2193 - BLUE HURT/board.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2193 - BLUE HURT/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2193 - BLUE HURT/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2193 - BLUE HURT/env.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2193 - BLUE HURT/eye.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2193 - BLUE HURT/main.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2193 - BLUE HURT/metal_roughness.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2197 - Turrents/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2197 - Turrents/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2197 - Turrents/metal_roughness.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2197 - Turrents/turrent.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2198 - Lemonhulk/body.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2198 - Lemonhulk/boots.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2198 - Lemonhulk/boots.skl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2198 - Lemonhulk/dummy_nrm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2198 - Lemonhulk/dummy_srm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2198 - Lemonhulk/lemon.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2198 - Lemonhulk/lmeone.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2176 - Eggman/egg.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2176 - Eggman/glove_l.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2176 - Eggman/glove_r.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2178 - DEFAULT SPENGIE GLOVES/glove_l.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2178 - DEFAULT SPENGIE GLOVES/glove_r.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2178 - DEFAULT SPENGIE GLOVES/hands.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2180 - The Chad Stormworks Player/glove_l.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2180 - The Chad Stormworks Player/glove_r.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2186 - CLANG/glove_l.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2186 - CLANG/glove_r.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2186 - CLANG/hands.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2188 - Structural Integrity/glove_l.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2188 - Structural Integrity/glove_r.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2188 - Structural Integrity/hands.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2190 - Jebediah Kerman/glove_l.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2190 - Jebediah Kerman/glove_r.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2190 - Jebediah Kerman/kerbal_bsm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2190 - Jebediah Kerman/kerbal_nrm.dds",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2192 - FAILURE/glove_l.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2192 - FAILURE/glove_r.fmdl",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Gloves/g2192 - FAILURE/nonkit.dds",
  "- Info migration_clean",
] };

const EGG_LOOSE = { exit: 0, lines: [
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2176 - EMPTY (id=2176, slot=01)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2177 - THIS BEAVER IS CONTAMINATED (id=2177, slot=02)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2178 - DEFAULT SPENGIE HEADLESS (id=2178, slot=03)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2179 - aeiou (id=2179, slot=04)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2180 - The Chad Stormworks Player (id=2180, slot=05)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2182 - Chaos Engine (id=2182, slot=07)",
  "- EGG Aesthetics Export VGL26: Info merge_kept_shared at Boots/k2182 - Chaos Engine (path=ball.dds)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2183 - Landing Gear On Rotors On Pistons (id=2183, slot=08)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2184 - Thomasformer (id=2184, slot=09)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2185 - Mechanically Engineered Autism (id=2185, slot=10)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2186 - CLANG (id=2186, slot=11)",
  "- EGG Aesthetics Export VGL26: Info merge_kept_shared at Boots/k2186 - CLANG (path=metal.dds)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2187 - Square-chan (id=2187, slot=12)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2188 - Structural Integrity (id=2188, slot=13)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2190 - Jebediah Kerman (id=2190, slot=15)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2191 - SPIDERTRON (id=2191, slot=16)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2192 - FAILURE (id=2192, slot=17)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2193 - BLUE HURT (id=2193, slot=18)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2197 - Turrents (id=2197, slot=22)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Boots/k2198 - Lemonhulk (id=2198, slot=23)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Gloves/g2176 - Eggman (id=2176, slot=01)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Gloves/g2177 - EMPTY (id=2177, slot=02)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Gloves/g2178 - DEFAULT SPENGIE GLOVES (id=2178, slot=03)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Gloves/g2180 - The Chad Stormworks Player (id=2180, slot=05)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Gloves/g2186 - CLANG (id=2186, slot=11)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Gloves/g2188 - Structural Integrity (id=2188, slot=13)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Gloves/g2190 - Jebediah Kerman (id=2190, slot=15)",
  "- EGG Aesthetics Export VGL26: Info wearer_assumed at Gloves/g2192 - FAILURE (id=2192, slot=17)",
  "- EGG Aesthetics Export VGL26: Info logo_small_dropped at Logo/emblem_0XXX_r.png (share=7%)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX10 - Mechanically Engineered Autism/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/visor_srm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX19 - M1 Bradley/face_high.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/m_bradley_dummy_nrm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX19 - M1 Bradley/face_high.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/m_bradley_dummy_srm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX19 - M1 Bradley/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/m_bradley_dummy_nrm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX19 - M1 Bradley/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/m_bradley_dummy_srm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX20 - Godforsaken Astronaut/face_high.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/spec.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Boots/k2198 - Lemonhulk/boots.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/kit_texture_nrm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Boots/k2198 - Lemonhulk/boots.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/kit_texture_srm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX23 - Lemonhulk/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/kit_texture_nrm.dds)",
  "- EGG Aesthetics Export VGL26: Warning common_reference_kept at Faces/XXX23 - Lemonhulk/fcl_hair.fmdl (reference=/Assets/pes16/model/character/common/000/sourceimages/kit_texture_srm.dds)",
  "- EGG Aesthetics Export VGL26: Warning leftover_dropped at Boots/k2176 - EMPTY/boots.skl",
  "- Info migration_clean",
] };

const CO_FIXTURE = { exit: 0, lines: [
  "- co Spring: Warning kit_without_config at co Note.txt (slot=p1)",
  "- co Spring: Warning kit_without_config at co Note.txt (slot=p2)",
  "- co Spring: Warning kit_without_config at co Note.txt (slot=p3)",
  "- co Spring: Warning kit_without_config at co Note.txt (slot=g1)",
  "- co Spring: Warning leftover_dropped at Other/readme.txt",
  "- co Full Spring: Info notes_found at notes.txt",
  "- Info migration_clean",
] };

const AT_LOOSE = { exit: 1, lines: [
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3126 - MochiWHAT (id=3126, slot=01)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3127 - The Yoinky Sploinky (id=3127, slot=02)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3132 - Moogy (id=3132, slot=07)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3134 - Genki Train (id=3134, slot=09)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3138 - Bouncy Bouncy (id=3138, slot=13)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3140 - TakeP (id=3140, slot=15)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3141 - Konomini (id=3141, slot=16)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3144 - Bad Man (id=3144, slot=19)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Boots/k3146 - Fufu Im Dying (id=3146, slot=21)",
  "- @  Aesthetics Export VGL26: Error unmigrated_content at Boots/k3149 - Flat Kit",
  "- @  Aesthetics Export VGL26: Error unmigrated_content at Boots/k3150 - Boob Kit",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3128 - Neech (id=3128, slot=03)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3129 - Rolling Riceballs (id=3129, slot=04)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3130 - You Called You Blithering Idiot (id=3130, slot=05)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3131 - Your Keys Please (id=3131, slot=06)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3133 - Bring Out Miki (id=3133, slot=08)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3135 - Ugh (id=3135, slot=10)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3136 - Shrimple As (id=3136, slot=11)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3137 - You Will Never Be Japanese (id=3137, slot=12)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3139 - I Wanna Doun (id=3139, slot=14)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3140 - TakeP (id=3140, slot=15)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3142 - Mentally Chill (id=3142, slot=17)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3143 - Fatmari (id=3143, slot=18)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3145 - Dookie (id=3145, slot=20)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3146 - Fufu Im Dying (id=3146, slot=21)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3147 - 72 (id=3147, slot=22)",
  "- @  Aesthetics Export VGL26: Info wearer_assumed at Gloves/g3148 - Nonowa (id=3148, slot=23)",
  "- @  Aesthetics Export VGL26: Info logo_small_dropped at Logo/emblem_0781_r.png (share=15%)",
  "- Info migration_not_clean",
] };

const REFS_TRACER = { exit: 0, lines: [
  "- Info migration_clean",
] };

const MODELS_CLEAN = { exit: 0, lines: [
  "- EGG models upgraded: Info model_folder_destination at Common (into=root)",
  "- EGG models upgraded: Info model_folder_destination at 05 - The Chad Stormworks Player (into=Players)",
  "- EGG models upgraded: Info model_folder_destination at Structural Integrity (into=Gloves)",
  "- EGG models upgraded: Info model_folder_destination at CLANG (into=Boots)",
  "- Info migration_clean",
] };

const MODELS_OLD = { exit: 1, lines: [
  "- EGG old models: Error unmigrated_content at Old player folders",
  "- EGG old models upgraded: Info model_folder_destination at Common (into=root)",
  "- EGG old models upgraded: Info model_folder_destination at 05 - The Chad Stormworks Player (into=Players)",
  "- EGG old models upgraded: Info model_folder_destination at Structural Integrity (into=Gloves)",
  "- EGG old models upgraded: Info model_folder_destination at CLANG (into=Boots)",
  "- Info migration_not_clean",
] };

// The sample inputs, keyed by a short id. Each team's number is its row of
// teams_list.txt; `output` is the folder name the run writes for upgrade
// (model-folder runs write into the output directory itself: null).
const INPUTS = {
  egg: { target: "export", name: "EGG Aesthetics Export VGL26",
    path: "C:\\Users\\you\\Downloads\\EGG Aesthetics Export VGL26",
    team: "/egg/", teamId: 792, output: "egg Full Aesthetics Export VGL26",
    runs: { savefile: EGG_SAVE21, loose: EGG_LOOSE } },
  // co Spring.zip (a .zip counts as an export): its --loose run and its
  // --savefile run with Autumn24Q's save, whose /co/ roster wears no custom boots
  // or gloves, print the same seven lines (co_save_autumn24q.txt). A save whose
  // /co/ wears custom items (Genso Cup 2's) ends not clean with
  // custom_model_missing instead (co_save_genso.txt).
  co: { target: "export", name: "co Spring.zip",
    path: "C:\\Users\\you\\Downloads\\co Spring.zip",
    team: "/co/", teamId: 714, output: "co Full Spring",
    runs: { savefile: CO_FIXTURE, loose: CO_FIXTURE } },
  // @'s loose run ends not clean (two boots at the block's last positions go
  // to no player); a savefile it does not hold refuses the run (exit 2), its
  // line built at run time (upgrade.rs ~363's format).
  at: { target: "export", name: "@  Aesthetics Export VGL26",
    path: "C:\\Users\\you\\Downloads\\@  Aesthetics Export VGL26",
    team: "/@/", teamId: 781, output: "@ Full  Aesthetics Export VGL26",
    runs: { savefile: { exit: 2, lines: [] }, loose: AT_LOOSE } },
  // A referee export takes neither argument: one run, as the CLI prints it.
  refs: { target: "export", name: "refs Tracer",
    path: "C:\\Users\\you\\Downloads\\refs Tracer",
    team: null, output: "refs Tracer upgraded",
    runs: { plain: REFS_TRACER } },
  // upgrade-models writes its folders straight into the output directory
  // ("<input> upgraded" by default): one run, as the CLI prints it.
  models: { target: "models", name: "EGG models",
    path: "C:\\Users\\you\\Downloads\\EGG models",
    team: null, output: null,
    runs: { plain: MODELS_CLEAN } },
  models_old: { target: "models", name: "EGG old models",
    path: "C:\\Users\\you\\Downloads\\EGG old models",
    team: null, output: null,
    runs: { plain: MODELS_OLD } },
};

// ---- State ----
// D: the draft controls' GIVENs. S: the view's state; L its log panel's (the
// host widget's six fields).
const D = { discovered: true, outputExists: false, runLength: "short" };
const S = { input: null, savefile: "", savefileEdited: false, loose: false,
  outputInto: "", outputEdited: false,
  run: null, result: null, moved: false, closeAtEnd: false, logScroll: 0 };
const L = { log: [], logHeight: 180, logCollapsed: false, logFilter: "all",
  logStick: true, logNew: 0 };

const input = () => S.input ? INPUTS[S.input] : null;
// A referee export (its name's first word `refs`, any letter case) takes
// neither a savefile nor loose.
const isRefs = () => !!input() && input().name.split(" ")[0].toLowerCase() === "refs";
// The savefile field's shown value: the discovered save until the member edits.
const savefile = () => S.savefileEdited ? S.savefile : discoveredSave();
const discoveredSave = () => !D.discovered ? ""
  : (DISCOVERED.find(d => d.version === shellPesVersion()) || { path: "" }).path;
// The arguments' controls disable together: a referee export takes neither,
// and model-folder mode has no team to resolve.
const argsDisabled = () => !!input() && (isRefs() || input().target === "models");
const argsDisabledWhy = () => isRefs()
  ? "A referee export takes no savefile and no loose switch"
  : "Model folders take no savefile and no loose switch";
// Output into (-o): the input's parent for an export, "<input> upgraded" for
// model folders (upgrade-models writes straight into it).
const outputDefault = () => !input() ? ""
  : input().target === "export" ? folderOf(input().path)
  : input().path + " upgraded";
const outputInto = () => S.outputEdited ? S.outputInto : outputDefault();
// Loose mode (help "Running from a terminal"): each boots or gloves folder
// goes to the player at its ID's position in the team's old block of 25 IDs,
// each pairing reported as wearer_assumed.
const LOOSE_TIP = "Each boots or gloves folder goes to the player at its ID's position in the team's old block of 25 IDs, each pairing reported as wearer_assumed. An ID on the block's last two positions or past it belongs to no player and is reported as unmigrated content.";
// The command the button's tooltip names, as the CLI line; null when the
// arguments are not all set (a team export needs a savefile or loose).
function upgradeArgs() {
  const i = input();
  if (!i) return null;
  const tail = ` -o "${outputInto()}"`;
  if (i.target === "models") return `upgrade-models "${i.path}"` + tail;
  if (isRefs()) return `upgrade "${i.path}"` + tail;
  if (S.loose) return `upgrade "${i.path}" --loose` + tail;
  const sv = savefile();
  if (!sv) return null;
  return `upgrade "${i.path}" --savefile "${sv}"` + tail;
}

// A drop on a target (the draft controls' combo): one input at a time, the
// other target cleared by it; a new input clears the previous run's report.
// The savefile and loose take the new input's defaults; Output into resets to
// the default unless the member edited it (S.outputEdited kept).
function setInput(key) {
  stopRun();
  S.input = key;
  S.savefile = ""; S.savefileEdited = false; S.loose = false;
  S.result = null; S.moved = false; S.closeAtEnd = false;
  L.log = []; L.logNew = 0; L.logStick = true;
}

// One console line becomes { severity, code, scope, text }: "- <export>:
// <Severity> <code> at <path> (<context>)" or the last line "- <Severity>
// <code>"; a line that does not parse (an error's line) is Error without a code.
function parseLogLine(text) {
  let m = text.match(/^- (Info|Warning|Error|Fatal) (\S+)$/);
  if (m) return { severity: m[1], code: m[2], scope: null, context: null, text };
  m = text.match(/^- .+?: (Info|Warning|Error|Fatal) (\S+) at (.*?)(?: \((.*)\))?$/);
  if (m) return { severity: m[1], code: m[2], scope: m[3], context: m[4] ?? null, text };
  return { severity: "Error", code: null, scope: null, context: null, text };
}

// The sample run for the arguments taken: savefile or loose for a team
// export, the one run of a referee export or a model-folder input.
const runKey = () => isRefs() || input().target === "models" ? "plain"
  : S.loose ? "loose" : "savefile";
// A refused (exit 2) or aborted (exit 3) run draws its one stderr line at
// once, as an Error line (the unparsed-line rule), no list, no notice.
function instantResult(exit, text) {
  appendLog([parseLogLine(text)], L);
  S.result = { exit, output: outputInto() };
  render();
}
// upgrade.rs ~363: "--savefile {}: it does not hold the resolved team {name}
// ({id})"; the savefile is the field's value at run time.
const refusalText = () => `error: --savefile ${savefile()}: it does not hold the resolved team ${input().team} (${input().teamId})`;
// staging.rs ~25: "{}: the output folder already exists"; upgrade aborts on
// the folder it would create, upgrade-models on the output directory itself.
const abortText = () => input().target === "models"
  ? `error: ${outputInto()}: the output folder already exists`
  : `error: ${outputInto()}\\${input().output}: the output folder already exists`;
// The output's name: the folder upgrade writes, or the output directory
// itself for upgrade-models.
const outputName = () => input().target === "models"
  ? outputInto().split("\\").pop() : input().output;
// The fake run: the sample's lines arrive into the log in batches over about
// two seconds ("about 20 s" draws the same over twenty); the activity reads
// the elapsed time only (no counts).
function startRun(instant) {
  // A new run's report replaces the last one's (the refusal and the abort
  // included).
  L.log = []; L.logNew = 0; L.logStick = true;
  S.result = null; S.moved = false;
  const run = input().runs[runKey()];
  // The save lacking the team is refused before the staging folder is
  // claimed (upgrade.rs's order), so it wins over the output-exists abort.
  if (run.exit === 2) { instantResult(2, refusalText()); return; }
  if (D.outputExists) { instantResult(3, abortText()); return; }
  S.run = { lines: run.lines, exit: run.exit, next: 0, elapsed: 0,
    start: Date.now(), output: outputInto() };
  // A link's run=done draws the whole report at once, no timer. The Upgrade
  // button passes its click event as the argument, so this tests === true.
  if (instant === true) {
    S.run.timer = 0;
    appendLog(S.run.lines.map(parseLogLine), L);
    S.run.next = S.run.lines.length;
    finishRun();
    return;
  }
  toolActivity("export-upgrader", { elapsed: 0 });
  render();
  const batch = Math.ceil(run.lines.length / 16);
  S.run.timer = setInterval(() => {
    const r = S.run;
    if (!r) return;
    const part = r.lines.slice(r.next, r.next + batch).map(parseLogLine);
    r.next += part.length;
    r.elapsed = Math.floor((Date.now() - r.start) / 1000);
    toolActivity("export-upgrader", { elapsed: r.elapsed });
    appendLog(part, L);
    if (r.next >= r.lines.length) finishRun();
  }, D.runLength === "long" ? 1250 : 125);
}
function stopRun() {
  if (!S.run) return;
  clearInterval(S.run.timer); S.run = null;
  toolActivity("export-upgrader", null);
}
function finishRun() {
  const r = S.run;
  clearInterval(r.timer);
  S.run = null;
  toolActivity("export-upgrader", null);
  S.result = { exit: r.exit, output: r.output };
  if (r.exit === 0)
    publishNotice("Upgraded " + outputName(),
      { label: "Open output folder", run: openOutputStub }, "success");
  else render();
  if (S.closeAtEnd) { S.closeAtEnd = false; closeStub(); }
}

// The clean run's Move button: the output goes into the shell's default
// exports folder and the Team compiler takes over.
function moveAndCompile() {
  S.moved = true;
  switchToTool("team-compiler");
}

// The Browse buttons' stand-in file dialogs (a window_ titled as the native
// picker would be): one button per pickable sample path, Cancel closing it.
// Each carries its link name, so the link's dialog key reopens it.
function picker(name, title, options, pick) {
  window_(title, h("div", {},
    weak("The system's file picker opens here; the draft lists the samples it could pick."),
    options.map(([label, v]) =>
      h("div", {}, button(label, () => { pick(v); closeWindow(); render(); })))),
    [button("Cancel", closeWindow)], name);
  render(); // the link carries the open picker's name
}
const targetSamples = target => Object.keys(INPUTS).filter(k => INPUTS[k].target === target);
const exportPicker = () => picker("pick-export", "Upgrade an export",
  targetSamples("export").map(k => [INPUTS[k].path, k]), setInput);
const modelsPicker = () => picker("pick-models", "Upgrade model folders",
  targetSamples("models").map(k => [INPUTS[k].path, k]), setInput);
const savefilePicker = () => picker("pick-savefile", "Savefile",
  DISCOVERED.map(d => [d.path, d.path]),
  v => { S.savefile = v; S.savefileEdited = true; });
const outputPicker = () => picker("pick-output", "Output into",
  [["C:\\Users\\you\\Downloads", "C:\\Users\\you\\Downloads"],
    ["C:\\4cc\\unlimited-tool-works-0.0.1\\exports", "C:\\4cc\\unlimited-tool-works-0.0.1\\exports"]],
  v => { S.outputInto = v; S.outputEdited = true; });

// The close stub shown at a run's end after Close when it ends (EU-GUI-07);
// with no run in flight the host shows the same stub itself.
const closeStub = () => window_("Close", weak("The window closes here."),
  [button("OK", closeWindow)]);
// The view's closeRequest hook (the shell's draft control asks each view): a
// run in flight switches to this view, draws the prompt and holds the close;
// without one the host's stub covers it.
function closeRequest() {
  if (!S.run) return false;
  switchToTool("export-upgrader");
  window_("An upgrade is running",
    weak("Closing now stops it before it writes anything."),
    [button("Close when it ends", () => { S.closeAtEnd = true; closeWindow(); }),
      button("Keep the window open", closeWindow)]);
  return true;
}

// A disabled control's tooltip sits on a wrapper span, which still receives
// the hover a disabled input does not.
const tip = (why, el) => h("span", { title: why }, el);

// One drop target: the hint, or the held input's name and path, the Browse
// button at its bottom. A filled target is bordered in the selection colour.
function dropTarget(target) {
  const i = input();
  const holds = i && i.target === target;
  return h("div", { class: "group eu-target" + (holds ? " filled" : ""),
      "data-panel": target === "export" ? "target-export" : "target-models" },
    strong(target === "export" ? "Upgrade an export" : "Upgrade model folders"),
    holds ? [h("div", {}, strong(i.name)),
      h("div", { class: "weak mono" }, i.path)]
      : weak(target === "export"
        ? "Drop an old export here: a folder, a .zip or a .7z"
        : "Drop a folder of old model folders here"),
    h("div", { class: "grow" }),
    h("div", {}, button("Browse…", target === "export" ? exportPicker : modelsPicker)));
}

// The run's arguments under the first target, in its column: Savefile and
// Loose, the CLI's and nothing more. The fields fill their cells.
function argsGrid() {
  const dis = argsDisabled(), why = argsDisabledWhy();
  const saveDis = dis || S.loose;
  const saveWhy = dis ? why : "Loose mode takes no savefile";
  return h("table", { class: "grid eu-args", "data-panel": "arguments" },
    h("tr", {},
      h("td", {}, "Savefile"),
      h("td", { class: "fld" }, saveDis ? tip(saveWhy,
        h("input", { type: "text", class: "mono", disabled: true,
          value: savefile(), style: "width:100%" }))
        : h("input", { type: "text", class: "mono", value: savefile(),
          style: "width:100%",
          oninput: e => { S.savefile = e.target.value; S.savefileEdited = true; } })),
      h("td", {}, saveDis ? tip(saveWhy, button("Browse…", null, { disabled: true }))
        : button("Browse…", savefilePicker))),
    h("tr", {},
      h("td", {}, "Loose"),
      h("td", { colspan: "2" },
        h("span", { title: dis ? why : LOOSE_TIP },
          h("label", { class: "cb" },
            h("input", { type: "checkbox", checked: S.loose, disabled: dis,
              onchange: e => { S.loose = e.target.checked; render(); } }),
            "No savefile: infer the wearers from the IDs")))));
}

// Output into (-o) serves both targets, so its row sits under both columns;
// the field fills its cell.
function outputRow() {
  return h("div", { class: "row", "data-panel": "output" },
    h("span", { style: "white-space:nowrap" }, "Output into"),
    h("div", { class: "eu-out" },
      h("input", { type: "text", class: "mono", value: outputInto(),
        style: "width:100%",
        oninput: e => { S.outputInto = e.target.value; S.outputEdited = true; } })),
    button("Browse…", outputPicker));
}

function runRow() {
  const args = upgradeArgs();
  // A disabled button gets no browser tooltip; its reason goes on a wrapper.
  const btn = button("Upgrade", startRun, { disabled: !args || !!S.run,
    title: args || null });
  return h("div", { class: "row", "data-panel": "run" },
    args ? btn : tip(S.input
      ? "A team export takes a savefile or the loose switch"
      : "Drop an input on a target first", btn),
    S.run ? weak(`Upgrading… ${mmss(S.run.elapsed)}`) : null);
}

// The run's verdict in words; a clean upgrade adds Move to exports and compile.
function doneLine() {
  if (!S.result) return null;
  const i = input();
  if (S.moved)
    return h("div", { class: "eu-done", "data-panel": "done" },
      h("span", {}, `Moved to ${EXE_FOLDER}\\exports\\${i.output}.`));
  const out = S.result.output + (i.target === "export" ? "\\" + i.output : "");
  const text = S.result.exit === 0 ? `Clean: written to ${out}.`
    : S.result.exit === 1 ? "Not clean: nothing was written. Fix what the list below names in the "
      + (i.target === "models" ? "input folder" : "old export") + ", then upgrade it again."
    : S.result.exit === 2 ? "Refused: nothing was written."
    : "Aborted: nothing was written.";
  return h("div", { class: "eu-done", "data-panel": "done" }, h("span", {}, text),
    " ",
    S.result.exit === 0 && i.target === "export"
      ? button("Move to exports and compile", moveAndCompile) : null);
}

// What the run left behind, above the log: one row per unmigrated_content (to
// fix in the input, error colour) or leftover_dropped (the output left it out,
// warning colour), as the log lists them. The list scrolls past 160px so the
// log panel keeps its place.
function leftBehind() {
  const rows = L.log.filter(l =>
    l.code === "unmigrated_content" || l.code === "leftover_dropped");
  if (!rows.length) return null;
  const toFix = rows.filter(l => l.code === "unmigrated_content").length;
  const hasContext = rows.some(l => l.context);
  return h("div", { "data-panel": "left-behind" },
    h("div", {}, strong(`Left behind: ${rows.length}`), " ",
      weak(`${toFix} to fix in the input · ${rows.length - toFix} left out of the output`)),
    h("div", { class: "eu-left" },
      h("table", { class: "grid" }, rows.map(l =>
        h("tr", { class: l.code === "unmigrated_content" ? "err" : "warn" },
          h("td", { class: "mono" }, l.scope), h("td", {}, l.code),
          hasContext ? h("td", { class: "weak" }, l.context) : null)))));
}

// ---- The view ----
function drawView(center) {
  const oldLog = center.querySelector(".log-body");
  if (oldLog) S.logScroll = oldLog.scrollTop;
  center.innerHTML = "";
  center.append(h("div", { class: "pipe" },
    h("div", { class: "pipe-grid" },
      h("div", { class: "eu-cols" },
        h("div", { class: "eu-col" }, dropTarget("export"), argsGrid()),
        h("div", { class: "eu-col" }, dropTarget("models"))),
      outputRow(),
      runRow(),
      doneLine(),
      leftBehind()),
    named(logPanel(L, { tool: "export-upgrader" }), "log")));
  const lb = center.querySelector(".log-body");
  if (lb) lb.scrollTop = L.logStick ? lb.scrollHeight : S.logScroll;
}

// ---- Draft controls: the scenarios' GIVENs, visibly outside Studio ----
function draftControls() {
  return [
    h("label", { class: "cb", title: "A file drop on the target (EU-GUI-01)" }, "Drop on Upgrade an export",
      combo([[null, "(nothing)"],
        ...targetSamples("export").map(k => [k, INPUTS[k].name])],
        input() && input().target === "export" ? S.input : null,
        v => { if (v) setInput(v);
          else if (input() && input().target === "export") setInput(null); })),
    h("label", { class: "cb", title: "A file drop on the target (EU-GUI-03)" }, "Drop on Upgrade model folders",
      combo([[null, "(nothing)"],
        ...targetSamples("models").map(k => [k, INPUTS[k].name])],
        input() && input().target === "models" ? S.input : null,
        v => { if (v) setInput(v);
          else if (input() && input().target === "models") setInput(null); })),
    h("span", { title: "EU-GUI-01" },
      cb("A save is discovered for the selected version", D.discovered,
        v => D.discovered = v)),
    h("span", { title: "EU-GUI-05" },
      cb("The output folder already exists", D.outputExists,
        v => D.outputExists = v)),
    h("label", { class: "cb", title: "EU-GUI-07" }, "The run takes",
      combo([["short", "about 2 s"], ["long", "about 20 s"]], D.runLength,
        v => D.runLength = v)),
    collapsing("export-upgrader/walk", "How to walk the scenarios", () =>
      WALK.map(([id, how]) => h("div", {}, strong(id), " ", how)))];
}
const WALK = [
  ["EU-GUI-01", "sidebar selector at PES 21, drop egg on Upgrade an export: the target names it, the savefile holds the PES 21 discovered save, loose off, Output into the input's parent, Upgrade enabled; the discovered control off and a fresh drop: the field empty, Upgrade disabled until loose is switched on; loose on + Upgrade runs egg's --loose sample"],
  ["EU-GUI-02", "drop co Spring.zip and Upgrade with the save taken: the log shows the fixture's lines, the left-behind list holds the Other/readme.txt leftover_dropped row, the done line reads Clean, the notice offers Open output folder; Move to exports and compile activates the Team compiler and the done line reads Moved to"],
  ["EU-GUI-03", "drop egg, then EGG old models on the model-folders target: the first target's hint is back, the fields disabled, Output into reads <input> upgraded; the run's Old player folders row, not clean, no notice"],
  ["EU-GUI-04", "drop refs Tracer: both fields disabled with their tooltip, Upgrade enabled at once; the run takes neither argument and ends clean"],
  ["EU-GUI-05", "the output-exists control on, then an Upgrade of egg, co Spring.zip or refs Tracer (a save @ lacks is refused first): the run aborts at once with the one error line naming the folder, the done line reads Aborted, no notice"],
  ["refused / not clean", "drop @ Aesthetics Export VGL26: Upgrade with the discovered save is refused (the save does not hold /@/), the log naming the save and the team; loose on, the upgrade ends not clean with two boots folders left behind"],
  ["EU-GUI-06", "a Browse button opens the stand-in picker and picking a path fills the field as a drop would (the native dialog itself is manual)"],
  ["EU-GUI-07", "the run at about 20 s, a different tool's view active, the shell draft controls' Request the window's close: this view comes forward with the prompt; Close when it ends shows the close stub at the run's end, Keep the window open lets it end plain"],
];

// The view's one screen (the host's layout edit keys its tweaks by it).
const screen = () => "main";

// The link's keys (plans/core/gui.md "Links"): the input dropped, loose, the
// draft controls that change what is drawn, and run=done for a finished run.
function linkKeys() {
  const k = {};
  if (S.input) k.input = S.input;
  if (S.loose) k.loose = "1";
  if (D.outputExists) k.exists = "1";
  if (!D.discovered) k.nosave = "1";
  if (D.runLength === "long") k.long = "1";
  if (S.result) k.run = "done";
  // Only a name of this view's own windows: the host's named windows (About)
  // are the host's `win` key, not this view's dialog.
  const w = document.querySelector("#overlay .window");
  if (w && DLGS[w.dataset.dlg]) k.dialog = w.dataset.dlg;
  return k;
}
// A `dialog` value to its opener: the four fake pickers.
const DLGS = { "pick-export": exportPicker, "pick-models": modelsPicker,
  "pick-savefile": savefilePicker, "pick-output": outputPicker };
const linkWarn = what => console.warn("export-upgrader link: " + what);
function linkOpen(scr, keys) {
  if (scr !== "main") linkWarn("unknown screen " + scr);
  for (const name of ["exists", "nosave", "long"]) {
    const v = keys[name];
    if (v === undefined) continue;
    if (v !== "0" && v !== "1") { linkWarn(name + "=" + v); continue; }
    if (name === "exists") D.outputExists = v === "1";
    if (name === "nosave") D.discovered = v !== "1";
    if (name === "long") D.runLength = v === "1" ? "long" : "short";
  }
  if (keys.input !== undefined) {
    if (INPUTS[keys.input]) setInput(keys.input);
    else linkWarn("input=" + keys.input);
  }
  if (keys.loose !== undefined) {
    if (keys.loose === "1") S.loose = true;
    else if (keys.loose !== "0") linkWarn("loose=" + keys.loose);
  }
  // run=done goes last: the run reads the arguments and controls set above.
  if (keys.run === "done" && S.input) startRun(true);
  else if (keys.run !== undefined && keys.run !== "done") linkWarn("run=" + keys.run);
  if (keys.dialog !== undefined) {
    if (DLGS[keys.dialog]) DLGS[keys.dialog]();
    else linkWarn("dialog=" + keys.dialog);
  }
}
registerTool({ id: "export-upgrader", css: EXPORT_UPGRADER_CSS, draw: drawView,
  controls: draftControls, closeRequest, screen, linkKeys, linkOpen });
})();
