# Unlimited Tool Works — Library crates plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## First-release desktop GPU BC7

GPU BC7 for PES 19–21 is a first-release (1.0.0) target, using `block_compression`'s existing wgpu
backend inside `dds_convert`, built in Phase 16 on the Team compiler's texture step; Release 0.1.0
ships the CPU encoder only (`plans/core/development_plan.md` "Releases").

The library supplies the kernels; Studio implements bounded batching, upload/readback, pipeline
integration, and fallback. It can remain the permanent GPU backend. BC1/BC3 stay on the CPU
initially; already-compatible compressed inputs and cache hits need no encoding at all.

Until this lands, no compile encodes BC7 (`spec/libs/dds_convert.md` "Target codecs and CPU
encoder"). After it, a machine with no usable GPU encodes BC3/BC1, not the CPU's BC7, so CPU
mode below means the deterministic reference for tests, not a compile's fallback.

- **Auto / CPU:** Auto is the desktop default, using the GPU when ready, supported, and beneficial.
  CPU mode explicitly selects the deterministic reference. Unsupported/unready GPUs and recoverable
  GPU failures fall back to CPU; report the active backend and fallback reason. The CLI can obtain
  a headless compute device without opening the GUI.
- **Compression backend:** prefer Vulkan on Windows/Linux and Metal on macOS. Studio selects the
  wgpu device/queue passed to `GpuBlockCompressor`; this does not require a different encoder or
  custom kernels. If that backend is unavailable or unsuitable, use CPU, with no automatic DX12
  fallback initially. The library documents slow BC7 pipeline creation specifically under DX12's
  DXC shader compiler; do not reintroduce that path through fallback. This policy concerns texture
  compression only: the GUI draws through Direct3D 12 on Windows (`plans/core/gui.md` "The
  graphics backend"), which does not move the encoder off Vulkan.
- **Cold-start behavior:** measure cold pipeline creation on Vulkan/Metal and upload/readback cost
  early. Prepare off the UI thread, reuse the compute device and pipelines, and allow CPU work while
  GPU preparation is pending. A cold first compile and end-to-end throughput matter more than
  kernel-only numbers; avoiding the documented DX12 issue does not guarantee fast startup on every
  Vulkan/Metal driver.
- **Bounded work:** coordinate GPU batches with the existing memory budget, cancellation, and writer
  progress. Read compressed blocks back for packing; avoid unbounded submission from rayon workers.
  All consumers of a shared texture conversion reuse one encoded result, including after fallback,
  rather than creating conflicting CPU/GPU copies of the same output.
- **Integration constraints:** use a compatible suite-wide wgpu version; query adapter capabilities
  and limits, not Windows display enumeration. Preserve raw channel values and mip dimensions when
  uploading, including the API's non-sRGB view requirement and block-aligned edge padding.
- **Verification:** both implementations need tests against real opaque/alpha/normal-map fixtures.
  Check supported target versions, startup, full-path timing, resource pressure, cancellation, and
  fallback on representative hardware. No measured speedup is assumed in this plan.

**Measured (2026-10-19, AMD Radeon RX 9060 XT, 16 logical CPUs; `.tmp/phase_16/16x/findings.md`).** On
real 4096² kit textures with alpha (13 mips, BC7 `alpha_basic`), Vulkan encodes a chain in
0.48 s once warm, against 16.1 s per chain with all 16 CPU threads busy (134.5 s on one), at
the same quality (level 0 against the source: 67.35 dB PSNR, largest channel difference 3, the
CPU's 67.31 dB and 3).

Building the BC1-5 and BC7 pipelines took 16.3 s in the first process after the driver was
installed and 25-27 ms in every later one: the driver's shader cache keeps them, so the cold
start is paid once per driver install or update, not per compile (not yet measured on NVIDIA or
Intel).

DX12 is worse on every count (37 s cold, 1.1 s per chain, a largest difference of 18), and one
submission of a whole 8192² chain ran about 6 s on it and lost the device to Windows' GPU
timeout, which Vulkan's 2.8 s survived but is the same risk.

So:

- **The encoder is Vulkan's (Metal's on macOS), started on its own thread.** The warm start
  costs less than one 1024² BC7 encode on the CPU, so a ready GPU takes every BC7 texture. A
  start not finished yet (the rare cold one) leaves the CPU encoding, and the GPU takes the
  textures still waiting once it is ready. Whether the GPU starts at all is decided by whether
  the version's compile has BC7 work (it starts with the Team compiler, below), not by a
  predicted cost: a cold start cannot be predicted.
- **Submissions are bounded in pixels, not textures:** each one encodes at most a fixed slab of
  rows (`add_compression_task`'s row offset), sized so it runs a small fraction of the 2 s
  timeout on the measured GPU, whatever the texture's size.
- **A GPU failure is final:** the mip it happened on and everything after it are encoded on the
  CPU, the encoder reports itself unavailable with the reason, and the rest of the run stays on
  the CPU. The mips the device encoded before it are kept (a texture of mixed GPU and CPU mips
  is valid BC7). A lost device is such a failure too, though wgpu panics on it rather than
  returning an error. A software adapter (a Vulkan driver running on the CPU) is refused at the
  start, being slower than the CPU encoder.
- **A driver that crashes is found in a child process, once per driver.** A Vulkan driver can
  report full support and then fault inside an ordinary call, taking the whole process down
  with no error to fall back from: Intel's HD Graphics 620 driver 31.0.101.2137 does so in
  `igvk64.dll` while wgpu creates its first descriptor pool (`Device::new`), on every start,
  as the GUI met it on 0.0.1's clean-machine check (`plans/core/gui.md` "The graphics
  backend").

  Before a compile first starts the encoder on a given adapter, the exe starts itself hidden
  with a probe argument (as the crash monitor is started, kept out of `--help`). The child
  creates the device on the encoder's backend, builds its pipelines and encodes one block, and
  exits 0. A crash, a non-zero exit or no answer within a bound sized to the measured cold
  start turns the GPU off for that adapter: the compile goes on on the CPU (BC3/BC1 where BC7
  needs the GPU, as above) and its report names the probe's outcome.

  The result is kept in the data folder keyed by the adapter's vendor, device and driver
  version, which wgpu reports without creating a device (the laptop listed its adapters before
  the crash), so a driver update probes again and nothing else does. The probe's pipeline
  build also warms the driver's shader cache, so the cold start it pays is not paid again in
  the compile. The alternative, a list of known bad drivers, only covers drivers someone has
  already seen crash.
- **The encoder starts when the Team compiler opens, not at the first Compile** (the
  maintainer, 2026-11-02), on a PES version whose compile encodes BC7 (19-21; on 15-18 no
  GPU starts): the probe when the adapter's driver has no recorded outcome, then the
  encoder on its thread, both off the UI thread, so a Compile clicked a while later finds it
  ready.

  What it saves depends on the driver's shader cache. On the measured GPU a cold cache, the
  first start after a driver install or update, costs 16.3 s, which the member then spends
  looking at the tool rather than waiting on the compile. A warm one costs 25-27 ms of
  pipeline building plus the device's creation, a fraction of a second.

  A Compile clicked before it is ready goes on as above: the CPU encodes and the GPU joins.
  Leaving the tool keeps the encoder for the rest of the session, as a compile would.

**The workload the speed is judged on.** A Studio export's textures are PNG by design: members
edit them losslessly, and flat colors make a PNG smaller than its DDS. So on PES 19-21 nearly
every texture of a compile is encoded, not passed through. A player's textures add up to about
one 4096² chain, and a team has seven kits, so a team is roughly 30 chains: about 15 s of
encoding at the 0.48 s measured above, on a fairly high-end GPU, and 16 s per chain on the CPU
encoder, too slow for a suite whose first goal is speed.

A full-cup compile is run once per set of exports, so its every texture is encoded cold: no
conversion cache, in memory or on disk, shortens it, and the encoder's own speed is the floor.
The levers left are the encoder's:

- a faster BC7 preset (`fast`, `very_fast`, `ultra_fast`);
- several textures in flight on the GPU at once rather than one;
- flat 4×4 blocks (common in PNG kits) written directly and kept out of the encoder's search,
  BC7 blocks being independent.

A faster preset is taken only if its quality stays above BC3's; on the CPU the same holds for a
SIMD encoder (`intel_tex_2`'s ISPC kernels) against the current one. If no CPU BC7 gets close, a
compile for PES 19-21 that has no GPU writes BC3, as for PES 15-18 (the maintainer's fallback,
2026-10-21).

Writing our own GPU encoder is the last resort, only if these fall short: it is a permanent
maintenance item the library spares us. The measurement deciding all this is worklog 16.x c.

**Built ahead (16.x a, before Release 0.1.0):** the encoder itself, in `dds_convert` behind
`cfg(not(target_arch = "wasm32"))`, with its quality and fallback tests. Wiring it into the
Team compiler's texture step stays after 0.1.0, which ships the CPU encoder only.

**Reproducibility:** explicit CPU mode remains the byte-reproducible reference. The user accepts
valid GPU-encoded texture bytes differing across GPUs/drivers; containing CPKs may differ as a
consequence. GPU output must still satisfy decoded-content/quality checks. Asset paths, model/bin
output, ordering, and collision decisions remain deterministic. The cache distinguishes encoder
implementations/configurations and invalidates on device changes.

## Future Features

### Custom GPU optimization and browser acceleration

Custom maximum-throughput kernels are considered only if measurements expose a worthwhile gap in
the library backend. Possible algorithm references include AMD Compressonator and bc7e, subject to
normal license and quality checks. Browser GPU acceleration remains deferred; its CPU path remains
available. Neither requires a second bespoke encoder for the initial desktop release.

## `vtree` path types

What exists of this is in `spec/libs/README.md` "`vtree` path types".

Filesystem source providers also enforce containment after resolving symlinks/junctions; a safe
virtual path alone does not prove that its physical source remains inside the export.
