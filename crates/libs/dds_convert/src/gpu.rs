//! A BC7 encoder on a Vulkan (Metal on macOS) device through `block_compression`'s
//! wgpu backend, with a CPU fallback: a texture the device cannot take is
//! encoded by the CPU path, and once an encode fails on the device the mip it
//! happened on and everything after it, in that texture and in later ones, is
//! encoded on the CPU too. Native only.

use std::future::Future;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use block_compression::{CompressionVariant, GpuBlockCompressor};
use wgpu::util::{DeviceExt, TextureDataOrder};

use crate::ConvertError;
use crate::encode::{self, Padded, pad_to_blocks};

/// One submission's pixel budget: a 4096x4096 level 0 encodes in about 0.36 s
/// on the measured GPU, so a slab is about 0.09 s, far under Windows' 2 s GPU
/// timeout.
const SUBMISSION_PIXELS: u32 = 4 * 1024 * 1024;

/// How long the readback waits for the queued slabs: far past the seconds the
/// work costs, so only a stall or a lost device reaches it.
const READBACK_TIMEOUT: Duration = Duration::from_secs(120);

/// A GPU BC7 encoder on a Vulkan (Metal on macOS) device, built by `GpuEncoder::new`
/// (blocking: a caller that must not wait starts it on a thread of its own). Once an
/// encode fails on the device, the encoder stays unavailable and says why.
pub struct GpuEncoder {
    device: wgpu::Device,
    queue: wgpu::Queue,
    compressor: GpuBlockCompressor,
    adapter_name: String,
    max_texture_dimension: u32,
    submission_budget: u32,
    unavailable: Option<String>,
    mips_on_device: usize,
    /// Errors wgpu reports outside an error scope land here instead of panicking.
    uncaptured: Arc<Mutex<Option<String>>>,
}

/// Why no GPU encoder could be made (no Vulkan or Metal adapter, the device refused).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("GPU encoder unavailable: {0}")]
pub struct GpuUnavailable(pub String);

impl GpuEncoder {
    /// Builds an encoder on the machine's Vulkan (Metal on macOS) adapter with
    /// the adapter's own limits, refusing a software fallback adapter (slower
    /// than the CPU encoder). Blocking: compiling the pipelines can take
    /// seconds on a driver whose shader cache is cold, so a caller that must
    /// not wait starts this on a thread of its own.
    pub fn new() -> Result<GpuEncoder, GpuUnavailable> {
        start(wgpu::Backends::VULKAN | wgpu::Backends::METAL)
    }

    /// The adapter's name, for the report of the backend in use.
    pub fn adapter(&self) -> &str {
        &self.adapter_name
    }

    /// The reason the encoder stopped, once an encode failed on the device; `None` while usable.
    pub fn unavailable(&self) -> Option<&str> {
        self.unavailable.as_deref()
    }

    /// How many mips the device has encoded, for the run's report of the backend in use.
    pub fn mips_on_device(&self) -> usize {
        self.mips_on_device
    }

    /// Whether the device can take a `width`x`height` upload for `variant`: [`fits`]
    /// against its 2D texture limit.
    pub(crate) fn accepts(&self, variant: CompressionVariant, width: u32, height: u32) -> bool {
        fits(self.max_texture_dimension, variant, width, height)
    }

    /// Encodes one mip's RGBA8 pixels to `variant` blocks: on the device while
    /// it is usable and `accepts` the texture, on the CPU otherwise. A device
    /// failure is logged once, marks the encoder `unavailable`, and that mip is
    /// encoded on the CPU; errors of the pixels themselves propagate.
    pub(crate) fn encode(
        &mut self,
        variant: CompressionVariant,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<Vec<u8>, ConvertError> {
        if self.unavailable.is_none() {
            let padded = pad_to_blocks(width, height, rgba)?;
            if self.accepts(variant, padded.width, padded.height) {
                // wgpu panics rather than reports some device failures: a lost
                // device inside `Device::poll` goes to `handle_error_fatal`,
                // never to an error scope or the uncaptured hook (wgpu 30's
                // `backend/wgpu_core.rs` `poll`, which returns only what
                // wgpu-core's `WaitIdleError::to_poll_error` converts: a
                // timeout or a wrong submission index). `contained`'s
                // `AssertUnwindSafe` holds because a caught panic marks the encoder
                // unavailable, after which nothing calls the device again.
                match contained(|| self.gpu_encode(variant, &padded)) {
                    Ok(blocks) => {
                        self.mips_on_device += 1;
                        return Ok(blocks);
                    }
                    Err(reason) => {
                        log::warn!(
                            "GPU texture encode failed; this mip and the rest of the run use the CPU encoder: {reason}"
                        );
                        self.unavailable = Some(reason);
                    }
                }
            }
        }
        encode::encode_mip(variant, width, height, rgba)
    }

    /// One padded mip through the device. Error scopes wrap the whole upload,
    /// dispatch and readback so a validation or out-of-memory error, an
    /// uncaptured error, a `map_async` failure or a `poll` failure is a reason
    /// rather than wgpu's default panic.
    fn gpu_encode(
        &mut self,
        variant: CompressionVariant,
        padded: &Padded,
    ) -> Result<Vec<u8>, String> {
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let memory = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let outcome = self.gpu_encode_scoped(variant, padded);
        let memory_error = block_on(memory.pop());
        let validation_error = block_on(validation.pop());
        let uncaptured = self.uncaptured.lock().unwrap().take();
        match outcome {
            Err(reason) => Err(reason),
            Ok(blocks) => {
                match memory_error
                    .or(validation_error)
                    .map(|e| e.to_string())
                    .or(uncaptured)
                {
                    Some(reason) => Err(reason),
                    None => Ok(blocks),
                }
            }
        }
    }

    /// Upload, slab dispatches, readback. One `Rgba8Unorm` texture holds the
    /// padded mip (raw channel values, the non-sRGB view the API requires);
    /// each slab is one task and one compute pass in its own submission so no
    /// dispatch approaches the GPU timeout, and the blocks are read back once.
    fn gpu_encode_scoped(
        &mut self,
        variant: CompressionVariant,
        padded: &Padded,
    ) -> Result<Vec<u8>, String> {
        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some("dds_convert mip"),
                size: wgpu::Extent3d {
                    width: padded.width,
                    height: padded.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            &padded.rgba,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let size = variant.blocks_byte_size(padded.width, padded.height) as u64;
        let blocks = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blocks"),
            size,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let slab_list = slabs(padded.width, padded.height, self.submission_budget);
        for (index, (start, rows)) in slab_list.iter().enumerate() {
            // The byte offset of the slab's first block row.
            let offset = u64::from(*start) / 4 * u64::from(variant.bytes_per_row(padded.width));
            self.compressor.add_compression_task(
                variant,
                &view,
                padded.width,
                *rows,
                &blocks,
                Some(*start),
                Some(u32::try_from(offset).map_err(|_| "block offset past u32".to_string())?),
            );
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("bc7 slab"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("bc7"),
                    timestamp_writes: None,
                });
                self.compressor.compress(&mut pass);
            }
            if index + 1 == slab_list.len() {
                encoder.copy_buffer_to_buffer(&blocks, 0, &staging, 0, size);
            }
            self.queue.submit([encoder.finish()]);
        }
        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Err(error) = sender.send(result) {
                log::debug!("map_async result had no receiver: {error}");
            }
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(READBACK_TIMEOUT),
            })
            .map_err(|error| format!("device poll: {error}"))?;
        receiver
            .recv()
            .map_err(|error| format!("map_async receiver: {error}"))?
            .map_err(|error| format!("map_async: {error}"))?;
        let bytes = slice
            .get_mapped_range()
            .map_err(|error| format!("mapped range: {error}"))?
            .to_vec();
        staging.unmap();
        Ok(bytes)
    }
}

/// Runs `device_work`, a panic inside it becoming a reason like any other
/// device failure (why wgpu panics there: `GpuEncoder::encode`). The caller
/// stops using the device after a reason, which is what makes
/// `AssertUnwindSafe` hold.
fn contained<T>(device_work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    panic::catch_unwind(AssertUnwindSafe(device_work)).unwrap_or_else(|payload| {
        Err(format!(
            "the GPU encode panicked: {}",
            panic_reason(payload.as_ref())
        ))
    })
}

/// The message a caught panic carried, for the `unavailable` reason.
fn panic_reason(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic without a message".to_string())
}

/// Whether a `width`x`height` upload for `variant` fits a device whose 2D
/// textures reach `max_dimension`: the block-aligned padded size within it on
/// both axes, and the blocks buffer within the u32 `blocks_offset` indexing
/// `add_compression_task` takes.
fn fits(max_dimension: u32, variant: CompressionVariant, width: u32, height: u32) -> bool {
    width.div_ceil(4).saturating_mul(4) <= max_dimension
        && height.div_ceil(4).saturating_mul(4) <= max_dimension
        && variant.blocks_byte_size(width, height) <= u32::MAX as usize
}

/// A `width`x`height` mip's row ranges as `(start row, rows)` slabs under
/// `budget` pixels each: whole block rows, the largest multiple of 4 with
/// `width * rows <= budget` and at least 4. `height` is block-aligned, so
/// every slab's rows are a multiple of 4 as `add_compression_task` requires.
fn slabs(width: u32, height: u32, budget: u32) -> Vec<(u32, u32)> {
    let rows_per_slab = ((budget / width) & !3).max(4);
    let mut out = Vec::new();
    let mut start = 0;
    while start < height {
        let rows = rows_per_slab.min(height - start);
        out.push((start, rows));
        start += rows;
    }
    out
}

/// Refuses a software adapter: its compute runs on the CPU, slower than the CPU encoder.
fn refuse_software(device_type: wgpu::DeviceType, name: &str) -> Result<(), GpuUnavailable> {
    match device_type {
        wgpu::DeviceType::Cpu => Err(GpuUnavailable(format!(
            "only a software adapter was found ({name})"
        ))),
        _ => Ok(()),
    }
}

/// Builds an encoder on `backends`, or says why no adapter or device could be
/// made. Separated from [`GpuEncoder::new`] so a test can ask for no backends.
fn start(backends: wgpu::Backends) -> Result<GpuEncoder, GpuUnavailable> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .map_err(|error| GpuUnavailable(format!("no adapter was found: {error}")))?;
    let info = adapter.get_info();
    refuse_software(info.device_type, &info.name)?;
    let adapter_name = info.name;
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("dds_convert bc7"),
        required_features: wgpu::Features::empty(),
        // The adapter's own limits, so the largest texture it can hold fits.
        required_limits: adapter.limits(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .map_err(|error| GpuUnavailable(format!("the device refused: {error}")))?;
    let uncaptured = Arc::new(Mutex::new(None::<String>));
    device.on_uncaptured_error(Arc::new({
        let uncaptured = Arc::clone(&uncaptured);
        move |error: wgpu::Error| {
            let mut slot = uncaptured.lock().unwrap();
            if slot.is_none() {
                *slot = Some(error.to_string());
            }
        }
    }));
    Ok(GpuEncoder {
        max_texture_dimension: device.limits().max_texture_dimension_2d,
        compressor: GpuBlockCompressor::new(device.clone(), queue.clone()),
        device,
        queue,
        adapter_name,
        submission_budget: SUBMISSION_PIXELS,
        unavailable: None,
        mips_on_device: 0,
        uncaptured,
    })
}

/// Drives a future to readiness without an executor: native wgpu resolves
/// `request_adapter`, `request_device` and error-scope pops on their first
/// poll, so no executor crate is needed for them.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = include_bytes!("../tests/fixtures/source.png");
    const PNG_OPAQUE: &[u8] = include_bytes!("../tests/fixtures/source_opaque.png");

    #[test]
    fn no_backends_reports_no_adapter() {
        let Err(error) = start(wgpu::Backends::empty()) else {
            panic!("an encoder on no backends");
        };
        assert!(
            error.0.contains("no adapter was found"),
            "reason: {}",
            error.0
        );
    }

    #[test]
    fn slabs_split_a_4096_mip_at_the_default_budget() {
        assert_eq!(
            slabs(4096, 4096, SUBMISSION_PIXELS),
            vec![(0, 1024), (1024, 1024), (2048, 1024), (3072, 1024)]
        );
    }

    #[test]
    fn slabs_split_at_a_small_budget() {
        assert_eq!(slabs(8, 12, 32), vec![(0, 4), (4, 4), (8, 4)]);
    }

    #[test]
    fn slabs_below_one_block_row_still_align_and_cover() {
        // A budget under `width * 4` still gives 4-row slabs; every slab's
        // rows are a multiple of 4 and the slabs cover the height exactly,
        // a shorter last slab included ((8, 20, 64) ends on 4 of 8 rows).
        for (width, height, budget) in [(8, 12, 8), (64, 64, 64 * 4), (12, 20, 4), (8, 20, 64)] {
            let slab_list = slabs(width, height, budget);
            assert!(!slab_list.is_empty());
            let mut covered = 0;
            for (start, rows) in &slab_list {
                assert_eq!(*start, covered, "gap before slab at {start}");
                assert_eq!(rows % 4, 0, "unaligned slab at {start}");
                covered += rows;
            }
            assert_eq!(covered, height);
        }
        assert_eq!(slabs(8, 20, 64), vec![(0, 8), (8, 8), (16, 4)]);
    }

    #[test]
    fn a_panic_in_device_work_becomes_a_reason() {
        assert_eq!(contained(|| Ok::<u8, String>(7)), Ok(7));
        assert_eq!(
            contained(|| Err::<u8, String>("scoped".to_owned())),
            Err("scoped".to_owned())
        );
        let panicked =
            contained(|| -> Result<u8, String> { panic!("Error in Device::poll: lost") });
        assert_eq!(
            panicked,
            Err("the GPU encode panicked: Error in Device::poll: lost".to_owned())
        );
    }

    #[test]
    fn a_caught_panic_keeps_its_message() {
        let text: Box<dyn std::any::Any + Send> = Box::new("Error in Device::poll: lost");
        assert_eq!(panic_reason(text.as_ref()), "Error in Device::poll: lost");
        let formatted: Box<dyn std::any::Any + Send> = Box::new(format!("lost {}", 7));
        assert_eq!(panic_reason(formatted.as_ref()), "lost 7");
        let other: Box<dyn std::any::Any + Send> = Box::new(7_u32);
        assert_eq!(panic_reason(other.as_ref()), "a panic without a message");
    }

    #[test]
    fn software_adapters_are_refused() {
        let Err(error) = refuse_software(wgpu::DeviceType::Cpu, "llvmpipe") else {
            panic!("a software adapter was accepted");
        };
        assert!(error.0.contains("llvmpipe"), "reason: {}", error.0);
        for device_type in [
            wgpu::DeviceType::DiscreteGpu,
            wgpu::DeviceType::IntegratedGpu,
            wgpu::DeviceType::VirtualGpu,
            wgpu::DeviceType::Other,
        ] {
            assert!(refuse_software(device_type, "gpu").is_ok());
        }
    }

    #[test]
    fn fits_checks_both_axes_and_the_blocks_size() {
        let v = CompressionVariant::BC7(block_compression::BC7Settings::alpha_basic());
        assert!(fits(32, v, 32, 32));
        // The width pads to 36; the height pads to 36.
        assert!(!fits(32, v, 33, 32));
        assert!(!fits(32, v, 32, 33));
        // 16384 x 16384 blocks x 16 bytes is 2^32, past the u32 offset indexing.
        assert!(!fits(70000, v, 65536, 65536));
        // 16384 x 16383 x 16 = 4,294,705,152, the largest size that fits.
        assert!(fits(70000, v, 65536, 65532));
    }

    /// A deterministic RGBA pattern with varied alpha, for textures the GPU
    /// and CPU encoders must not encode identically (on `source.png` they
    /// write the same blocks, which hides which path ran).
    fn patterned_rgba(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        for (i, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            *pixel = [
                (i * 37 % 256) as u8,
                (i * 17 % 256) as u8,
                (i * 11 % 256) as u8,
                (64 + i * 3 % 192) as u8,
            ];
        }
        rgba
    }

    /// PSNR of `decoded` against `source` over every byte, in dB; infinite
    /// for identical data.
    fn psnr(source: &[u8], decoded: &[u8]) -> f64 {
        let squared: f64 = source
            .iter()
            .zip(decoded)
            .map(|(x, y)| f64::from(x.abs_diff(*y)) * f64::from(x.abs_diff(*y)))
            .sum();
        let mse = squared / source.len() as f64;
        if mse == 0.0 {
            f64::INFINITY
        } else {
            10.0 * (255.0 * 255.0 / mse).log10()
        }
    }

    #[test]
    #[ignore = "needs a Vulkan or Metal GPU"]
    fn gpu_bc7_quality_within_1db_of_cpu() {
        let mut gpu = GpuEncoder::new().expect("a Vulkan or Metal adapter");
        // The adapter name is what a fresh request for the same adapter reports.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .expect("a Vulkan or Metal adapter");
        assert_eq!(gpu.adapter(), adapter.get_info().name.as_str());
        // The CPU reference uses the preset the fixture calls for: it pins the
        // GPU's quality against the right encoder settings, so a wrong preset
        // on the GPU path shows rather than degrading both sides together.
        for (name, bytes, expected) in [
            (
                "source.png",
                PNG,
                block_compression::BC7Settings::alpha_basic(),
            ),
            (
                "source_opaque.png",
                PNG_OPAQUE,
                block_compression::BC7Settings::opaque_basic(),
            ),
        ] {
            let decoded = crate::decode(bytes, crate::SourceFormat::Png).unwrap();
            let emit = encode::emitted_mips(&decoded);
            let variant = encode::compression_variant(crate::BlockCodec::Bc7, &emit);
            let cpu_variant = CompressionVariant::BC7(expected);
            for (level, mip) in emit.iter().enumerate() {
                let padded = pad_to_blocks(mip.width, mip.height, &mip.pixels).unwrap();
                let gpu_blocks = gpu.gpu_encode(variant, &padded).unwrap();
                let cpu_blocks =
                    encode::encode_mip(cpu_variant, mip.width, mip.height, &mip.pixels).unwrap();
                let mut gpu_rgba = vec![0u8; padded.rgba.len()];
                block_compression::decode::decompress_blocks_as_rgba8(
                    variant,
                    padded.width,
                    padded.height,
                    &gpu_blocks,
                    &mut gpu_rgba,
                );
                let mut cpu_rgba = vec![0u8; padded.rgba.len()];
                block_compression::decode::decompress_blocks_as_rgba8(
                    cpu_variant,
                    padded.width,
                    padded.height,
                    &cpu_blocks,
                    &mut cpu_rgba,
                );
                let gpu_psnr = psnr(&padded.rgba, &gpu_rgba);
                let cpu_psnr = psnr(&padded.rgba, &cpu_rgba);
                assert!(
                    gpu_psnr >= cpu_psnr - 1.0,
                    "{name} mip {level}: gpu {gpu_psnr:.2} dB vs cpu {cpu_psnr:.2} dB"
                );
            }
        }
        assert_eq!(gpu.unavailable(), None);
    }

    #[test]
    #[ignore = "needs a Vulkan or Metal GPU"]
    fn slabs_do_not_change_blocks() {
        let mut gpu = GpuEncoder::new().expect("a Vulkan or Metal adapter");
        let decoded = crate::decode(PNG, crate::SourceFormat::Png).unwrap();
        let rgba = crate::resize(&decoded.mips[0], decoded.width, decoded.height, 64, 64).unwrap();
        let padded = pad_to_blocks(64, 64, &rgba).unwrap();
        let variant = CompressionVariant::BC7(block_compression::BC7Settings::alpha_basic());
        let whole = gpu.gpu_encode(variant, &padded).unwrap();
        gpu.submission_budget = 64 * 4;
        let slabbed = gpu.gpu_encode(variant, &padded).unwrap();
        assert_eq!(whole, slabbed);
        assert_eq!(gpu.unavailable(), None);
    }

    #[test]
    #[ignore = "needs a Vulkan or Metal GPU"]
    fn unaligned_alpha_source_through_gpu_keeps_container() {
        let mut gpu = GpuEncoder::new().expect("a Vulkan or Metal adapter");
        // 6x10 with alpha: unaligned in both axes, so the padding path and
        // BC7 both run; PES 21 keeps it an FTEX.
        let decoded = crate::Decoded {
            width: 6,
            height: 10,
            mips: vec![patterned_rgba(6, 10)],
            blocks: None,
            authored_mips: false,
            uncompressed: false,
        };
        let emit = encode::emitted_mips(&decoded);
        let gpu_blocks =
            encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, Some(&mut gpu)).unwrap();
        let cpu_blocks = encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, None).unwrap();
        let format = encode::pixel_format(crate::BlockCodec::Bc7);
        let gpu_out =
            encode::container(pes_version::PesVersion::Pes21, format, 6, 10, &gpu_blocks).unwrap();
        let cpu_out =
            encode::container(pes_version::PesVersion::Pes21, format, 6, 10, &cpu_blocks).unwrap();
        let info = ftex::info(&gpu_out).unwrap();
        assert_eq!(info, ftex::info(&cpu_out).unwrap());
        // The 64-byte header holds layout fields only; the mip records after
        // it carry per-mip uncompressed sizes, which compression must not
        // change (compressed sizes and offsets may differ and are not compared).
        assert_eq!(&gpu_out[..64], &cpu_out[..64], "ftex header");
        for level in 0..u32::from(info.mipmaps) {
            let at = 64 + level as usize * 16 + 4;
            let size = u32::from_le_bytes(gpu_out[at..at + 4].try_into().unwrap());
            assert_eq!(
                size as usize,
                ftex::mip_size(info.format, 6, 10, 1, level),
                "mip {level} uncompressed size"
            );
        }
        assert_eq!(gpu.unavailable(), None);
        assert_eq!(gpu.mips_on_device(), emit.len(), "the GPU took every mip");
    }

    #[test]
    #[ignore = "needs a Vulkan or Metal GPU"]
    fn device_failure_falls_back_to_cpu_and_stays_unavailable() {
        let mut gpu = GpuEncoder::new().expect("a Vulkan or Metal adapter");
        let limit = gpu.device.limits().max_texture_dimension_2d;
        // `limit + 4` wide: `accepts` passes it, the device cannot hold it.
        gpu.max_texture_dimension = u32::MAX;
        let width = limit + 4;
        let decoded = crate::Decoded {
            width,
            height: 4,
            mips: vec![patterned_rgba(width, 4)],
            blocks: None,
            authored_mips: false,
            uncompressed: false,
        };
        let emit = encode::emitted_mips(&decoded);
        let gpu_blocks =
            encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, Some(&mut gpu)).unwrap();
        let cpu_blocks = encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, None).unwrap();
        assert_eq!(gpu_blocks, cpu_blocks);
        assert!(
            gpu.unavailable().is_some(),
            "the device failure must mark the encoder"
        );
        assert_eq!(gpu.mips_on_device(), 0);
        // Once down, it stays down: a texture it could take encodes on the CPU too.
        let decoded = crate::decode(PNG_OPAQUE, crate::SourceFormat::Png).unwrap();
        let emit = encode::emitted_mips(&decoded);
        let gpu_blocks =
            encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, Some(&mut gpu)).unwrap();
        let cpu_blocks = encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, None).unwrap();
        assert_eq!(gpu_blocks, cpu_blocks);
        assert!(gpu.unavailable().is_some());
        assert_eq!(gpu.mips_on_device(), 0);
    }

    #[test]
    #[ignore = "needs a Vulkan or Metal GPU"]
    fn too_large_texture_falls_back_to_cpu() {
        let mut gpu = GpuEncoder::new().expect("a Vulkan or Metal adapter");
        gpu.max_texture_dimension = 32;
        let decoded = crate::Decoded {
            width: 64,
            height: 64,
            mips: vec![patterned_rgba(64, 64)],
            blocks: None,
            authored_mips: false,
            uncompressed: false,
        };
        let emit = encode::emitted_mips(&decoded);
        let gpu_blocks =
            encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, Some(&mut gpu)).unwrap();
        let cpu_blocks = encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, None).unwrap();
        assert_eq!(gpu_blocks, cpu_blocks);
        assert_eq!(gpu.unavailable(), None);
        assert_eq!(gpu.mips_on_device(), 0);
    }

    #[test]
    #[ignore = "needs a Vulkan or Metal GPU"]
    fn lost_device_falls_back_to_cpu() {
        let mut gpu = GpuEncoder::new().expect("a Vulkan or Metal adapter");
        gpu.device.destroy();
        let decoded = crate::Decoded {
            width: 64,
            height: 64,
            mips: vec![patterned_rgba(64, 64)],
            blocks: None,
            authored_mips: false,
            uncompressed: false,
        };
        let emit = encode::emitted_mips(&decoded);
        let gpu_blocks =
            encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, Some(&mut gpu)).unwrap();
        let cpu_blocks = encode::encode_mips(crate::BlockCodec::Bc7, &emit, None, None).unwrap();
        assert_eq!(gpu_blocks, cpu_blocks);
        assert!(gpu.unavailable().is_some());
        assert_eq!(gpu.mips_on_device(), 0);
    }
}
