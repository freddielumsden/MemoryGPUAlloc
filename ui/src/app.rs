//! Presentation layer: draws the workspace and turns clicks and drag-and-drop
//! into `Command`s, which are applied to the `Workspace` after the frame.

use std::collections::HashMap;

use egui::{Align, Id, Layout, RichText, ScrollArea, Ui, vec2};

use gpu_memory_alloc::allocator::Allocator;
use gpu_memory_alloc::gpu::GPU;

use crate::model::{
    AllocatorNode, AllocatorState, GpuLocation, NodeId, OpResult, PoolLocation, PoolNode, RefLocation, Workspace,
};
use crate::theme;
use crate::widgets::{self, Tile, badge, card, drop_zone, inset, section_header, wrap_grid};

/// Anything that can be picked up with the mouse.
#[derive(Clone, Copy, Debug)]
pub enum Drag {
    Ref(RefLocation),
    Gpu(GpuLocation),
    Pool(NodeId),
    /// One ref from an online allocator's internal pool.
    PoolToken(NodeId),
}

/// Anywhere something can be dropped.
#[derive(Clone, Copy, Debug)]
enum Target {
    Tray,
    Pool(NodeId),
    PoolsArea,
    Rack,
    AllocGpus(NodeId),
    AllocPoolSlot(NodeId),
    GpuCell(NodeId, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Kind {
    MemRef,
    MemPool,
    Gpu,
    Allocator,
}

impl Kind {
    const ALL: [Kind; 4] = [Kind::MemRef, Kind::MemPool, Kind::Gpu, Kind::Allocator];

    fn name(self) -> &'static str {
        match self {
            Kind::MemRef => "MemRef",
            Kind::MemPool => "MemPool",
            Kind::Gpu => "GPU",
            Kind::Allocator => "Allocator",
        }
    }

    fn path(self) -> &'static str {
        match self {
            Kind::MemRef => "memory::MemRef",
            Kind::MemPool => "memory::MemPool",
            Kind::Gpu => "gpu::GPU",
            Kind::Allocator => "allocator::Allocator",
        }
    }

    fn color(self) -> egui::Color32 {
        match self {
            Kind::MemRef => theme::MEMORY,
            Kind::MemPool => theme::POOL,
            Kind::Gpu => theme::GPU,
            Kind::Allocator => theme::ACCENT,
        }
    }
}

enum Command {
    Drop(Drag, Target),
    Add(Kind, usize),
    Remove(Kind, usize),
    Initialise(NodeId),
    Allocate(NodeId, usize, usize),
    AllocateEvenly(NodeId, usize),
    Deallocate(NodeId, usize),
    Run(NodeId),
}

struct AllocInputs {
    size: String,
    gpu: usize,
}

impl Default for AllocInputs {
    fn default() -> Self {
        AllocInputs { size: "1".into(), gpu: 0 }
    }
}

/// Per-frame UI state that is not part of the backend model.
#[derive(Default)]
struct UiState {
    amounts: HashMap<Kind, String>,
    alloc_inputs: HashMap<NodeId, AllocInputs>,
    commands: Vec<Command>,
}

struct LogLine {
    ok: bool,
    text: String,
}

pub struct App {
    ws: Workspace,
    state: UiState,
    log: Vec<LogLine>,
}

const TILE: egui::Vec2 = egui::vec2(58.0, 40.0);
const SMALL_TILE: egui::Vec2 = egui::vec2(34.0, 26.0);
const POOL_CARD_WIDTH: f32 = 236.0;
const GPU_CELL_WIDTH: f32 = 150.0;
const MIN_GPU_SLOT_WIDTH: f32 = 300.0;
/// Rendering thousands of identical tiles helps nobody; the rest are summarised.
const MAX_TILES: usize = 160;

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut app = App { ws: Workspace::default(), state: UiState::default(), log: Vec::new() };
        app.push_log(true, "Workspace ready: 18 memory refs, 1 pool, 8 GPUs, 1 allocator.".into());
        app
    }

    fn push_log(&mut self, ok: bool, text: String) {
        self.log.push(LogLine { ok, text });
        if self.log.len() > 500 {
            self.log.drain(..100);
        }
    }

    fn apply_commands(&mut self) {
        for cmd in std::mem::take(&mut self.state.commands) {
            let (result, text) = self.execute(cmd);
            match result {
                Ok(()) => self.push_log(true, text),
                Err(e) => self.push_log(false, format!("{text}: {e}")),
            }
        }
    }

    fn execute(&mut self, cmd: Command) -> (OpResult, String) {
        let ws = &mut self.ws;
        match cmd {
            Command::Drop(drag, target) => (apply_drop(ws, drag, target), describe_drop(drag, target)),
            Command::Add(kind, n) => {
                match kind {
                    Kind::MemRef => ws.add_refs(n),
                    Kind::MemPool => ws.add_pools(n),
                    Kind::Gpu => ws.add_gpus(n),
                    Kind::Allocator => ws.add_allocators(n),
                }
                (Ok(()), format!("Added {n} × {}", kind.name()))
            }
            Command::Remove(kind, n) => {
                let removed = match kind {
                    Kind::MemRef => ws.remove_refs(n),
                    Kind::MemPool => ws.remove_pools(n),
                    Kind::Gpu => ws.remove_gpus(n),
                    Kind::Allocator => ws.remove_allocators(n),
                };
                let result = if removed == 0 && n > 0 { Err("nothing free to remove".into()) } else { Ok(()) };
                (result, format!("Removed {removed} × {}", kind.name()))
            }
            Command::Initialise(id) => (ws.initialise(id), format!("Allocator #{id}: Allocator::new()")),
            Command::Allocate(id, size, gpu) => {
                (ws.allocate(id, size, gpu), format!("Allocator #{id}: allocate({size}, {gpu})"))
            }
            Command::AllocateEvenly(id, size) => {
                (ws.allocate_evenly(id, size), format!("Allocator #{id}: allocate_evenly({size})"))
            }
            Command::Deallocate(id, gpu) => (ws.deallocate(id, gpu), format!("Allocator #{id}: deallocate({gpu})")),
            Command::Run(id) => (ws.run(id), format!("Allocator #{id}: run()")),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::same(12));

        egui::TopBottomPanel::top("memory_blocks").frame(panel).show(ctx, |ui| {
            header_bar(ui, &self.ws);
            ui.add_space(8.0);
            memory_tray(ui, &self.ws, &mut self.state);
        });

        egui::SidePanel::right("components")
            .frame(panel)
            .resizable(false)
            .exact_width(260.0)
            .show(ctx, |ui| components_panel(ui, &self.ws, &mut self.state));

        egui::TopBottomPanel::bottom("event_log")
            .frame(panel)
            .resizable(true)
            .default_height(120.0)
            .show(ctx, |ui| event_log(ui, &self.log));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::same(14)))
            .show(ctx, |ui| {
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    pools_section(ui, &self.ws, &mut self.state);
                    ui.add_space(14.0);
                    rack_section(ui, &self.ws, &mut self.state);
                    ui.add_space(14.0);
                    allocators_section(ui, &self.ws, &mut self.state);
                });
            });

        self.apply_commands();
    }
}

// ------------------------------------------------------------------ drag & drop rules

fn accepts(ws: &Workspace, target: Target, drag: &Drag) -> bool {
    match (*drag, target) {
        (Drag::Ref(from), Target::Tray) => from != RefLocation::Tray,
        (Drag::Ref(from), Target::Pool(id)) => from != RefLocation::Pool(id),
        (Drag::Gpu(from), Target::Rack) => from != GpuLocation::Rack,
        (Drag::Gpu(from), Target::AllocGpus(id)) => from != GpuLocation::Allocator(id),
        (Drag::Pool(p), Target::PoolsArea) => ws.pool_location(p) != Some(PoolLocation::Workspace),
        (Drag::Pool(p), Target::AllocPoolSlot(id)) => {
            ws.pool_location(p) != Some(PoolLocation::Allocator(id)) && staged_pool(ws, id).is_none()
        }
        (Drag::PoolToken(from), Target::GpuCell(id, _)) => from == id,
        _ => false,
    }
}

fn apply_drop(ws: &mut Workspace, drag: Drag, target: Target) -> OpResult {
    match (drag, target) {
        (Drag::Ref(from), Target::Tray) => ws.move_ref(from, RefLocation::Tray),
        (Drag::Ref(from), Target::Pool(id)) => ws.move_ref(from, RefLocation::Pool(id)),
        (Drag::Gpu(from), Target::Rack) => ws.move_gpu(from, GpuLocation::Rack),
        (Drag::Gpu(from), Target::AllocGpus(id)) => ws.move_gpu(from, GpuLocation::Allocator(id)),
        (Drag::Pool(p), Target::PoolsArea) => ws.move_pool(p, PoolLocation::Workspace),
        (Drag::Pool(p), Target::AllocPoolSlot(id)) => ws.move_pool(p, PoolLocation::Allocator(id)),
        (Drag::PoolToken(id), Target::GpuCell(_, gpu)) => ws.allocate(id, 1, gpu),
        _ => Err("cannot drop that here".into()),
    }
}

fn describe_drop(drag: Drag, target: Target) -> String {
    let what = match drag {
        Drag::Ref(_) => "MemRef".to_string(),
        Drag::Gpu(_) => "GPU".to_string(),
        Drag::Pool(id) => format!("MemPool #{id}"),
        Drag::PoolToken(id) => return format!("Allocator #{id}: allocate(1, {})", gpu_of(target)),
    };
    let to = match target {
        Target::Tray => "memory blocks".to_string(),
        Target::Pool(id) => format!("MemPool #{id}"),
        Target::PoolsArea => "workspace".to_string(),
        Target::Rack => "GPU rack".to_string(),
        Target::AllocGpus(id) | Target::AllocPoolSlot(id) => format!("Allocator #{id}"),
        Target::GpuCell(id, gpu) => format!("Allocator #{id} GPU {gpu}"),
    };
    format!("Moved {what} → {to}")
}

fn gpu_of(target: Target) -> usize {
    if let Target::GpuCell(_, gpu) = target { gpu } else { 0 }
}

fn staged_pool(ws: &Workspace, id: NodeId) -> Option<&PoolNode> {
    ws.allocators.iter().find(|a| a.id == id).and_then(|a| match &a.state {
        AllocatorState::Staging { pool, .. } => pool.as_ref(),
        AllocatorState::Online { .. } => None,
    })
}

/// Wraps `widgets::drop_zone` with the workspace's acceptance rules and queues
/// a `Command::Drop` when something lands.
fn target_zone<R>(
    ui: &mut Ui,
    ws: &Workspace,
    state: &mut UiState,
    frame: egui::Frame,
    target: Target,
    add_contents: impl FnOnce(&mut Ui, &mut UiState) -> R,
) -> R {
    let (inner, dropped) = drop_zone(ui, frame, |d: &Drag| accepts(ws, target, d), |ui| add_contents(ui, state));
    if let Some(drag) = dropped {
        state.commands.push(Command::Drop(drag, target));
    }
    inner
}

// ------------------------------------------------------------------ top bar

fn header_bar(ui: &mut Ui, ws: &Workspace) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("GPU MEMORY ALLOCATOR").strong().size(16.0).color(theme::TEXT));
        ui.label(RichText::new("control console").color(theme::TEXT_DIM));
        ui.add_space(12.0);
        let online = ws.allocators.iter().filter(|a| matches!(a.state, AllocatorState::Online { .. })).count();
        stat(ui, "BLOCKS FREE", ws.tray.len(), theme::MEMORY);
        stat(ui, "POOLS", ws.pools.len(), theme::POOL);
        stat(ui, "GPUS FREE", ws.rack.len(), theme::GPU);
        stat(ui, "ALLOCATORS", ws.allocators.len(), theme::ACCENT);
        stat(ui, "ONLINE", online, theme::ACCENT);
    });
}

fn stat(ui: &mut Ui, label: &str, value: usize, color: egui::Color32) {
    inset().inner_margin(egui::Margin::symmetric(8, 3)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).small().color(theme::TEXT_DIM));
            ui.label(RichText::new(value.to_string()).monospace().strong().color(color));
        });
    });
}

fn memory_tray(ui: &mut Ui, ws: &Workspace, state: &mut UiState) {
    section_header(ui, "Memory blocks · loose MemRefs", "drag a block into a pool");
    let frame = inset().inner_margin(egui::Margin::same(10));
    target_zone(ui, ws, state, frame, Target::Tray, |ui, _| {
        ui.set_width(ui.available_width());
        ScrollArea::vertical().id_salt("tray_scroll").max_height(104.0).show(ui, |ui| {
            if ws.tray.is_empty() {
                ui.label(
                    RichText::new(
                        "No loose memory refs. Add some from the Components panel, or drag one back out of a pool.",
                    )
                    .color(theme::TEXT_DIM),
                );
                return;
            }
            ui.horizontal_wrapped(|ui| {
                for (i, mem_ref) in ws.tray.iter().take(MAX_TILES).enumerate() {
                    let size = format!("{} B", mem_ref.length());
                    Tile { size: TILE, color: theme::MEMORY, title: "REF", subtitle: &size, led: None }
                        .show_draggable(ui, Id::new(("tray_ref", i)), Drag::Ref(RefLocation::Tray))
                        .on_hover_text("MemRef — drag into a MemPool");
                }
                overflow_note(ui, ws.tray.len());
            });
        });
    });
}

fn overflow_note(ui: &mut Ui, total: usize) {
    if total > MAX_TILES {
        ui.label(RichText::new(format!("+{} more", total - MAX_TILES)).color(theme::TEXT_DIM));
    }
}

// ------------------------------------------------------------------ central sections

fn pools_section(ui: &mut Ui, ws: &Workspace, state: &mut UiState) {
    section_header(ui, "Memory pools", &format!("{} in workspace", ws.pools.len()));
    let frame = egui::Frame::new().corner_radius(theme::CARD_RADIUS).inner_margin(egui::Margin::same(4));
    target_zone(ui, ws, state, frame, Target::PoolsArea, |ui, state| {
        ui.set_width(ui.available_width());
        if ws.pools.is_empty() {
            ui.label(RichText::new("No pools. Add one from the Components panel.").color(theme::TEXT_DIM));
        }
        wrap_grid(ui, ws.pools.len(), POOL_CARD_WIDTH, |ui, i| pool_card(ui, ws, state, &ws.pools[i]));
    });
}

fn pool_card(ui: &mut Ui, ws: &Workspace, state: &mut UiState, node: &PoolNode) {
    let id = node.id;
    let size = node.pool.size();
    target_zone(ui, ws, state, card(), Target::Pool(id), |ui, _| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.dnd_drag_source(Id::new(("pool_handle", id)), Drag::Pool(id), |ui| {
                badge(ui, &format!("☰ MEMPOOL #{id}"), theme::POOL)
            })
            .response
            .on_hover_text("Drag the pool into an allocator's pool slot");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{size} refs")).monospace().color(theme::TEXT));
            });
        });
        ui.add_space(4.0);
        inset().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(SMALL_TILE.y);
            if node.pool.is_empty() {
                ui.label(RichText::new("empty — drop memory blocks here").color(theme::TEXT_DIM).small());
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for i in 0..size.min(MAX_TILES) {
                    Tile { size: SMALL_TILE, color: theme::MEMORY, title: "", subtitle: "", led: None }
                        .show_draggable(ui, Id::new(("pool_ref", id, i)), Drag::Ref(RefLocation::Pool(id)))
                        .on_hover_text("MemRef — drag to another pool or back to memory blocks");
                }
                overflow_note(ui, size);
            });
        });
    });
}

fn rack_section(ui: &mut Ui, ws: &Workspace, state: &mut UiState) {
    section_header(ui, "GPU rack · unassigned", &format!("{} GPUs", ws.rack.len()));
    let frame = inset().inner_margin(egui::Margin::same(10));
    target_zone(ui, ws, state, frame, Target::Rack, |ui, _| {
        ui.set_width(ui.available_width());
        if ws.rack.is_empty() {
            ui.label(RichText::new("No free GPUs. Add some from the Components panel.").color(theme::TEXT_DIM));
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for (i, gpu) in ws.rack.iter().take(MAX_TILES).enumerate() {
                gpu_tile(ui, Id::new(("rack_gpu", i)), GpuLocation::Rack, gpu.is_available(), "");
            }
            overflow_note(ui, ws.rack.len());
        });
    });
}

fn gpu_tile(ui: &mut Ui, id: Id, location: GpuLocation, available: bool, subtitle: &str) {
    Tile { size: TILE, color: theme::GPU, title: "GPU", subtitle, led: Some(available) }
        .show_draggable(ui, id, Drag::Gpu(location))
        .on_hover_text(if available { "GPU — available" } else { "GPU — busy" });
}

fn allocators_section(ui: &mut Ui, ws: &Workspace, state: &mut UiState) {
    section_header(ui, "Allocators", &format!("{} total", ws.allocators.len()));
    if ws.allocators.is_empty() {
        ui.label(RichText::new("No allocators. Add one from the Components panel.").color(theme::TEXT_DIM));
    }
    for node in &ws.allocators {
        allocator_card(ui, ws, state, node);
        ui.add_space(10.0);
    }
}

fn allocator_card(ui: &mut Ui, ws: &Workspace, state: &mut UiState, node: &AllocatorNode) {
    let id = node.id;
    card().inner_margin(egui::Margin::same(12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("ALLOCATOR #{id}")).strong().size(14.0).color(theme::ACCENT));
            match node.state {
                AllocatorState::Staging { .. } => badge(ui, "STAGING", theme::ACCENT),
                AllocatorState::Online { .. } => badge(ui, "ONLINE", theme::GPU),
            };
        });
        ui.add_space(6.0);
        match &node.state {
            AllocatorState::Staging { gpus, pool } => staging_body(ui, ws, state, id, gpus, pool.as_ref()),
            AllocatorState::Online { allocator } => online_body(ui, state, id, allocator),
        }
    });
}

fn staging_body(ui: &mut Ui, ws: &Workspace, state: &mut UiState, id: NodeId, gpus: &[GPU], pool: Option<&PoolNode>) {
    // Flex-style: GPU and pool slots sit side by side when there is room and
    // stack when there is not.
    let pool_width = POOL_CARD_WIDTH + 24.0;
    let spacing = ui.spacing().item_spacing.x;
    if ui.available_width() >= MIN_GPU_SLOT_WIDTH + spacing + pool_width {
        ui.horizontal_top(|ui| {
            let gpu_width = ui.available_width() - pool_width - spacing;
            ui.allocate_ui_with_layout(vec2(gpu_width, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(gpu_width);
                staged_gpus_slot(ui, ws, state, id, gpus);
            });
            ui.allocate_ui_with_layout(vec2(pool_width, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(pool_width);
                staged_pool_slot(ui, ws, state, id, pool);
            });
        });
    } else {
        staged_gpus_slot(ui, ws, state, id, gpus);
        ui.add_space(6.0);
        staged_pool_slot(ui, ws, state, id, pool);
    }

    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button(RichText::new("▶ Initialise").strong()).clicked() {
            state.commands.push(Command::Initialise(id));
        }
        ui.label(
            RichText::new("Calls Allocator::new(gpus, pool). The configuration is locked afterwards.")
                .small()
                .color(theme::TEXT_DIM),
        );
    });
}

fn staged_gpus_slot(ui: &mut Ui, ws: &Workspace, state: &mut UiState, id: NodeId, gpus: &[GPU]) {
    ui.label(RichText::new(format!("GPUS · Vec<GPU> · {}", gpus.len())).small().strong().color(theme::TEXT_DIM));
    target_zone(ui, ws, state, inset(), Target::AllocGpus(id), |ui, _| {
        ui.set_width(ui.available_width());
        ui.set_min_height(TILE.y);
        if gpus.is_empty() {
            ui.label(RichText::new("drop GPUs here from the rack").color(theme::TEXT_DIM));
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for (i, gpu) in gpus.iter().take(MAX_TILES).enumerate() {
                let index = i.to_string();
                gpu_tile(ui, Id::new(("staged_gpu", id, i)), GpuLocation::Allocator(id), gpu.is_available(), &index);
            }
            overflow_note(ui, gpus.len());
        });
    });
}

fn staged_pool_slot(ui: &mut Ui, ws: &Workspace, state: &mut UiState, id: NodeId, pool: Option<&PoolNode>) {
    ui.label(RichText::new("POOL · MemPool").small().strong().color(theme::TEXT_DIM));
    target_zone(ui, ws, state, inset(), Target::AllocPoolSlot(id), |ui, state| {
        ui.set_width(ui.available_width());
        ui.set_min_height(TILE.y);
        match pool {
            Some(node) => pool_card(ui, ws, state, node),
            None => {
                ui.label(RichText::new("drop a MemPool here").color(theme::TEXT_DIM));
            }
        }
    });
}

fn online_body(ui: &mut Ui, state: &mut UiState, id: NodeId, allocator: &Allocator) {
    let pool_size = allocator.pool_size();
    // Internal pool: drag a token from it onto a GPU to allocate one ref.
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("INTERNAL POOL").small().strong().color(theme::TEXT_DIM));
        if pool_size > 0 {
            Tile { size: TILE, color: theme::POOL, title: &pool_size.to_string(), subtitle: "refs", led: None }
                .show_draggable(ui, Id::new(("pool_token", id)), Drag::PoolToken(id))
                .on_hover_text("Drag onto a GPU to allocate(1, gpu)");
            ui.label(RichText::new("drag onto a GPU to allocate one ref").small().color(theme::TEXT_DIM));
        } else {
            Tile { size: TILE, color: theme::BORDER, title: "0", subtitle: "refs", led: None }.show(ui);
            ui.label(RichText::new("pool exhausted").small().color(theme::TEXT_DIM));
        }
    });
    ui.add_space(6.0);

    let gpu_count = allocator.no_gpus();
    if gpu_count == 0 {
        ui.label(RichText::new("This allocator was initialised without GPUs.").color(theme::TEXT_DIM));
    }
    wrap_grid(ui, gpu_count, GPU_CELL_WIDTH, |ui, gpu| {
        let refs = allocator.gpu_ref_count(gpu).unwrap_or(0);
        let (_, dropped) = drop_zone(
            ui,
            inset(),
            |d: &Drag| matches!(d, Drag::PoolToken(from) if *from == id),
            |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("GPU {gpu}")).monospace().strong().color(theme::GPU));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Free").on_hover_text(format!("deallocate({gpu})")).clicked() {
                            state.commands.push(Command::Deallocate(id, gpu));
                        }
                    });
                });
                ui.label(RichText::new(format!("{refs} refs")).small().color(theme::TEXT_DIM));
                ref_meter(ui, refs);
            },
        );
        if let Some(drag) = dropped {
            state.commands.push(Command::Drop(drag, Target::GpuCell(id, gpu)));
        }
    });

    ui.add_space(8.0);
    let inputs = state.alloc_inputs.entry(id).or_default();
    let mut queued = Vec::new();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("SIZE").small().strong().color(theme::TEXT_DIM));
        let size = widgets::count_input(ui, &mut inputs.size);
        ui.label(RichText::new("GPU").small().strong().color(theme::TEXT_DIM));
        inputs.gpu = inputs.gpu.min(gpu_count.saturating_sub(1));
        egui::ComboBox::from_id_salt(("gpu_pick", id)).width(56.0).selected_text(inputs.gpu.to_string()).show_ui(
            ui,
            |ui| {
                for g in 0..gpu_count {
                    ui.selectable_value(&mut inputs.gpu, g, g.to_string());
                }
            },
        );
        ui.separator();
        let has_size = size.is_some();
        if ui.add_enabled(has_size && gpu_count > 0, egui::Button::new("Allocate")).clicked() {
            queued.push(Command::Allocate(id, size.unwrap_or(0), inputs.gpu));
        }
        if ui.add_enabled(has_size, egui::Button::new("Allocate evenly")).clicked() {
            queued.push(Command::AllocateEvenly(id, size.unwrap_or(0)));
        }
        if ui.add_enabled(gpu_count > 0, egui::Button::new("Deallocate")).clicked() {
            queued.push(Command::Deallocate(id, inputs.gpu));
        }
        if ui.button("▶ Run").on_hover_text("Allocator::run()").clicked() {
            queued.push(Command::Run(id));
        }
    });
    state.commands.extend(queued);
}

/// Row of small squares showing how many refs a GPU holds.
fn ref_meter(ui: &mut Ui, refs: usize) {
    const SHOWN: usize = 12;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 10.0), egui::Sense::hover());
    let cell = (rect.width() / SHOWN as f32).min(11.0);
    for i in 0..SHOWN {
        let r = egui::Rect::from_min_size(rect.min + vec2(i as f32 * cell, 0.0), vec2(cell - 2.0, 8.0));
        let filled = i < refs.min(SHOWN);
        let color = if filled { theme::MEMORY } else { theme::BORDER };
        ui.painter().rect_filled(r, 2.0, color);
    }
}

// ------------------------------------------------------------------ side panel

fn components_panel(ui: &mut Ui, ws: &Workspace, state: &mut UiState) {
    section_header(ui, "Components", "");
    ui.add_space(4.0);
    for kind in Kind::ALL {
        let (count_label, count) = match kind {
            Kind::MemRef => ("loose", ws.tray.len()),
            Kind::MemPool => ("in workspace", ws.pools.len()),
            Kind::Gpu => ("in rack", ws.rack.len()),
            Kind::Allocator => ("total", ws.allocators.len()),
        };
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(kind.name()).strong().color(kind.color()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(count.to_string()).monospace().strong());
                    ui.label(RichText::new(count_label).small().color(theme::TEXT_DIM));
                });
            });
            ui.label(RichText::new(kind.path()).monospace().small().color(theme::TEXT_DIM));
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let text = state.amounts.entry(kind).or_insert_with(|| "1".into());
                let amount = widgets::count_input(ui, text);
                if ui.add_enabled(amount.is_some(), egui::Button::new("+ Add")).clicked() {
                    state.commands.push(Command::Add(kind, amount.unwrap_or(0)));
                }
                if ui.add_enabled(amount.is_some(), egui::Button::new("− Remove")).clicked() {
                    state.commands.push(Command::Remove(kind, amount.unwrap_or(0)));
                }
            });
        });
        ui.add_space(6.0);
    }

    ui.add_space(10.0);
    ui.label(
        RichText::new(
            "Remove takes from free items only: loose refs, workspace pools (refs return to the tray), \
             rack GPUs, and the newest allocators (staged parts are returned; online ones are dropped).",
        )
        .small()
        .color(theme::TEXT_DIM),
    );
}

// ------------------------------------------------------------------ bottom panel

fn event_log(ui: &mut Ui, log: &[LogLine]) {
    section_header(ui, "Event log", &format!("{} entries", log.len()));
    inset().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
            for line in log {
                ui.horizontal(|ui| {
                    let (tag, color) = if line.ok { ("OK ", theme::GPU) } else { ("ERR", theme::DANGER) };
                    ui.label(RichText::new(tag).monospace().strong().color(color));
                    ui.label(RichText::new(&line.text).monospace());
                });
            }
        });
    });
}
