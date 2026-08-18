//! Performance stress test with automated benchmarks matching React Flow's methodology.
//!
//! Generates a grid of nodes and edges, tracks per-operation render times,
//! and supports automated benchmarks for reproducible measurements.
//!
//! Usage:
//!   cargo run --release --example stress_test              # Default: 25x25 chain
//!   cargo run --release --example stress_test -- 20 20     # 20x20 chain (400 nodes)
//!   cargo run --release --example stress_test -- 50 50 grid  # Grid mode (2 edges per node)
//!
//! Controls:
//!   - Arrow keys/Tab: navigate between nodes
//!   - h/j/k/l: pan viewport
//!   - +/-: zoom in/out
//!   - f: fit view
//!   - c: center on selected node
//!   - i: toggle interactivity lock
//!   - Delete/Backspace: delete selected
//!   - d: toggle debug info overlay
//!   - t: run drag benchmark
//!   - s: run select benchmark
//!   - r: run remount benchmark
//!   - a: run all benchmarks
//!   - q: quit

use std::{
    env, fmt,
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{
    Background, BackgroundVariant, Controls, Edge, EventResponse, Flow, FlowEvent, Node, StepEdge,
    TextContent,
};
use rataflow_examples::{ExampleMeta, render_shell};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};

fn meta() -> ExampleMeta<'static> {
    ExampleMeta {
        title: "Stress Test",
        description: None,
        keys: vec![],
    }
}

// ---------------------------------------------------------------------------
// Per-operation stats (interactive mode)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum Operation {
    #[default]
    Idle,
    Pan,
    Zoom,
    Drag,
    Select,
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Idle => write!(f, "Idle"),
            Operation::Pan => write!(f, "Pan"),
            Operation::Zoom => write!(f, "Zoom"),
            Operation::Drag => write!(f, "Drag"),
            Operation::Select => write!(f, "Select"),
        }
    }
}

#[derive(Default)]
struct OperationStats {
    render_times: Vec<Duration>,
    event_times: Vec<Duration>,
}

impl OperationStats {
    fn record(&mut self, render_time: Duration, event_time: Duration) {
        self.render_times.push(render_time);
        self.event_times.push(event_time);
    }

    fn count(&self) -> usize {
        self.render_times.len()
    }

    fn avg_render_ms(&self) -> f64 {
        if self.render_times.is_empty() {
            return 0.0;
        }
        let total: Duration = self.render_times.iter().sum();
        total.as_secs_f64() * 1000.0 / self.render_times.len() as f64
    }

    fn min_render_ms(&self) -> f64 {
        self.render_times
            .iter()
            .min()
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }

    fn max_render_ms(&self) -> f64 {
        self.render_times
            .iter()
            .max()
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }

    fn avg_event_ms(&self) -> f64 {
        if self.event_times.is_empty() {
            return 0.0;
        }
        let total: Duration = self.event_times.iter().sum();
        total.as_secs_f64() * 1000.0 / self.event_times.len() as f64
    }

    fn render_fps(&self) -> f64 {
        let avg = self.avg_render_ms();
        if avg > 0.0 { 1000.0 / avg } else { 0.0 }
    }
}

struct StateSnapshot {
    viewport_x: f64,
    viewport_y: f64,
    zoom: f64,
    is_dragging: bool,
    selection: Option<String>,
}

impl StateSnapshot {
    fn capture<N: rataflow::NodeContent, E: rataflow::EdgeContent>(flow: &Flow<N, E>) -> Self {
        Self {
            viewport_x: flow.viewport.x,
            viewport_y: flow.viewport.y,
            zoom: flow.viewport.zoom,
            is_dragging: flow.is_dragging(),
            selection: flow
                .selected_nodes()
                .next()
                .map(|n| format!("n:{}", n.id))
                .or_else(|| flow.selected_edges().next().map(|e| format!("e:{}", e.id))),
        }
    }

    fn detect_operation(&self, after: &StateSnapshot) -> Operation {
        if after.is_dragging {
            return Operation::Drag;
        }
        if (after.zoom - self.zoom).abs() > 0.001 {
            return Operation::Zoom;
        }
        if (after.viewport_x - self.viewport_x).abs() > 0.1
            || (after.viewport_y - self.viewport_y).abs() > 0.1
        {
            return Operation::Pan;
        }
        if after.selection != self.selection {
            return Operation::Select;
        }
        Operation::Idle
    }
}

struct FrameStats {
    idle_stats: OperationStats,
    pan_stats: OperationStats,
    zoom_stats: OperationStats,
    drag_stats: OperationStats,
    select_stats: OperationStats,

    recent_render_times: Vec<Duration>,
    max_recent: usize,

    last_render_ms: f64,
    last_event_ms: f64,
    last_operation: Operation,
}

impl FrameStats {
    fn new(max_recent: usize) -> Self {
        Self {
            idle_stats: OperationStats::default(),
            pan_stats: OperationStats::default(),
            zoom_stats: OperationStats::default(),
            drag_stats: OperationStats::default(),
            select_stats: OperationStats::default(),
            recent_render_times: Vec::with_capacity(max_recent),
            max_recent,
            last_render_ms: 0.0,
            last_event_ms: 0.0,
            last_operation: Operation::Idle,
        }
    }

    fn record(&mut self, render_time: Duration, event_time: Duration, operation: Operation) {
        self.last_render_ms = render_time.as_secs_f64() * 1000.0;
        self.last_event_ms = event_time.as_secs_f64() * 1000.0;
        self.last_operation = operation;

        if self.recent_render_times.len() >= self.max_recent {
            self.recent_render_times.remove(0);
        }
        self.recent_render_times.push(render_time);

        let stats = match operation {
            Operation::Idle => &mut self.idle_stats,
            Operation::Pan => &mut self.pan_stats,
            Operation::Zoom => &mut self.zoom_stats,
            Operation::Drag => &mut self.drag_stats,
            Operation::Select => &mut self.select_stats,
        };
        stats.record(render_time, event_time);
    }

    fn current_fps(&self) -> f64 {
        if self.recent_render_times.is_empty() {
            return 0.0;
        }
        let total: Duration = self.recent_render_times.iter().sum();
        let avg = total.as_secs_f64() / self.recent_render_times.len() as f64;
        if avg > 0.0 { 1.0 / avg } else { 0.0 }
    }

    fn stats_for(&self, op: Operation) -> &OperationStats {
        match op {
            Operation::Idle => &self.idle_stats,
            Operation::Pan => &self.pan_stats,
            Operation::Zoom => &self.zoom_stats,
            Operation::Drag => &self.drag_stats,
            Operation::Select => &self.select_stats,
        }
    }

    fn total_frames(&self) -> usize {
        self.idle_stats.count()
            + self.pan_stats.count()
            + self.zoom_stats.count()
            + self.drag_stats.count()
            + self.select_stats.count()
    }

    fn print_report(&self) {
        eprintln!("\nPer-Operation Statistics (render time only, excludes poll wait):");
        eprintln!(
            "  {:8} {:>8} {:>12} {:>12} {:>12} {:>10} {:>10}",
            "Op", "Frames", "Render avg", "Render min", "Render max", "Event avg", "FPS"
        );
        eprintln!("  {:-<82}", "");

        for op in [
            Operation::Idle,
            Operation::Pan,
            Operation::Zoom,
            Operation::Drag,
            Operation::Select,
        ] {
            let stats = self.stats_for(op);
            if stats.count() > 0 {
                eprintln!(
                    "  {:8} {:>8} {:>10.2}ms {:>10.2}ms {:>10.2}ms {:>8.2}ms {:>10.1}",
                    op.to_string(),
                    stats.count(),
                    stats.avg_render_ms(),
                    stats.min_render_ms(),
                    stats.max_render_ms(),
                    stats.avg_event_ms(),
                    stats.render_fps()
                );
            }
        }
        eprintln!("  {:-<82}", "");
        eprintln!("  Total frames: {}", self.total_frames());
    }
}

// ---------------------------------------------------------------------------
// Automated benchmarks
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum BenchmarkKind {
    DragInViewport,
    SelectNode,
    Remount,
}

impl BenchmarkKind {
    fn label(self) -> &'static str {
        match self {
            BenchmarkKind::DragInViewport => "dragInViewport",
            BenchmarkKind::SelectNode => "selectNode",
            BenchmarkKind::Remount => "remount",
        }
    }

    fn total_frames(self) -> u32 {
        match self {
            // 1 mousedown + 20 mousemoves + 1 mouseup + 1 trailing record
            BenchmarkKind::DragInViewport => 23,
            // 1 select + 1 trailing record
            BenchmarkKind::SelectNode => 2,
            // 1 remount + 1 trailing record
            BenchmarkKind::Remount => 2,
        }
    }
}

struct BenchmarkFrame {
    render_ms: f64,
    stage: &'static str,
}

struct Benchmark {
    kind: BenchmarkKind,
    frame: u32,
    stage: &'static str,
    frames: Vec<BenchmarkFrame>,
}

impl Benchmark {
    fn new(kind: BenchmarkKind) -> Self {
        Self {
            kind,
            frame: 0,
            stage: "setup",
            frames: Vec::new(),
        }
    }

    /// Execute one frame's mutation. Called before render.
    fn step(&mut self, flow: &mut Flow<TextContent, StepEdge>, cols: usize, rows: usize) {
        let total = self.kind.total_frames();
        if self.frame >= total {
            return;
        }

        match self.kind {
            BenchmarkKind::DragInViewport => match self.frame {
                0 => {
                    self.stage = "mousedown";
                    flow.select_node("n18_0");
                }
                1..=20 => {
                    self.stage = "mousemove";
                    flow.move_node("n18_0", (-5.0, 0.0));
                }
                21 => {
                    self.stage = "mouseup";
                }
                _ => {}
            },
            BenchmarkKind::SelectNode => {
                if self.frame == 0 {
                    self.stage = "select";
                    let node_id = format!("n{}_{}", cols / 2, rows / 2);
                    flow.select_node(&node_id);
                }
            }
            BenchmarkKind::Remount => {
                if self.frame == 0 {
                    self.stage = "remount";
                    let nodes = generate_chain_nodes(cols, rows);
                    let edges = generate_chain_edges(cols, rows);
                    *flow = Flow::with_graph(nodes, edges).expect("valid graph");
                    flow.min_zoom = 0.1;
                    flow.request_fit_view();
                }
            }
        }
    }

    /// Record the render time for the current frame. Called after render.
    fn record_render(&mut self, render_time: Duration) {
        self.frames.push(BenchmarkFrame {
            render_ms: render_time.as_secs_f64() * 1000.0,
            stage: self.stage,
        });
        self.frame += 1;
    }

    fn is_complete(&self) -> bool {
        self.frame >= self.kind.total_frames()
    }

    fn log_results(&self) {
        // Group by stage (preserving order)
        let mut stage_order: Vec<&str> = Vec::new();
        let mut stage_times: Vec<Vec<f64>> = Vec::new();

        for frame in &self.frames {
            if let Some(idx) = stage_order.iter().position(|&s| s == frame.stage) {
                stage_times[idx].push(frame.render_ms);
            } else {
                stage_order.push(frame.stage);
                stage_times.push(vec![frame.render_ms]);
            }
        }

        eprintln!("\n--- {} ---", self.kind.label());
        for (stage, times) in stage_order.iter().zip(stage_times.iter()) {
            let avg = times.iter().sum::<f64>() / times.len() as f64;
            let min = times.iter().cloned().reduce(f64::min).unwrap_or(0.0);
            let max = times.iter().cloned().reduce(f64::max).unwrap_or(0.0);
            eprintln!(
                "  {}: {} frames, avg {:.2}ms, min {:.2}ms, max {:.2}ms",
                stage,
                times.len(),
                avg,
                min,
                max
            );
        }

        // Also log in JSON format (matches WASM output for comparison)
        let mut parts = Vec::new();
        for (stage, times) in stage_order.iter().zip(stage_times.iter()) {
            let values: Vec<String> = times.iter().map(|t| format!("{:.1}", t)).collect();
            parts.push(format!("\"{}\":[{}]", stage, values.join(",")));
        }
        eprintln!("  JSON: {{{}}}", parts.join(","));
    }
}

/// Runs multiple benchmarks in sequence with idle frames between them.
struct BenchmarkSuite {
    queue: Vec<BenchmarkKind>,
    current: Option<Benchmark>,
    cooldown: u32,
    cooldown_remaining: u32,
}

impl BenchmarkSuite {
    fn new(benchmarks: Vec<BenchmarkKind>) -> Self {
        Self {
            queue: benchmarks,
            current: None,
            cooldown: 30,
            cooldown_remaining: 0,
        }
    }

    fn is_active(&self) -> bool {
        self.current.is_some() || !self.queue.is_empty() || self.cooldown_remaining > 0
    }

    fn current_label(&self) -> &str {
        if self.cooldown_remaining > 0 {
            return "cooldown";
        }
        self.current
            .as_ref()
            .map(|b| b.kind.label())
            .unwrap_or("idle")
    }

    /// Run the mutation step. Called before render.
    fn step(&mut self, flow: &mut Flow<TextContent, StepEdge>, cols: usize, rows: usize) {
        if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
            return;
        }

        if self.current.is_none() {
            if let Some(kind) = self.queue.first().copied() {
                self.queue.remove(0);
                eprintln!("--- Starting benchmark: {} ---", kind.label());
                self.current = Some(Benchmark::new(kind));
            } else {
                return;
            }
        }

        if let Some(bench) = &mut self.current {
            bench.step(flow, cols, rows);
        }
    }

    /// Record render time. Called after render. Returns true when suite is done.
    fn record_render(&mut self, render_time: Duration) -> bool {
        if self.cooldown_remaining > 0 || self.current.is_none() {
            return !self.is_active();
        }

        if let Some(bench) = &mut self.current {
            bench.record_render(render_time);
            if bench.is_complete() {
                bench.log_results();
                self.current = None;
                self.cooldown_remaining = self.cooldown;
            }
        }

        !self.is_active()
    }
}

// ---------------------------------------------------------------------------
// Graph generation
// ---------------------------------------------------------------------------

fn generate_chain_nodes(cols: usize, rows: usize) -> Vec<Node<TextContent>> {
    let node_width = 8.0;
    let node_height = 3.0;
    let spacing_x = 12.0;
    let spacing_y = 5.0;

    let mut nodes = Vec::with_capacity(cols * rows);

    for row in 0..rows {
        for col in 0..cols {
            let id = format!("n{}_{}", col, row);
            let x = col as f64 * spacing_x + 2.0;
            let y = row as f64 * spacing_y + 2.0;
            let label = format!("{},{}", col, row);
            let node = Node::new(
                &id,
                (x, y),
                (node_width, node_height),
                TextContent::from(label.as_str()),
            );
            nodes.push(node);
        }
    }

    nodes
}

fn generate_chain_edges(cols: usize, rows: usize) -> Vec<Edge<StepEdge>> {
    let mut edges = Vec::new();
    let mut prev_id: Option<String> = None;

    for row in 0..rows {
        for col in 0..cols {
            let current_id = format!("n{}_{}", col, row);
            if let Some(prev) = &prev_id {
                let edge_id = format!("e_{}_{}", prev, current_id);
                edges.push(Edge::new(&edge_id, prev, &current_id));
            }
            prev_id = Some(current_id);
        }
    }

    edges
}

fn generate_grid_edges(cols: usize, rows: usize) -> Vec<Edge<StepEdge>> {
    let mut edges = Vec::new();

    for row in 0..rows {
        for col in 0..cols {
            let source_id = format!("n{}_{}", col, row);

            if col + 1 < cols {
                let target_id = format!("n{}_{}", col + 1, row);
                let edge_id = format!("e_{}_{}_h", col, row);
                edges.push(Edge::new(&edge_id, &source_id, &target_id));
            }

            if row + 1 < rows {
                let target_id = format!("n{}_{}", col, row + 1);
                let edge_id = format!("e_{}_{}_v", col, row);
                edges.push(Edge::new(&edge_id, &source_id, &target_id));
            }
        }
    }

    edges
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> rataflow_examples::Result<()> {
    let args: Vec<String> = env::args().collect();

    let bench_mode = args.iter().any(|a| a == "--bench");
    let positional: Vec<&str> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .map(|s| s.as_str())
        .collect();

    let cols: usize = positional
        .first()
        .and_then(|s| s.parse().ok())
        .unwrap_or(25);
    let rows: usize = positional.get(1).and_then(|s| s.parse().ok()).unwrap_or(25);
    let grid_mode = positional.get(2).map(|&s| s == "grid").unwrap_or(false);

    let node_count = cols * rows;
    let edge_count: usize = if grid_mode {
        (cols - 1) * rows + cols * (rows - 1)
    } else {
        node_count.saturating_sub(1)
    };

    let mode_name = if grid_mode { "grid" } else { "chain" };
    eprintln!(
        "Generating {}x{} {}: {} nodes, {} edges",
        cols, rows, mode_name, node_count, edge_count
    );

    let nodes = generate_chain_nodes(cols, rows);
    let edges = if grid_mode {
        generate_grid_edges(cols, rows)
    } else {
        generate_chain_edges(cols, rows)
    };

    if bench_mode {
        run_headless(nodes, edges, cols, rows)
    } else {
        run_interactive(nodes, edges, cols, rows, node_count, edge_count)
    }
}

/// Headless benchmark: no TUI, renders to an off-screen buffer, prints results, exits.
fn run_headless(
    nodes: Vec<Node<TextContent>>,
    edges: Vec<Edge<StepEdge>>,
    cols: usize,
    rows: usize,
) -> rataflow_examples::Result<()> {
    use ratatui::backend::TestBackend;

    let mut flow = Flow::with_graph(nodes, edges)?;
    flow.min_zoom = 0.1;
    flow.select_node("n0_0");

    // Use a fixed terminal size for reproducible results
    let backend = TestBackend::new(200, 60);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Warm up: render a few frames so fit_view resolves
    flow.request_fit_view();
    for _ in 0..3 {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(
                Background::new(&flow)
                    .variant(BackgroundVariant::Dots)
                    .gap(10, 5),
                area,
            );
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);
        })?;
    }

    eprintln!("=== Running all benchmarks ===");
    let mut suite = BenchmarkSuite::new(vec![
        BenchmarkKind::DragInViewport,
        BenchmarkKind::SelectNode,
        BenchmarkKind::Remount,
    ]);

    loop {
        suite.step(&mut flow, cols, rows);

        let render_start = Instant::now();
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(
                Background::new(&flow)
                    .variant(BackgroundVariant::Dots)
                    .gap(10, 5),
                area,
            );
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);
        })?;
        let render_time = render_start.elapsed();

        if suite.record_render(render_time) {
            eprintln!("=== All benchmarks complete ===");
            break;
        }
    }

    Ok(())
}

/// Interactive TUI mode with manual and automated benchmarks.
fn run_interactive(
    nodes: Vec<Node<TextContent>>,
    edges: Vec<Edge<StepEdge>>,
    cols: usize,
    rows: usize,
    node_count: usize,
    edge_count: usize,
) -> rataflow_examples::Result<()> {
    let mut flow = Flow::with_graph(nodes, edges)?;
    flow.min_zoom = 0.1;
    flow.select_node("n0_0");

    let mut frame_stats = FrameStats::new(60);
    let mut show_debug = true;
    let start_time = Instant::now();
    let mut suite: Option<BenchmarkSuite> = None;

    let tick_rate = Duration::from_millis(16);
    let mut last_tick = Instant::now();

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;

    flow.request_fit_view();

    loop {
        let benchmark_active = suite.as_ref().is_some_and(|s| s.is_active());
        let state_before = StateSnapshot::capture(&flow);

        // Event handling (skip during benchmarks)
        let event_process_start = Instant::now();
        let mut should_quit = false;
        let mut events_processed: u32 = 0;

        let timeout = if benchmark_active {
            Duration::ZERO
        } else {
            tick_rate.saturating_sub(last_tick.elapsed())
        };

        if event::poll(timeout)? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key) if !benchmark_active => {
                        events_processed += 1;
                        match key.code {
                            KeyCode::Char('q') => should_quit = true,
                            KeyCode::Char('d') => show_debug = !show_debug,
                            KeyCode::Char('t') => {
                                suite =
                                    Some(BenchmarkSuite::new(vec![BenchmarkKind::DragInViewport]));
                            }
                            KeyCode::Char('s') => {
                                suite = Some(BenchmarkSuite::new(vec![BenchmarkKind::SelectNode]));
                            }
                            KeyCode::Char('r') => {
                                suite = Some(BenchmarkSuite::new(vec![BenchmarkKind::Remount]));
                            }
                            KeyCode::Char('a') => {
                                eprintln!("=== Running all benchmarks ===");
                                suite = Some(BenchmarkSuite::new(vec![
                                    BenchmarkKind::DragInViewport,
                                    BenchmarkKind::SelectNode,
                                    BenchmarkKind::Remount,
                                ]));
                            }
                            _ => {
                                let response = flow.handle_controls_key_event(key);
                                if matches!(response, EventResponse::NotHandled) {
                                    flow.handle_key_event(key);
                                }
                            }
                        }
                    }
                    CrosstermEvent::Mouse(mouse) if !benchmark_active => {
                        events_processed += 1;
                        for event in flow.handle_mouse_event(mouse).into_events() {
                            if let FlowEvent::ConnectionCompleted(conn) = event {
                                flow.add_edge_from_connection(conn, StepEdge::default());
                            }
                        }
                    }
                    _ => {}
                }

                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        let event_time = event_process_start.elapsed();

        if should_quit {
            break;
        }

        // Benchmark mutation step (before render)
        if let Some(s) = &mut suite {
            s.step(&mut flow, cols, rows);
        }

        // Render
        let render_start = Instant::now();
        terminal.draw(|frame| {
            let area = render_shell(frame, frame.area(), &meta());

            frame.render_widget(
                Background::new(&flow)
                    .variant(BackgroundVariant::Dots)
                    .gap(10, 5),
                area,
            );
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);

            // Stats overlay
            let selected_info = flow
                .selected_nodes()
                .next()
                .map(|n| n.id.clone())
                .unwrap_or_else(|| "None".to_string());

            let bench_label = suite
                .as_ref()
                .filter(|s| s.is_active())
                .map(|s| s.current_label())
                .unwrap_or("idle");

            let stats_text = if show_debug {
                let elapsed = start_time.elapsed().as_secs();
                let op_summary: Vec<String> = [
                    Operation::Idle,
                    Operation::Pan,
                    Operation::Zoom,
                    Operation::Drag,
                    Operation::Select,
                ]
                .iter()
                .filter_map(|&op| {
                    let stats = frame_stats.stats_for(op);
                    if stats.count() > 0 {
                        Some(format!("{}:{:.1}ms", op, stats.avg_render_ms()))
                    } else {
                        None
                    }
                })
                .collect();

                format!(
                    " {} nodes, {} edges | FPS: {:.1} | Render: {:.2}ms | Event: {:.2}ms | {} | {}s | {} | {}",
                    node_count, edge_count,
                    frame_stats.current_fps(),
                    frame_stats.last_render_ms,
                    frame_stats.last_event_ms,
                    bench_label,
                    elapsed,
                    selected_info,
                    op_summary.join(" | "),
                )
            } else {
                format!(
                    " {} nodes, {} edges | FPS: {:.1} | Render: {:.2}ms | {}",
                    node_count, edge_count,
                    frame_stats.current_fps(),
                    frame_stats.last_render_ms,
                    bench_label,
                )
            };

            let stats_style = if frame_stats.current_fps() < 30.0 {
                Style::default().fg(Color::Indexed(167)).add_modifier(Modifier::BOLD)
            } else if frame_stats.current_fps() < 50.0 {
                Style::default().fg(Color::Indexed(179))
            } else {
                Style::default().fg(Color::Indexed(71))
            };

            let stats_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            };
            frame.render_widget(Paragraph::new(stats_text).style(stats_style), stats_area);
        })?;
        let render_time = render_start.elapsed();

        // Record benchmark render time (after render)
        if let Some(s) = &mut suite
            && s.record_render(render_time)
        {
            eprintln!("=== All benchmarks complete ===");
            suite = None;
        }

        // Record interactive stats
        let state_after = StateSnapshot::capture(&flow);
        let operation = if events_processed > 0 {
            state_before.detect_operation(&state_after)
        } else {
            Operation::Idle
        };
        frame_stats.record(render_time, event_time, operation);

        if last_tick.elapsed() >= tick_rate {
            flow.tick_auto_pan(last_tick.elapsed());
            last_tick = Instant::now();
        }
    }

    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();

    eprintln!("\nFinal Statistics:");
    eprintln!("  Total nodes: {}", node_count);
    eprintln!("  Total edges: {}", edge_count);

    frame_stats.print_report();

    Ok(())
}
