mod layout;

use crate::config::Config;
use crate::graph::{Graph, NodeType};
use egui::{Color32, Pos2, Sense, Stroke, Vec2};
use layout::Layout;
use std::collections::HashSet;
use std::sync::mpsc;

/// Categorical palette (colorblind-checked, fixed slot order), light and dark steps.
fn colour(kind: NodeType, dark: bool) -> Color32 {
    let (l, d) = match kind {
        NodeType::Repo => (0x2a78d6, 0x3987e5),
        NodeType::Agent => (0xeb6834, 0xd95926),
        NodeType::Service => (0x1baf7a, 0x199e70),
        NodeType::Framework => (0xeda100, 0xc98500),
        NodeType::Runtime => (0xe87ba4, 0xd55181),
        NodeType::Container => (0x008300, 0x008300),
        NodeType::Port => (0x6250d6, 0x9085e9),
        NodeType::Mcp => (0xe34948, 0xe66767),
    };
    let h = if dark { d } else { l };
    Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
}

#[derive(PartialEq)]
enum View {
    Graph,
    Table,
}

pub struct App {
    graph: Graph,
    config: Config,
    layout: Layout,
    hidden: HashSet<NodeType>,
    query: String,
    selected: Option<String>,
    view: View,
    pan: Vec2,
    zoom: f32,
    dragging: Option<usize>,
    scanning: Option<mpsc::Receiver<Graph>>,
    log: Vec<String>,
}

impl App {
    fn new(graph: Graph, config: Config) -> Self {
        let layout = Layout::new(&graph, Pos2::ZERO);
        Self {
            graph,
            config,
            layout,
            hidden: [NodeType::Port].into(),
            query: String::new(),
            selected: None,
            view: View::Graph,
            pan: Vec2::ZERO,
            zoom: 1.0,
            dragging: None,
            scanning: None,
            log: vec![],
        }
    }

    fn visible(&self) -> Vec<bool> {
        let q = self.query.to_lowercase();
        self.graph
            .nodes
            .iter()
            .map(|n| {
                !self.hidden.contains(&n.kind)
                    && (q.is_empty()
                        || n.label.to_lowercase().contains(&q)
                        || n.meta.values().any(|v| v.to_lowercase().contains(&q)))
            })
            .collect()
    }

    fn rescan(&mut self) {
        let (tx, rx) = mpsc::channel();
        let cfg = self.config.clone();
        std::thread::spawn(move || {
            let g = crate::scan::scan(&cfg, |_| {});
            if let Ok(cwd) = std::env::current_dir() {
                let _ = crate::scan::save(&g, &cwd);
            }
            let _ = tx.send(g);
        });
        self.scanning = Some(rx);
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("stacklens");
            ui.label(egui::RichText::new(format!("scanned {}", self.graph.generated_at)).weak());
            ui.add_space(12.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search nodes…")
                    .desired_width(260.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let busy = self.scanning.is_some();
                if ui
                    .add_enabled(
                        !busy,
                        egui::Button::new(if busy { "Scanning…" } else { "Rescan" }),
                    )
                    .clicked()
                {
                    self.rescan();
                }
                ui.selectable_value(&mut self.view, View::Table, "Table");
                ui.selectable_value(&mut self.view, View::Graph, "Graph");
            });
        });
    }

    fn stats(&self, ui: &mut egui::Ui) {
        let count = |k: NodeType| self.graph.of_type(k).count();
        let sessions: usize = self
            .graph
            .of_type(NodeType::Repo)
            .filter_map(|n| n.meta.get("claudeSessions")?.parse::<usize>().ok())
            .sum();
        let tiles = [
            ("Repos", count(NodeType::Repo)),
            ("AI agents", count(NodeType::Agent)),
            ("Services", count(NodeType::Service)),
            ("Frameworks", count(NodeType::Framework)),
            ("Runtimes", count(NodeType::Runtime)),
            ("Connections", self.graph.edges.len()),
            ("Claude sessions", sessions),
        ];
        ui.horizontal_wrapped(|ui| {
            for (l, n) in tiles {
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(n.to_string()).size(22.0).strong());
                            ui.label(egui::RichText::new(l).small().weak());
                        });
                    });
            }
        });
    }

    fn legend(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        ui.horizontal_wrapped(|ui| {
            for k in NodeType::ALL {
                let n = self.graph.of_type(k).count();
                let on = !self.hidden.contains(&k);
                let text = egui::RichText::new(format!("● {} {}", k.label(), n)).color(if on {
                    colour(k, dark)
                } else {
                    ui.visuals().weak_text_color()
                });
                if ui.add(egui::Button::new(text).frame(on)).clicked() {
                    if on {
                        self.hidden.insert(k);
                    } else {
                        self.hidden.remove(&k);
                    }
                    self.layout.reheat();
                }
            }
        });
    }

    fn graph_view(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
        let visible = self.visible();
        self.layout.step(Pos2::ZERO, &visible);
        if self.layout.alpha > 0.002 {
            ui.ctx().request_repaint();
        }

        let centre = rect.center();
        let to_screen = |p: Pos2, pan: Vec2, zoom: f32| centre + (p.to_vec2() * zoom) + pan;
        let to_world = |p: Pos2, pan: Vec2, zoom: f32| Pos2::ZERO + ((p - centre) - pan) / zoom;

        // Zoom with scroll, pan by dragging empty space.
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let z = (self.zoom * (1.0 + scroll * 0.002)).clamp(0.2, 4.0);
                if let Some(m) = resp.hover_pos() {
                    let w = to_world(m, self.pan, self.zoom);
                    self.zoom = z;
                    self.pan = m - centre - w.to_vec2() * z;
                }
            }
        }
        let hover_world = resp.hover_pos().map(|m| to_world(m, self.pan, self.zoom));
        let hit = hover_world.and_then(|w| {
            self.layout
                .bodies
                .iter()
                .enumerate()
                .filter(|(i, _)| visible[*i])
                .find(|(_, b)| (b.pos - w).length() <= b.r / self.zoom.max(0.5) + 6.0)
                .map(|(i, _)| i)
        });

        if resp.drag_started() {
            self.dragging = hit;
        }
        if resp.dragged() {
            match self.dragging {
                Some(i) => {
                    let w = to_world(resp.interact_pointer_pos().unwrap(), self.pan, self.zoom);
                    let b = &mut self.layout.bodies[i];
                    b.pos = w;
                    b.pinned = true;
                    self.layout.reheat();
                }
                None => self.pan += resp.drag_delta(),
            }
        }
        if resp.drag_stopped() {
            if let Some(i) = self.dragging {
                self.layout.bodies[i].pinned = false;
            }
            self.dragging = None;
        }
        if resp.clicked() {
            self.selected = hit.map(|i| self.layout.bodies[i].id.clone());
        }

        // Neighbourhood of the selection.
        let sel = self
            .selected
            .as_deref()
            .and_then(|id| self.layout.index(id));
        let mut neigh = HashSet::new();
        if let Some(s) = sel {
            for &(a, b) in &self.layout.links {
                if a == s {
                    neigh.insert(b);
                }
                if b == s {
                    neigh.insert(a);
                }
            }
        }

        let link_col = if dark {
            Color32::from_gray(74)
        } else {
            Color32::from_gray(184)
        };
        let accent = colour(NodeType::Repo, dark);
        for &(a, b) in &self.layout.links {
            if !visible[a] || !visible[b] {
                continue;
            }
            let hi = sel.is_some_and(|s| a == s || b == s);
            let dim = sel.is_some() && !hi;
            let col = if hi {
                accent
            } else if dim {
                link_col.gamma_multiply(0.25)
            } else {
                link_col
            };
            painter.line_segment(
                [
                    to_screen(self.layout.bodies[a].pos, self.pan, self.zoom),
                    to_screen(self.layout.bodies[b].pos, self.pan, self.zoom),
                ],
                Stroke::new(if hi { 2.0 } else { 1.5 }, col),
            );
        }
        let font = egui::FontId::proportional(11.0 * self.zoom.clamp(0.8, 1.6));
        for (i, b) in self.layout.bodies.iter().enumerate() {
            if !visible[i] {
                continue;
            }
            let n = &self.graph.nodes[i];
            let p = to_screen(b.pos, self.pan, self.zoom);
            if !rect.expand(40.0).contains(p) {
                continue;
            }
            let dim = sel.is_some() && sel != Some(i) && !neigh.contains(&i);
            let mut col = colour(n.kind, dark);
            if dim {
                col = col.gamma_multiply(0.22);
            }
            let r = b.r * self.zoom.clamp(0.6, 2.0);
            painter.circle(
                p,
                r,
                col,
                Stroke::new(
                    if sel == Some(i) || hit == Some(i) {
                        2.5
                    } else {
                        1.5
                    },
                    ui.visuals().extreme_bg_color,
                ),
            );
            if !dim || self.zoom > 1.5 {
                let tc = if dim {
                    ui.visuals().weak_text_color()
                } else {
                    ui.visuals().text_color()
                };
                painter.text(
                    p + Vec2::new(0.0, r + 3.0),
                    egui::Align2::CENTER_TOP,
                    &n.label,
                    font.clone(),
                    tc,
                );
            }
        }
        if let Some(i) = hit {
            let n = &self.graph.nodes[i];
            egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), egui::Id::new("tip"), |ui| {
                ui.strong(&n.label);
                ui.label(format!(
                    "{} · {} links",
                    n.kind.label(),
                    self.graph.degree(&n.id)
                ));
            });
        }
        painter.text(
            rect.left_bottom() + Vec2::new(10.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            "drag to pan · scroll to zoom · drag a node to pin it · click for details",
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
    }

    fn table_view(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        let visible = self.visible();
        let mut rows: Vec<usize> = (0..self.graph.nodes.len())
            .filter(|&i| visible[i])
            .collect();
        rows.sort_by(|&a, &b| {
            let (x, y) = (&self.graph.nodes[a], &self.graph.nodes[b]);
            x.kind.cmp(&y.kind).then(x.label.cmp(&y.label))
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("table")
                .striped(true)
                .num_columns(4)
                .min_col_width(60.0)
                .show(ui, |ui| {
                    ui.strong("Type");
                    ui.strong("Name");
                    ui.strong("Links");
                    ui.strong("Details");
                    ui.end_row();
                    for i in rows {
                        let n = &self.graph.nodes[i];
                        ui.colored_label(colour(n.kind, dark), format!("● {}", n.kind.label()));
                        if ui.link(&n.label).clicked() {
                            self.selected = Some(n.id.clone());
                        }
                        ui.label(self.graph.degree(&n.id).to_string());
                        ui.label(
                            egui::RichText::new(
                                n.meta
                                    .iter()
                                    .take(4)
                                    .map(|(k, v)| format!("{k}: {v}"))
                                    .collect::<Vec<_>>()
                                    .join(" · "),
                            )
                            .small()
                            .weak(),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    fn panel(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        let Some(n) = self
            .selected
            .as_deref()
            .and_then(|id| self.graph.get(id))
            .cloned()
        else {
            ui.label(egui::RichText::new("Select a node to see what it connects to.").weak());
            return;
        };
        ui.label(
            egui::RichText::new(format!(
                "{} · via {}",
                n.kind.label().to_uppercase(),
                n.source
            ))
            .small()
            .weak(),
        );
        ui.horizontal(|ui| {
            ui.colored_label(colour(n.kind, dark), "●");
            ui.heading(&n.label);
        });
        ui.add_space(6.0);
        egui::Grid::new("meta")
            .num_columns(2)
            .spacing([12.0, 3.0])
            .show(ui, |ui| {
                for (k, v) in &n.meta {
                    ui.label(egui::RichText::new(k).weak());
                    ui.add(egui::Label::new(v).wrap());
                    ui.end_row();
                }
            });
        let mut next = None;
        for (title, outgoing) in [("Uses", true), ("Used by", false)] {
            let list: Vec<_> = self
                .graph
                .edges
                .iter()
                .filter(|e| {
                    if outgoing {
                        e.from == n.id
                    } else {
                        e.to == n.id
                    }
                })
                .filter_map(|e| {
                    Some((
                        e.kind.as_str(),
                        self.graph.get(if outgoing { &e.to } else { &e.from })?,
                    ))
                })
                .collect();
            if list.is_empty() {
                continue;
            }
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!("{} ({})", title.to_uppercase(), list.len()))
                    .small()
                    .strong(),
            );
            for (kind, m) in list {
                ui.horizontal(|ui| {
                    ui.colored_label(colour(m.kind, dark), "●");
                    if ui.link(&m.label).clicked() {
                        next = Some(m.id.clone());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(kind).small().weak())
                    });
                });
            }
        }
        if next.is_some() {
            self.selected = next;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.scanning {
            if let Ok(g) = rx.try_recv() {
                self.layout = Layout::new(&g, Pos2::ZERO);
                self.graph = g;
                self.scanning = None;
                self.selected = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(4.0);
            self.stats(ui);
            ui.add_space(2.0);
            self.legend(ui);
            ui.add_space(4.0);
        });
        egui::SidePanel::right("panel")
            .default_width(300.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.panel(ui));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| match self.view {
                View::Graph => self.graph_view(ui),
                View::Table => self.table_view(ui),
            });
        let _ = &self.log;
    }
}

pub fn run(graph: Graph, config: Config) {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 880.0])
            .with_title("stacklens"),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "stacklens",
        opts,
        Box::new(|cc| {
            cc.egui_ctx.set_theme(egui::ThemePreference::System);
            Ok(Box::new(App::new(graph, config)))
        }),
    ) {
        eprintln!("ui error: {e}");
    }
}
