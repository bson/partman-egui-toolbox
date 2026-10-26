// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A 3D scene drawn by the GPU in an egui canvas, turned about by
//! dragging: coloured meshes, as a part's model on its board, lit from
//! over the viewer's shoulder, the nearest shown where they overlap.
//!
//! It draws with the app's own wgpu device, from the UI code: into a
//! texture egui's renderer is given once, multisampled, with a depth
//! buffer (egui's meshes have none), again only when the view, the scene
//! or the canvas's size changes. egui then shows the texture as it would
//! any picture, so nothing has to match how egui draws its own frame. The
//! app hands its wgpu render state over once, with `install`; without it
//! (another renderer, a test that doesn't render), the canvas says so.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use egui::{Color32, Id, Rect, Response, Sense, TextureId, Ui, Vec2, pos2, vec2};
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;

/// Coloured triangles: points, each triangle three of their indices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub points: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub color: Color32,
}

/// What's drawn: meshes in one space, z up, x right, y away (a board's
/// mm, say). Faces are lit alike from either side, as models' faces don't
/// all face out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub meshes: Vec<Mesh>,
}

/// How the scene is seen: turned about z (yaw), tipped toward the viewer
/// (pitch, 0 from straight above), zoomed, and moved (in points).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub pan: Vec2,
}

impl Default for View {
    fn default() -> Self {
        View { yaw: -0.5, pitch: 0.9, zoom: 1.0, pan: Vec2::ZERO }
    }
}

/// Points scrolled for the scene to grow by e: a wheel notch (egui's 40
/// points) about a fifth.
const SCROLL_ZOOM: f32 = 200.0;

/// Behind the scene.
pub const BACKGROUND: Color32 = Color32::from_rgb(52, 56, 64);

/// Samples a pixel is drawn from: without, a part's thin bevels and edges
/// show only here and there, as dashes.
const SAMPLES: u32 = 4;

const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// A canvas not drawn for this many frames gives its textures back.
const KEEP_FRAMES: u64 = 120;

/// Scenes kept on the GPU, the most recently drawn.
const KEEP_SCENES: usize = 8;

/// The GPU's side, made by `install` and kept in egui's memory.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Arc<egui::mutex::RwLock<egui_wgpu::Renderer>>,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    /// Per canvas, by its id
    targets: HashMap<Id, Target>,
    /// Per scene, by the app's key, with the frame it was last drawn in
    scenes: HashMap<String, (Uploaded, u64)>,
}

/// A canvas's textures, as big as it is in pixels.
struct Target {
    size: [u32; 2],
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolved: wgpu::Texture,
    texture: TextureId,
    uniforms: wgpu::Buffer,
    bind: wgpu::BindGroup,
    /// What it was last drawn for, so it isn't drawn again for the same
    drawn: Option<(View, String)>,
    frame: u64,
}

/// A scene's triangles on the GPU, and its bounds, to fit it to a canvas.
struct Uploaded {
    vertices: wgpu::Buffer,
    count: u32,
    min: [f32; 3],
    max: [f32; 3],
}

#[derive(Clone)]
struct Shared(Arc<Mutex<Gpu>>);

fn id() -> Id {
    Id::new("partman_3d")
}

/// Gives the canvases the app's wgpu render state (eframe's
/// `CreationContext::wgpu_render_state`), once, at the start.
pub fn install(ctx: &egui::Context, rs: &egui_wgpu::RenderState) {
    let device = &rs.device;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("partman_3d"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("partman_3d"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("partman_3d"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("partman_3d"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: VERTEX as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4],
            })],
        },
        primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState { count: SAMPLES, mask: !0, alpha_to_coverage_enabled: false },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState { format: COLOR, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let gpu = Gpu { device: rs.device.clone(), queue: rs.queue.clone(), renderer: rs.renderer.clone(), pipeline, layout, targets: HashMap::new(), scenes: HashMap::new() };
    ctx.data_mut(|d| d.insert_temp(id(), Shared(Arc::new(Mutex::new(gpu)))));
}

/// Whether `install` has been, and the canvas can draw.
pub fn installed(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Shared>(id())).is_some()
}

/// Draws the scene in a canvas as wide as there's room and height tall:
/// dragging turns it, right-dragging moves it, the wheel, a two-finger
/// scroll or pinching zooms, a double click puts it back. key names the scene: the
/// same key, the same scene, kept on the GPU, so `scene` is only called
/// for one it hasn't got.
pub fn canvas(ui: &mut Ui, view: &mut View, key: &str, scene: impl FnOnce() -> Scene, height: f32) -> Response {
    let (response, painter) = ui.allocate_painter(vec2(ui.available_width(), height), Sense::click_and_drag());
    let rect = response.rect;
    steer(ui, &response, view);
    let Some(Shared(gpu)) = ui.ctx().data(|d| d.get_temp::<Shared>(id())) else {
        painter.rect_filled(rect, 0.0, BACKGROUND);
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "3D needs the wgpu renderer", egui::FontId::proportional(14.0), Color32::LIGHT_GRAY);
        return response;
    };
    let ppp = ui.ctx().pixels_per_point();
    let size = [(rect.width() * ppp).round().max(1.0) as u32, (rect.height() * ppp).round().max(1.0) as u32];
    let frame = ui.ctx().cumulative_frame_nr();
    let texture = {
        let mut gpu = gpu.lock().unwrap_or_else(|e| e.into_inner());
        gpu.sweep(frame);
        if !gpu.scenes.contains_key(key) {
            let up = gpu.upload(&scene());
            gpu.scenes.insert(key.to_string(), (up, frame));
            gpu.forget_scenes();
        }
        if let Some(s) = gpu.scenes.get_mut(key) {
            s.1 = frame;
        }
        gpu.draw(response.id, size, *view, key, ppp, frame)
    };
    painter.image(texture, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    response.on_hover_text("Drag to turn it, right-drag to move it, scroll or pinch to zoom, double-click to put it back")
}

/// How much to zoom by this frame, for a canvas the pointer's over:
/// pinching or ctrl and the wheel, as egui has them, and the wheel or a
/// two-finger scroll, up zooming in. The scrolling is taken, so the
/// scroll area around the canvas doesn't scroll as well. For any canvas
/// that zooms, so they all zoom alike.
pub fn scroll_zoom(ui: &Ui) -> f32 {
    ui.input_mut(|i| {
        let scrolled = i.smooth_scroll_delta.x + i.smooth_scroll_delta.y;
        i.smooth_scroll_delta = Vec2::ZERO;
        i.zoom_delta() * (scrolled / SCROLL_ZOOM).exp()
    })
}

/// The view as the pointer changes it.
fn steer(ui: &Ui, response: &Response, view: &mut View) {
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        view.yaw += d.x * 0.01;
        view.pitch = (view.pitch + d.y * 0.01).clamp(0.0, std::f32::consts::PI);
    }
    if response.dragged_by(egui::PointerButton::Secondary) {
        view.pan += response.drag_delta();
    }
    if response.hovered() {
        let zoom = scroll_zoom(ui);
        if zoom != 1.0 {
            view.zoom = (view.zoom * zoom).clamp(0.2, 50.0);
        }
    }
    if response.double_clicked() {
        *view = View::default();
    }
}

/// A vertex: position, the face's normal, colour (RGBA bytes).
const VERTEX: usize = 3 * 4 + 3 * 4 + 4;

impl Gpu {
    /// The scene's triangles, each with its own corners and the face's
    /// normal, as faces are drawn flat.
    fn upload(&self, scene: &Scene) -> Uploaded {
        let mut bytes = Vec::new();
        let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
        let mut count = 0;
        for m in &scene.meshes {
            let rgba = m.color.to_array();
            for t in &m.triangles {
                let Some(p) = t.iter().map(|&i| m.points.get(i as usize).copied()).collect::<Option<Vec<[f32; 3]>>>() else { continue };
                let n = normal(p[0], p[1], p[2]);
                for q in &p {
                    for d in 0..3 {
                        min[d] = min[d].min(q[d]);
                        max[d] = max[d].max(q[d]);
                    }
                    for v in q.iter().chain(&n) {
                        bytes.extend_from_slice(&v.to_le_bytes());
                    }
                    bytes.extend_from_slice(&rgba);
                }
                count += 3;
            }
        }
        if count == 0 {
            (min, max) = ([0.0; 3], [0.0; 3]);
            bytes.resize(VERTEX, 0);
        }
        let vertices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("partman_3d scene"), contents: &bytes, usage: wgpu::BufferUsages::VERTEX });
        Uploaded { vertices, count, min, max }
    }

    /// Scenes past the most kept, the least recently drawn, let go.
    fn forget_scenes(&mut self) {
        while self.scenes.len() > KEEP_SCENES {
            let Some(oldest) = self.scenes.iter().min_by_key(|(_, (_, f))| *f).map(|(k, _)| k.clone()) else { break };
            self.scenes.remove(&oldest);
        }
    }

    /// Canvases not drawn lately give their textures back.
    fn sweep(&mut self, frame: u64) {
        let gone: Vec<Id> = self.targets.iter().filter(|(_, t)| frame.saturating_sub(t.frame) > KEEP_FRAMES).map(|(id, _)| *id).collect();
        for id in gone {
            if let Some(t) = self.targets.remove(&id) {
                self.renderer.write().free_texture(&t.texture);
            }
        }
    }

    /// The canvas's texture, the scene drawn into it unless it's there
    /// already as it'd be drawn now.
    fn draw(&mut self, canvas: Id, size: [u32; 2], view: View, key: &str, ppp: f32, frame: u64) -> TextureId {
        if self.targets.get(&canvas).is_none_or(|t| t.size != size) {
            let t = self.target(size);
            if let Some(old) = self.targets.insert(canvas, t) {
                self.renderer.write().free_texture(&old.texture);
            }
        }
        let Gpu { device, queue, pipeline, scenes, targets, .. } = self;
        let target = targets.get_mut(&canvas).expect("made above");
        target.frame = frame;
        let now = Some((view, key.to_string()));
        if target.drawn == now {
            return target.texture;
        }
        let Some((scene, _)) = scenes.get(key) else { return target.texture };
        queue.write_buffer(&target.uniforms, 0, &uniforms(scene, &view, size, ppp));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("partman_3d") });
        {
            let bg = BACKGROUND.to_normalized_gamma_f32();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("partman_3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color,
                    depth_slice: None,
                    resolve_target: Some(&target.resolved.create_view(&wgpu::TextureViewDescriptor::default())),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: bg[0] as f64, g: bg[1] as f64, b: bg[2] as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &target.bind, &[]);
            pass.set_vertex_buffer(0, scene.vertices.slice(..));
            pass.draw(0..scene.count, 0..1);
        }
        queue.submit([encoder.finish()]);
        target.drawn = now;
        target.texture
    }

    /// A canvas's textures at size, the one shown given to egui.
    fn target(&mut self, size: [u32; 2]) -> Target {
        let extent = wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 };
        let texture = |label, format, samples, usage| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let view = |t: wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        let color = view(texture("partman_3d color", COLOR, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT));
        let depth = view(texture("partman_3d depth", DEPTH, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT));
        let resolved = texture("partman_3d picture", COLOR, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        let shown = resolved.create_view(&wgpu::TextureViewDescriptor::default());
        let texture = self.renderer.write().register_native_texture(&self.device, &shown, wgpu::FilterMode::Linear);
        let uniforms = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("partman_3d view"), size: UNIFORMS as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("partman_3d"), layout: &self.layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() }] });
        Target { size, color, depth, resolved, texture, uniforms, bind, drawn: None, frame: 0 }
    }
}

fn normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len == 0.0 { [0.0, 0.0, 1.0] } else { n.map(|c| c / len) }
}

/// The turn into the view: about z by yaw, then tipped back by pitch so
/// the scene's top leans away; z then points at the viewer. Rows.
fn turn(view: &View) -> [[f32; 3]; 3] {
    let (sy, cy) = view.yaw.sin_cos();
    let (sp, cp) = (-view.pitch).sin_cos();
    // rot_x(-pitch) · rot_z(yaw)
    [[cy, -sy, 0.0], [cp * sy, cp * cy, -sp], [sp * sy, sp * cy, cp]]
}

fn apply(m: &[[f32; 3]; 3], p: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|r| m[r][0] * p[0] + m[r][1] * p[1] + m[r][2] * p[2])
}

/// The uniforms: the turn (three rows, padded), then the scale and offset
/// from the turned scene to clip space, x, y and z.
const UNIFORMS: usize = 5 * 16;

/// Fitted as KiCad's viewer would: the turned scene's bounds (its box's
/// corners turned) filling the canvas within a margin, then zoomed and
/// moved; depth across the box's diagonal, so none of it is clipped.
fn uniforms(scene: &Uploaded, view: &View, size: [u32; 2], ppp: f32) -> Vec<u8> {
    let m = turn(view);
    let (lo, hi) = (scene.min, scene.max);
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for i in 0..8 {
        let c = [if i & 1 == 0 { lo[0] } else { hi[0] }, if i & 2 == 0 { lo[1] } else { hi[1] }, if i & 4 == 0 { lo[2] } else { hi[2] }];
        let q = apply(&m, c);
        for d in 0..3 {
            min[d] = min[d].min(q[d]);
            max[d] = max[d].max(q[d]);
        }
    }
    let (w, h) = (size[0] as f32, size[1] as f32);
    let margin = 12.0 * ppp;
    let span = |d: usize| (max[d] - min[d]).max(1e-6);
    let fit = ((w - 2.0 * margin) / span(0)).min((h - 2.0 * margin) / span(1)).max(1e-6);
    let px = fit * view.zoom;
    let center = [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0, (min[2] + max[2]) / 2.0];
    // clip = turned * scale + offset; y up in both, the pan's down
    let scale = [2.0 * px / w, 2.0 * px / h, -0.5 / span(2).max(1e-3) * 0.98];
    let offset = [-center[0] * scale[0] + 2.0 * view.pan.x * ppp / w, -center[1] * scale[1] - 2.0 * view.pan.y * ppp / h, 0.5 - center[2] * scale[2]];
    let mut out = Vec::with_capacity(UNIFORMS);
    for row in m {
        for v in row.iter().chain(&[0.0]) {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    for v in scale.iter().chain(&[0.0]).chain(&offset).chain(&[0.0]) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

const SHADER: &str = r#"
struct View {
    r0: vec4<f32>,
    r1: vec4<f32>,
    r2: vec4<f32>,
    scale: vec4<f32>,
    offset: vec4<f32>,
};
@group(0) @binding(0) var<uniform> view: View;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn turned(p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dot(view.r0.xyz, p), dot(view.r1.xyz, p), dot(view.r2.xyz, p));
}

@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) c: vec4<f32>) -> Out {
    let q = turned(p) * view.scale.xyz + view.offset.xyz;
    // Lit from over the viewer's shoulder, either side of a face alike
    let light = normalize(vec3<f32>(-0.3, 0.4, 0.85));
    let lit = 0.35 + 0.65 * abs(dot(turned(n), light));
    var out: Out;
    out.pos = vec4<f32>(q, 1.0);
    out.color = vec4<f32>(c.rgb * lit, 1.0);
    return out;
}

@fragment
fn fs(in: Out) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit cube, its faces two triangles each.
    fn cube(color: Color32) -> Scene {
        let points: Vec<[f32; 3]> = (0..8).map(|i| [(i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32]).collect();
        let quads = [[0, 1, 3, 2], [4, 5, 7, 6], [0, 1, 5, 4], [2, 3, 7, 6], [0, 2, 6, 4], [1, 3, 7, 5]];
        let triangles = quads.iter().flat_map(|q: &[u32; 4]| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect();
        Scene { meshes: vec![Mesh { points, triangles, color }] }
    }

    /// Scrolling over the canvas zooms, up in, and doesn't scroll the
    /// area it's in.
    #[test]
    fn scrolling_zooms() {
        use std::cell::Cell;
        use std::rc::Rc;
        let (zoom, offset) = (Rc::new(Cell::new(1.0)), Rc::new(Cell::new(0.0)));
        let (z, o) = (zoom.clone(), offset.clone());
        let mut view = View::default();
        let mut h = egui_kittest::Harness::builder().with_size(vec2(300.0, 300.0)).build_ui(move |ui| {
            let shown = egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                canvas(ui, &mut view, "none", Scene::default, 200.0);
                ui.add_space(600.0);
            });
            z.set(view.zoom);
            o.set(shown.state.offset.y);
        });
        h.hover_at(pos2(150.0, 100.0));
        h.run();
        let wheel = |dy: f32| egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: vec2(0.0, dy), phase: egui::TouchPhase::Move, modifiers: egui::Modifiers::NONE };
        h.event(wheel(40.0));
        h.run_steps(30);
        assert!(zoom.get() > 1.1, "in: {}", zoom.get());
        assert_eq!(offset.get(), 0.0, "the area didn't scroll");
        h.event(wheel(-80.0));
        h.run_steps(30);
        assert!(zoom.get() < 0.95, "out: {}", zoom.get());
        assert_eq!(offset.get(), 0.0, "nor down");
    }

    #[test]
    fn the_turn_tips_the_top_back() {
        // Straight above: unturned
        let m = turn(&View { yaw: 0.0, pitch: 0.0, ..Default::default() });
        assert_eq!(apply(&m, [1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
        // From the front: z up becomes y up, y away becomes z away
        let m = turn(&View { yaw: 0.0, pitch: std::f32::consts::FRAC_PI_2, ..Default::default() });
        let q = apply(&m, [0.0, 0.0, 1.0]);
        assert!((q[1] - 1.0).abs() < 1e-6 && q[2].abs() < 1e-6, "{q:?}");
        let q = apply(&m, [0.0, 1.0, 0.0]);
        assert!((q[2] + 1.0).abs() < 1e-6, "{q:?}");
    }

    /// Drawn by the GPU: the cube in the middle of the canvas, the
    /// background around it, and the nearer of two cubes in front.
    #[test]
    fn draws_on_the_gpu() {
        let rs = egui_kittest::wgpu::create_render_state(egui_kittest::wgpu::default_wgpu_setup(), egui_wgpu::RendererOptions::default());
        let renderer = egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs.clone());
        let mut installed = false;
        let mut h = egui_kittest::Harness::builder().with_size(vec2(300.0, 300.0)).renderer(renderer).build_ui(move |ui| {
            if !installed {
                install(ui.ctx(), &rs);
                installed = true;
            }
            let mut view = View { yaw: 0.0, pitch: 0.0, ..Default::default() };
            let mut scene = cube(Color32::from_rgb(200, 0, 0));
            // A green cube under the red one, seen from above: hidden
            let mut under = cube(Color32::from_rgb(0, 200, 0)).meshes.remove(0);
            for p in &mut under.points {
                p[2] -= 2.0;
            }
            scene.meshes.push(under);
            canvas(ui, &mut view, "cubes", || scene, 280.0);
        });
        h.run();
        let image = h.render().expect("renders");
        let px = |x: u32, y: u32| *image.get_pixel(x, y);
        let mid = px(150, 150);
        assert!(mid[0] > 100 && mid[1] < 50, "the red cube on top in the middle: {mid:?}");
        let corner = px(10, 20);
        assert!(corner[0].abs_diff(BACKGROUND.r()) < 8 && corner[2].abs_diff(BACKGROUND.b()) < 8, "the background: {corner:?}");
    }
}
