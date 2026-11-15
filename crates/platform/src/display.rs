//! The GPU side: the framebuffer and palette as textures, drawn with one
//! fragment shader.

use std::fmt;
use std::sync::Arc;

use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use crate::{Frame, Overlay};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Window(String),
    Gpu(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Window(e) => write!(f, "window: {e}"),
            Error::Gpu(e) => write!(f, "GPU: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// Where the framebuffer goes in the window. Matches `View` in the shader.
#[derive(Debug, Clone, Copy)]
pub(crate) struct View {
    pub offset: [f32; 2],
    pub scale: f32,
}

impl View {
    /// The UI scale `ui_scale`, centred, or the largest scale that fits
    /// if the frame doesn't fit at it.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn new(window: PhysicalSize<u32>, width: u32, height: u32, ui_scale: f64) -> Self {
        let fit = (window.width as f32 / width.max(1) as f32).min(window.height as f32 / height.max(1) as f32);
        let scale = fit.min(ui_scale as f32).max(f32::EPSILON);
        let offset = [
            ((window.width as f32 - width as f32 * scale) / 2.0).floor(),
            ((window.height as f32 - height as f32 * scale) / 2.0).floor(),
        ];
        View { offset, scale }
    }

    /// The uniform, with `overlay`'s rectangle (empty without one).
    #[allow(clippy::cast_precision_loss)]
    fn bytes(self, overlay: Option<&Overlay<'_>>) -> [u8; 32] {
        let rect = overlay.map_or([0.0; 4], |o| [o.x as f32, o.y as f32, o.width as f32, o.height as f32]);
        let mut b = [0; 32];
        for (i, v) in [self.offset[0], self.offset[1], self.scale, 0.0]
            .into_iter()
            .chain(rect)
            .enumerate()
        {
            b[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
        }
        b
    }
}

struct Textures {
    width: u32,
    height: u32,
    framebuffer: wgpu::Texture,
    /// The overlay's size, at least 1 × 1.
    overlay_size: (u32, u32),
    overlay: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

pub(crate) struct Display {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    palette: wgpu::Texture,
    view: wgpu::Buffer,
    textures: Option<Textures>,
    /// What the window shows, to send the GPU only what changed.
    shown: Option<Shown>,
}

/// The frame last drawn ([`Display::draw`]).
struct Shown {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    palette: [[u8; 3]; 256],
    view: [u8; 32],
    overlay: bool,
}

/// Whether `a` and `b` are the same, eight bytes at a time (WebAssembly's
/// `memcmp` goes a byte at a time).
fn same(a: &[u8], b: &[u8]) -> bool {
    let (wa, ra) = a.as_chunks::<8>();
    let (wb, rb) = b.as_chunks::<8>();
    wa.len() == wb.len() && ra == rb && wa.iter().zip(wb).all(|(x, y)| u64::from_ne_bytes(*x) == u64::from_ne_bytes(*y))
}

/// The rows `from` and `to` (as long, `width` bytes a row) differ in, as
/// `first..end`; `None` if they're the same.
fn changed_rows(from: &[u8], to: &[u8], width: usize) -> Option<std::ops::Range<usize>> {
    let width = width.max(1);
    let rows = from.len().min(to.len()) / width;
    let differs = |y: usize| !same(&from[y * width..(y + 1) * width], &to[y * width..(y + 1) * width]);
    let first = (0..rows).find(|&y| differs(y))?;
    let last = (first..rows).rev().find(|&y| differs(y)).unwrap_or(first);
    Some(first..last + 1)
}

fn gpu(e: impl fmt::Display) -> Error {
    Error::Gpu(e.to_string())
}

/// Where the browser's display comes once its GPU is set up.
#[cfg(target_arch = "wasm32")]
pub(crate) type Pending = std::rc::Rc<std::cell::RefCell<Option<Result<Display, Error>>>>;

fn descriptor(event_loop: &ActiveEventLoop) -> wgpu::InstanceDescriptor {
    wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(event_loop.owned_display_handle()))
}

impl Display {
    /// The display for `window`, waiting for the GPU.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(event_loop: &ActiveEventLoop, window: Arc<Window>) -> Result<Self, Error> {
        let instance = wgpu::Instance::new(descriptor(event_loop));
        pollster::block_on(Display::with_instance(&instance, window))
    }

    /// In the browser, which can't wait: starts setting the GPU up for
    /// `window` (WebGPU where the browser has it, else WebGL2) and gives
    /// the slot the display comes to.
    #[cfg(target_arch = "wasm32")]
    pub fn start(event_loop: &ActiveEventLoop, window: Arc<Window>) -> Pending {
        let slot = Pending::default();
        let to = std::rc::Rc::clone(&slot);
        let desc = descriptor(event_loop);
        wasm_bindgen_futures::spawn_local(async move {
            let instance = wgpu::util::new_instance_with_webgpu_detection(desc).await;
            let display = Display::with_instance(&instance, window).await;
            *to.borrow_mut() = Some(display);
        });
        slot
    }

    async fn with_instance(instance: &wgpu::Instance, window: Arc<Window>) -> Result<Self, Error> {
        let surface = instance.create_surface(window.clone()).map_err(gpu)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(gpu)?;
        #[cfg(target_arch = "wasm32")]
        crate::web::log(&format!(
            "GPU: {:?} ({})",
            adapter.get_info().backend,
            adapter.get_info().name
        ));
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("widget"),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(gpu)?;

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| gpu("the surface doesn't fit the adapter"))?;
        // The shader writes the palette's sRGB values as they are.
        config.format = config.format.remove_srgb_suffix();
        config.view_formats = vec![config.format];
        surface.configure(&device, &config);

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                texture_entry(0, wgpu::TextureSampleType::Uint),
                texture_entry(1, wgpu::TextureSampleType::Float { filterable: false }),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                texture_entry(3, wgpu::TextureSampleType::Float { filterable: false }),
            ],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(config.format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let palette = texture(&device, 256, 1, wgpu::TextureFormat::Rgba8Unorm);
        let view = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Display {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            layout,
            palette,
            view,
            textures: None,
            shown: None,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.shown = None;
        }
    }

    /// Makes the framebuffer texture `width` × `height` and the overlay's
    /// `overlay` (at least 1 × 1); what was shown is forgotten if they're
    /// new.
    fn ensure_textures(&mut self, width: u32, height: u32, overlay: (u32, u32)) {
        let overlay_size = (overlay.0.max(1), overlay.1.max(1));
        if self
            .textures
            .as_ref()
            .is_none_or(|t| (t.width, t.height, t.overlay_size) != (width, height, overlay_size))
        {
            let framebuffer = texture(&self.device, width, height, wgpu::TextureFormat::R8Uint);
            let overlay = texture(
                &self.device,
                overlay_size.0,
                overlay_size.1,
                wgpu::TextureFormat::Rgba8Unorm,
            );
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &framebuffer.create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &self.palette.create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.view.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(
                            &overlay.create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                ],
            });
            self.shown = None;
            self.textures = Some(Textures {
                width,
                height,
                framebuffer,
                overlay_size,
                overlay,
                bind_group,
            });
        }
    }

    /// Draws `frame` at UI scale `ui_scale`: the GPU gets only the rows,
    /// palette and placement that changed since the last frame. With
    /// nothing changed nothing new is shown (the window keeps the last
    /// picture), unless `again` (the window system asked for it).
    pub fn draw(&mut self, frame: &Frame<'_>, ui_scale: f64, again: bool) -> Result<(), Error> {
        let size = PhysicalSize::new(self.config.width, self.config.height);
        let view = View::new(size, frame.width, frame.height, ui_scale);
        // An overlay whose pixels don't fill it isn't shown.
        let overlay = frame
            .overlay
            .as_ref()
            .filter(|o| o.width > 0 && o.height > 0 && o.rgb.len() >= o.width as usize * o.height as usize * 3);
        let view_bytes = view.bytes(overlay);
        // The overlay texture keeps its size while none is shown, so a film
        // that stops and starts doesn't make new textures.
        let overlay_size = overlay.map_or_else(
            || self.textures.as_ref().map_or((1, 1), |t| t.overlay_size),
            |o| (o.width, o.height),
        );
        self.ensure_textures(frame.width, frame.height, overlay_size);
        let width = frame.width as usize;
        let pixels = frame.pixels.get(..width * frame.height as usize).unwrap_or(frame.pixels);
        let (rows, palette, placed) = match &self.shown {
            Some(s) if (s.width, s.height) == (frame.width, frame.height) => (
                changed_rows(&s.pixels, pixels, width),
                s.palette != *frame.palette,
                s.view != view_bytes || s.overlay != overlay.is_some(),
            ),
            _ => (Some(0..frame.height as usize), true, true),
        };
        if rows.is_none() && !palette && !placed && overlay.is_none() && !again {
            return Ok(());
        }
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                self.shown = None;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return Ok(()),
            other @ wgpu::CurrentSurfaceTexture::Validation => return Err(gpu(format!("{other:?}"))),
        };
        if placed {
            self.queue.write_buffer(&self.view, 0, &view_bytes);
        }
        if palette {
            let rgba: Vec<u8> = frame.palette.iter().flat_map(|&[r, g, b]| [r, g, b, 255]).collect();
            write(&self.queue, &self.palette, &rgba, 256, 1, 4);
        }
        let Some(textures) = &self.textures else {
            return Ok(());
        };
        if let Some(r) = &rows {
            write_rows(&self.queue, &textures.framebuffer, pixels, frame.width, r.clone());
        }
        if let Some(o) = overlay {
            let rgba: Vec<u8> = o.rgb[..o.width as usize * o.height as usize * 3]
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|&[r, g, b]| [r, g, b, 255])
                .collect();
            write(&self.queue, &textures.overlay, &rgba, o.width, o.height, 4);
        }

        let target = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &textures.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(output);
        self.remember(frame, pixels, rows, view_bytes, overlay.is_some());
        Ok(())
    }

    /// `frame` as shown, its rows `rows` changed since the last.
    fn remember(
        &mut self,
        frame: &Frame<'_>,
        pixels: &[u8],
        rows: Option<std::ops::Range<usize>>,
        view: [u8; 32],
        overlay: bool,
    ) {
        let width = frame.width as usize;
        match &mut self.shown {
            Some(s) if (s.width, s.height) == (frame.width, frame.height) => {
                if let Some(r) = rows {
                    let span = r.start * width..r.end * width;
                    s.pixels[span.clone()].copy_from_slice(&pixels[span]);
                }
                s.palette = *frame.palette;
                s.view = view;
                s.overlay = overlay;
            }
            _ => {
                self.shown = Some(Shown {
                    width: frame.width,
                    height: frame.height,
                    pixels: pixels.to_vec(),
                    palette: *frame.palette,
                    view,
                    overlay,
                });
            }
        }
    }
}

fn texture_entry(binding: u32, sample_type: wgpu::TextureSampleType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn texture(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

/// Rows `rows` of `pixels` (one byte a pixel, `width` a row) into the
/// same rows of `texture`.
fn write_rows(queue: &wgpu::Queue, texture: &wgpu::Texture, pixels: &[u8], width: u32, rows: std::ops::Range<usize>) {
    let w = width as usize;
    let (Ok(y), Ok(height)) = (u32::try_from(rows.start), u32::try_from(rows.len())) else {
        return;
    };
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 0, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        &pixels[rows.start * w..rows.end * w],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn write(queue: &wgpu::Queue, texture: &wgpu::Texture, data: &[u8], width: u32, height: u32, bytes_per_pixel: u32) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * bytes_per_pixel),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn views() {
        // At the UI scale, centred.
        let v = View::new(PhysicalSize::new(1201, 901), 800, 600, 1.5);
        assert_eq!((v.scale, v.offset), (1.5, [0.0, 0.0]));
        let v = View::new(PhysicalSize::new(1700, 1200), 800, 600, 2.0);
        assert_eq!((v.scale, v.offset), (2.0, [50.0, 0.0]));
        // 640×480 in a smaller desktop: as big as fits.
        let v = View::new(PhysicalSize::new(1200, 900), 640, 480, 2.0);
        assert_eq!((v.scale, v.offset), (1.875, [0.0, 0.0]));
    }

    #[test]
    fn changed_rows_span_the_differences() {
        let a = [0u8; 12];
        let mut b = a;
        assert_eq!(changed_rows(&a, &b, 3), None);
        b[4] = 1;
        assert_eq!(changed_rows(&a, &b, 3), Some(1..2));
        b[11] = 1;
        assert_eq!(changed_rows(&a, &b, 3), Some(1..4));
    }

    /// The shader parses and validates, as `create_shader_module` would
    /// check it at run time.
    #[test]
    fn shader_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("shader.wgsl")).expect("WGSL parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .expect("WGSL validates");
    }
}
