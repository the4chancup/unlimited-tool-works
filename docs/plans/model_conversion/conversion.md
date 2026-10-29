# Unlimited Tool Works — Model conversion plan: Model format conversion

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Model Format Conversion

### SKL and glTF: native integration, not a companion

What exists of this is in `spec/model_conversion/conversion.md` "SKL and glTF: native integration, not a companion".

A glTF-authored model stores its skeleton natively in the glTF `skin` (field mapping in the format
plan, "Skeleton: native glTF skin, not a companion SKL"). When the target is Fox, `ir_to_fmdl`'s
caller reconstructs the `.skl` binary from the IR bones — which `gltf_to_ir` filled from the skin's
joint hierarchy and bind data, applying the inverse coordinate/local-to-model-space mapping.

### A face made for the other engine

The two engines animate eyes differently, so a face model and its diff are made for one engine
(`.tmp/face_diff/findings.md`, the 2026-10-19 Sopdog section): pre-Fox blinks with the lower
eyelid too and assumes the eyes half closed, so its diff says how far to open and close them;
Fox assumes them fully open and rotates the whole eye, eyelids included, which authors
compensate for in the model (eyes pulled deeper into the head, eyelids weighted to the head).
A face folder says which engine its face was made for with a `fox` or `pre-fox` marker file, as a
kit folder says its layout (`spec/aesthetics_export/player_folders.md` "Kit layout marker"); no
marker means made for the target engine. When a face with face weighting (eye and eyelid bones
in its skin) is compiled for the other engine, the compiler applies the transformation the
Sopdog pair shows (`C:\Data\4cc\Lab\sopdog`: a Fox VGL22 face and the pre-Fox VGL24 one hacked
from it, its author's explanation in `chat.txt`): the model's eyes and eyelid weights moved as
the author moved them, and the diff's eye parameters (head floats 0x2C and 0x30, 60 on Fox
against 35 on pre-Fox) and eye, eyelid and brow offsets set for the target. The transformation
is derived from that pair and checked in game on both engines before it is built; if no rule
reproduces it, the compiler reports the face (W, naming the engine it was made for) instead.
We do not pass such a face through unreported, as the compiler does today, because its eyes
misbehave on the other engine with nothing telling the member why. The work follows the
Divegrass Before Christmas editors (worklog "Issues").
