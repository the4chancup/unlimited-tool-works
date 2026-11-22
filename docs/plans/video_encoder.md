# Unlimited Tool Works: Video encoder plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The Video encoder (`crates/tools/video_encoder`, tool id `video-encoder`, sidebar label "Video
encoder") makes the USM videos a cup ships: the season's swipe, the competition wipes, the menu
movies and the stadium billboards. It replaces CRI's `medianoche.exe` (about 2012), which bent's
2015 batch files drive (`C:/Data/4cc/Tools_4cc/4CC_PES15_tool_compilation_v0.1_alpha/`,
`movies/`): a closed program nobody can fix or rebuild, and today the only one that writes a
USM. No phase holds it yet. This plan defines the tool first; whether the browser edition
offers it and whether it plays videos are settled against that definition.

## What it reads

- **A PNG image sequence**, in every version of the tool: anyone with an image editor can make
  a simple swipe for their own event, with no video editor, and a PNG carries the transparency
  every swipe and wipe needs. The executable already decodes PNG.
- **The video files the cup's swipe makers export**, once their format is known (asked:
  `docs/maintainer/questions.html`, `swipe-source-files`). Its decoder is weighed against the
  size limits below before it is planned.

## What it writes

A USM as the cup's own are: MPEG-1 video, a second MPEG-1 stream for the transparency, no
sound. Only the two menu movies carry sound, in CRI's HCA codec, which has no open encoder; no
swipe, wipe or billboard does. The container, as observed in every USM the cup ships, is in
[`libs/usm.md`](libs/usm.md). The MPEG-1 encoder is the suite's own, in Rust: MPEG-1 is a small,
old standard, estimated at well under 1 MB of code, while ffmpeg linked in would bring a C build
into every platform's build and, estimated, 5 to 15 MB even cut down to a few decoders.

## Playing a USM

The tool plays a USM in its window, the one it wrote or one from the game or a DLC, if that
stays small: an MPEG-1 decoder of the suite's own, estimated like the encoder at well under
1 MB, with no sound at first. If it does not, the tool plays nothing, and members watch the
video they exported in their own player before encoding it.

## Size and editions

- The tool adds less than 20 MB to the desktop executable.
- Limited Tool Works, the browser edition, offers it if it adds little there as well. The
  PNG path, the encoder and the player are estimated together at under 2 MB; a video file's
  decoder is weighed on its own.

Every size here is an estimate, measured when a phase opens.

## To define

- The window: what a member picks (where the video goes in the game: the swipe, a wipe, a
  billboard; each place's size and frame rate, `libs/usm.md` "Sizes and rates"), a PNG
  sequence's frame rate, what the window shows before and after encoding.
- How a written USM reaches the game: a file the member drops into a DLC folder, or one the
  Compiler packs.
