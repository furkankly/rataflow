//! Stress test with automated benchmarks matching React Flow's methodology.
//!
//! Default: 25x25 chain (625 nodes, 624 edges) — same as React Flow's stress test.
//! Configurable via URL params: `?cols=25&rows=25` or `?size=25`.
//!
//! Benchmarks are rAF-aligned: one operation per animation frame, measuring
//! frame-to-frame duration via `performance.now()`. Results logged to console
//! in the same format as React Flow's FrameRecorder.

use crate::demo::Demo;
use crate::DemoEntry;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::time::Duration;

use rataflow::{
    Background, BackgroundVariant, Controls, Edge, EventResponse, Flow, Node, StepEdge, TextContent,
};

use rataflow_examples::ExampleMeta;

pub fn entry_stress_test() -> DemoEntry {
    DemoEntry {
        demo: Box::new(StressTestDemo::new()),
        meta: ExampleMeta {
            title: "Stress Test",
            description: Some(
                "Default: 25x25 grid (625 nodes, 624 edges).\nResize via URL, e.g. ?size=50#stress-test or ?cols=30&rows=20#stress-test",
            ),
            keys: vec![
                ("t", "drag test"),
                ("s", "select test"),
                ("r", "remount test"),
                ("a", "run all"),
                ("l", "log frames"),
                ("↑↓", "select next/prev"),
                ("hjkl", "pan"),
                ("+/-", "zoom"),
                ("f", "fit view"),
                ("c", "center"),
                ("i", "lock"),
                ("Del", "delete"),
            ],
        },
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
            let x = col as f64 * spacing_x;
            let y = row as f64 * spacing_y;
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

fn get_grid_size_from_url() -> (usize, usize) {
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            let mut cols = 25;
            let mut rows = 25;

            for param in search.trim_start_matches('?').split('&') {
                let parts: Vec<&str> = param.split('=').collect();
                if parts.len() == 2 {
                    match parts[0] {
                        "cols" => cols = parts[1].parse().unwrap_or(25),
                        "rows" => rows = parts[1].parse().unwrap_or(25),
                        "size" => {
                            let size = parts[1].parse().unwrap_or(25);
                            cols = size;
                            rows = size;
                        }
                        _ => {}
                    }
                }
            }

            return (cols, rows);
        }
    }
    (25, 25)
}

// ---------------------------------------------------------------------------
// Performance measurement
// ---------------------------------------------------------------------------

fn perf_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

fn log(msg: &str) {
    web_sys::console::log_1(&msg.into());
}

#[derive(Clone)]
struct FrameData {
    duration: f64,
    stage: String,
}

/// Measures frame-to-frame duration, called once per rAF-driven render.
///
/// Matches React Flow's FrameRecorder: records the time between consecutive
/// animation frames, with stage annotations for grouping results.
struct FrameRecorder {
    frames: Vec<FrameData>,
    stage: String,
    last_frame_time: f64,
    max_frames: usize,
}

impl FrameRecorder {
    fn new(max_frames: usize) -> Self {
        Self {
            frames: Vec::new(),
            stage: "idle".to_string(),
            last_frame_time: perf_now(),
            max_frames,
        }
    }

    fn set_stage(&mut self, stage: &str) {
        self.stage = stage.to_string();
    }

    fn record_frame(&mut self) {
        let now = perf_now();
        let duration = now - self.last_frame_time;
        self.last_frame_time = now;

        // Skip frames with >1s gap (tab was backgrounded)
        if duration > 1000.0 {
            return;
        }

        if self.frames.len() >= self.max_frames {
            self.frames.remove(0);
        }

        self.frames.push(FrameData {
            duration,
            stage: self.stage.clone(),
        });
    }

    /// Log results grouped by stage (matches React Flow's `FrameRecorder.getFrames()`).
    fn log_grouped(&self, label: &str) {
        // Collect stages in order of first appearance
        let mut stage_order: Vec<String> = Vec::new();
        let mut stage_frames: Vec<Vec<f64>> = Vec::new();

        for frame in &self.frames {
            if let Some(idx) = stage_order.iter().position(|s| s == &frame.stage) {
                stage_frames[idx].push(frame.duration);
            } else {
                stage_order.push(frame.stage.clone());
                stage_frames.push(vec![frame.duration]);
            }
        }

        let mut parts = Vec::new();
        for (stage, durations) in stage_order.iter().zip(stage_frames.iter()) {
            let values: Vec<String> = durations.iter().map(|d| format!("{:.1}", d)).collect();
            parts.push(format!("\"{}\":[{}]", stage, values.join(",")));
        }
        log(&format!(
            "[{}] Frame durations: {{{}}}",
            label,
            parts.join(",")
        ));

        // Observable-compatible format
        let observable: Vec<String> = self
            .frames
            .iter()
            .enumerate()
            .map(|(i, f)| {
                format!(
                    "{{\"index\":{},\"duration\":{:.1},\"stage\":\"{}\"}}",
                    i, f.duration, f.stage
                )
            })
            .collect();
        log(&format!(
            "[{}] Frame durations for Observable: [{}]",
            label,
            observable.join(",")
        ));
    }

    fn recent_fps(&self) -> f64 {
        let recent: Vec<f64> = self
            .frames
            .iter()
            .rev()
            .take(60)
            .map(|f| f.duration)
            .collect();
        if recent.is_empty() {
            return 0.0;
        }
        let avg = recent.iter().sum::<f64>() / recent.len() as f64;
        if avg > 0.0 {
            1000.0 / avg
        } else {
            0.0
        }
    }

    fn last_frame_ms(&self) -> f64 {
        self.frames.last().map(|f| f.duration).unwrap_or(0.0)
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

    /// Total frames including the trailing record frame.
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

struct Benchmark {
    kind: BenchmarkKind,
    recorder: FrameRecorder,
    frame: u32,
}

impl Benchmark {
    fn new(kind: BenchmarkKind) -> Self {
        Self {
            kind,
            recorder: FrameRecorder::new(500),
            frame: 0,
        }
    }

    /// Execute one frame of the benchmark. Returns `true` when complete.
    ///
    /// Timing model: `record_frame()` runs first, capturing the previous frame's
    /// complete duration (mutation + render + compositing). The stage label was set
    /// during the previous call, so it correctly describes the work that was measured.
    /// This matches React Flow's rAF-based FrameRecorder.
    fn step(&mut self, flow: &mut Flow<TextContent, StepEdge>, cols: usize, rows: usize) -> bool {
        // Record the previous frame's duration (stage label is from previous step)
        if self.frame > 0 {
            self.recorder.record_frame();
        }

        let total = self.kind.total_frames();

        // Last frame is trailing record only — no mutation
        if self.frame >= total {
            self.complete();
            return true;
        }

        // Execute the operation for this frame
        match self.kind {
            BenchmarkKind::DragInViewport => match self.frame {
                0 => {
                    self.recorder.set_stage("mousedown");
                    flow.select_node("n18_0");
                }
                1..=20 => {
                    self.recorder.set_stage("mousemove");
                    flow.move_node("n18_0", (-5.0, 0.0));
                }
                21 => {
                    self.recorder.set_stage("mouseup");
                    // No-op — just records the final move frame
                }
                _ => {
                    // Trailing record frame
                }
            },
            BenchmarkKind::SelectNode => {
                if self.frame == 0 {
                    self.recorder.set_stage("select");
                    let mid_col = cols / 2;
                    let mid_row = rows / 2;
                    let node_id = format!("n{}_{}", mid_col, mid_row);
                    flow.select_node(&node_id);
                }
            }
            BenchmarkKind::Remount => {
                if self.frame == 0 {
                    self.recorder.set_stage("remount");
                    let nodes = generate_chain_nodes(cols, rows);
                    let edges = generate_chain_edges(cols, rows);
                    *flow = Flow::with_graph(nodes, edges).expect("valid graph");
                    flow.min_zoom = 0.2;
                    flow.request_fit_view();
                }
            }
        }

        self.frame += 1;
        false
    }

    fn complete(&self) {
        self.recorder.log_grouped(self.kind.label());
    }
}

/// Runs multiple benchmarks in sequence with idle frames between them.
struct BenchmarkSuite {
    queue: Vec<BenchmarkKind>,
    current: Option<Benchmark>,
    /// Idle frames between benchmarks for the browser to settle.
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

    /// Advance one frame. Returns `true` when the entire suite is done.
    fn step(&mut self, flow: &mut Flow<TextContent, StepEdge>, cols: usize, rows: usize) -> bool {
        // Cooldown between benchmarks
        if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
            return false;
        }

        // Start next benchmark if none active
        if self.current.is_none() {
            if let Some(kind) = self.queue.first().copied() {
                self.queue.remove(0);
                log(&format!("--- Starting benchmark: {} ---", kind.label()));
                self.current = Some(Benchmark::new(kind));
            } else {
                return true;
            }
        }

        // Step current benchmark
        if let Some(bench) = &mut self.current {
            if bench.step(flow, cols, rows) {
                self.current = None;
                self.cooldown_remaining = self.cooldown;
            }
        }

        false
    }
}

// ---------------------------------------------------------------------------
// Demo
// ---------------------------------------------------------------------------

pub struct StressTestDemo {
    flow: Flow<TextContent, StepEdge>,
    display_recorder: FrameRecorder,
    suite: Option<BenchmarkSuite>,
    cols: usize,
    rows: usize,
    node_count: usize,
    edge_count: usize,
}

impl StressTestDemo {
    pub fn new() -> Self {
        let (cols, rows) = get_grid_size_from_url();
        let node_count = cols * rows;
        let edge_count = node_count.saturating_sub(1);

        log(&format!(
            "Generating {}x{} chain: {} nodes, {} edges",
            cols, rows, node_count, edge_count
        ));

        let nodes = generate_chain_nodes(cols, rows);
        let edges = generate_chain_edges(cols, rows);

        let mut flow = Flow::with_graph(nodes, edges).expect("valid graph");
        flow.min_zoom = 0.2;

        Self {
            flow,
            display_recorder: FrameRecorder::new(500),
            suite: None,
            cols,
            rows,
            node_count,
            edge_count,
        }
    }

    fn start_single(&mut self, kind: BenchmarkKind) {
        log(&format!("Benchmark: {}", kind.label()));
        self.suite = Some(BenchmarkSuite::new(vec![kind]));
    }

    fn start_all(&mut self) {
        log("=== Running all benchmarks ===");
        self.suite = Some(BenchmarkSuite::new(vec![
            BenchmarkKind::DragInViewport,
            BenchmarkKind::SelectNode,
            BenchmarkKind::Remount,
        ]));
    }
}

impl Default for StressTestDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl Demo for StressTestDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        // Ignore input during benchmarks to avoid interference
        if self.suite.as_ref().is_some_and(|s| s.is_active()) {
            return;
        }

        match event.code {
            rataflow::KeyCode::Char('f') => self.flow.request_fit_view(),
            rataflow::KeyCode::Char('l') => self.display_recorder.log_grouped("display"),
            rataflow::KeyCode::Char('t') => {
                self.start_single(BenchmarkKind::DragInViewport);
            }
            rataflow::KeyCode::Char('s') => {
                self.start_single(BenchmarkKind::SelectNode);
            }
            rataflow::KeyCode::Char('r') => {
                self.start_single(BenchmarkKind::Remount);
            }
            rataflow::KeyCode::Char('a') => {
                self.start_all();
            }
            _ => {
                let response = self.flow.handle_controls_key_event(event);
                if matches!(response, EventResponse::NotHandled) {
                    self.flow.handle_key_event(event);
                }
            }
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        // Ignore mouse during benchmarks
        if self.suite.as_ref().is_some_and(|s| s.is_active()) {
            return;
        }
        self.flow.handle_mouse_event(event);
    }

    fn flow_ops(&mut self) -> &mut dyn rataflow::FlowOps {
        &mut self.flow
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Advance benchmark if active
        if let Some(suite) = &mut self.suite {
            if suite.step(&mut self.flow, self.cols, self.rows) {
                log("=== All benchmarks complete ===");
                self.suite = None;
            }
        }

        // Record frame for display FPS
        self.display_recorder.record_frame();

        // Render
        frame.render_widget(
            Background::new(&self.flow)
                .variant(BackgroundVariant::Dots)
                .gap(10, 10),
            area,
        );
        frame.render_widget(&mut self.flow, area);
        frame.render_widget(Controls::new(&self.flow), area);

        // Stats overlay
        let fps = self.display_recorder.recent_fps();
        let frame_ms = self.display_recorder.last_frame_ms();

        let bench_label = self
            .suite
            .as_ref()
            .filter(|s| s.is_active())
            .map(|s| s.current_label())
            .unwrap_or("idle");

        let stats_text = format!(
            " {} nodes | {} edges | FPS: {:.0} | Frame: {:.1}ms | {} ",
            self.node_count, self.edge_count, fps, frame_ms, bench_label
        );

        let color = if fps >= 55.0 {
            Color::Indexed(71)
        } else if fps >= 30.0 {
            Color::Indexed(179)
        } else {
            Color::Indexed(167)
        };

        let stats_area = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: 1,
        };

        frame.render_widget(
            Paragraph::new(stats_text)
                .style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
            stats_area,
        );
    }
}
