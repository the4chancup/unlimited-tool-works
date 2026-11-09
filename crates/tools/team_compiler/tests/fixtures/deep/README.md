# Deep-pass fixtures (step 4.7)

Written by the lead; the generators are kept under `scripts/provenance/fixtures/`.

| File | Contents | For |
|---|---|---|
| `boots_far.fmdl` | the tracer export's `boots.fmdl` (`../tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/`) with the first vertex of its first mesh moved to (6000, 0, 0): one vertex past the 5000-unit limit, nothing else changed. Written through `fmdl`, so it is 64 bytes shorter than the source, whose own three `fmdl_weights_not_normalized` findings it keeps (`deep_far_vertex.rs`); its `dummy_kit` directory later respelled `common/team_folder/` from the community's `common/000/` (`fmdl_team_folder.rs`, step 0.0.2-poc12), then the stand-in itself `team_kit.dds` (`fmdl_team_kit.rs`, the same step) | TC-CHK-01: `vertex_too_far_from_origin` on a folder export, built by the test |
| `co Midcup Far.7z` | `Players/05 - Striker/boots.fmdl` (the bytes of `boots_far.fmdl`) and `Players/05 - Striker/glove_l.fmdl` (the tracer's, with no far vertex; 7-Zip makes no solid block of a single file), solid LZMA2 (`7z a -t7z -ms=on -m0=lzma2`; 7-Zip reports `Solid = +`) (`deep_fixtures.py`) | TC-CHK-02: the same finding from a solid `.7z` |
