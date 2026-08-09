#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use sysinfo::{System, Pid};
use chrono::Local;
use std::sync::mpsc::{channel, Sender, Receiver};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    format: String, // Force JSON output
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize, Debug)]
struct AgentOutput {
    message: String,
    action: String,
    target: Option<usize>,
}

#[derive(Clone, Debug)]
enum AgentMessage {
    Log(String),
    ProposedAction { action: String, target: Option<usize>, description: String },
}

#[derive(Clone)]
struct PendingAction {
    action: String,
    target: Option<usize>,
    description: String,
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("NexusOS Agentic Environment")
            .with_fullscreen(true)
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "NexusOS",
        options,
        Box::new(|cc| Box::new(NexusDesktopApp::new(cc))),
    )
}

struct NexusDesktopApp {
    sys: System,
    command_input: String,
    terminal_logs: Vec<String>,
    agent_tx: Sender<String>,
    gui_rx: Receiver<AgentMessage>,
    pending_action: Option<PendingAction>,
    show_dashboard: bool,
    show_terminal: bool,
}

impl NexusDesktopApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.window_fill = egui::Color32::from_rgba_premultiplied(18, 18, 22, 240); // Transparent windows
        style.visuals.panel_fill = egui::Color32::from_rgb(25, 25, 30);
        cc.egui_ctx.set_style(style);

        let mut sys = System::new_all();
        sys.refresh_all();

        let (gui_tx, gui_rx) = channel();
        let (agent_tx, agent_rx) = channel();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let gui_tx_clone = gui_tx.clone();
        rt.spawn(async move {
            run_agent_loop(agent_rx, gui_tx_clone).await;
        });

        // We do not store `rt` because we don't need to read it, but it must be kept alive.
        // `rt` drops when `NexusDesktopApp` drops if we store it. Since we just `spawn` it, we can store it or leak it.
        // Eframe will keep it alive if we store it. 
        // We'll store it in a forgotten box to avoid the warning, or just use std::thread for the runtime block.
        // Actually, just storing it and ignoring the warning is fine, but let's silence it.
        let _ = Box::leak(Box::new(rt)); 

        Self {
            sys,
            command_input: String::new(),
            terminal_logs: vec!["[System] NexusOS Desktop Environment initialized.".into(), "[System] Connected to Local LLM Gateway.".into()],
            agent_tx,
            gui_rx,
            pending_action: None,
            show_dashboard: true,
            show_terminal: true,
        }
    }
}

async fn run_agent_loop(rx: Receiver<String>, tx: Sender<AgentMessage>) {
    let client = reqwest::Client::new();
    
    if let Err(_) = client.get("http://localhost:11434/").send().await {
        let _ = tx.send(AgentMessage::Log("[Warning] Ollama not found on port 11434.".to_string()));
    }

    loop {
        if let Ok(user_msg) = rx.try_recv() {
            let system_prompt = format!(
                "You are NexusOS, an advanced agentic operating system.\n\
                You MUST respond in strict JSON format with exactly three fields: 'message' (string), 'action' (string), 'target' (integer or null).\n\
                AVAILABLE ACTIONS:\n\
                - 'KILL_PID': Set target to the process ID you want to kill.\n\
                - 'GET_STATS': Set target to null.\n\
                - 'NONE': If no action is needed, set target to null.\n\
                User request: '{}'",
                user_msg
            );

            let req_body = OllamaRequest {
                model: "llama3".to_string(),
                prompt: system_prompt,
                stream: false,
                format: "json".to_string(), // Force JSON schema
            };

            match client.post("http://localhost:11434/api/generate")
                .json(&req_body)
                .send()
                .await 
            {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<OllamaResponse>().await {
                        // Parse the strict JSON output
                        match serde_json::from_str::<AgentOutput>(&json.response) {
                            Ok(agent_out) => {
                                let _ = tx.send(AgentMessage::Log(format!("[Agent] {}", agent_out.message)));
                                
                                if agent_out.action == "KILL_PID" || agent_out.action == "GET_STATS" {
                                    let desc = format!("The AI wants to execute {} on target {:?}", agent_out.action, agent_out.target);
                                    let _ = tx.send(AgentMessage::ProposedAction {
                                        action: agent_out.action,
                                        target: agent_out.target,
                                        description: desc,
                                    });
                                }
                            }
                            Err(_) => {
                                let _ = tx.send(AgentMessage::Log("[Agent Error] AI hallucinated invalid JSON schema. Execution blocked.".to_string()));
                            }
                        }
                    } else {
                        let _ = tx.send(AgentMessage::Log("[Agent Error] Failed to parse Ollama response".to_string()));
                    }
                }
                Err(e) => {
                    let _ = tx.send(AgentMessage::Log(format!("[Agent Error] LLM Unreachable: {}", e)));
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

impl eframe::App for NexusDesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        while let Ok(msg) = self.gui_rx.try_recv() {
            match msg {
                AgentMessage::Log(l) => self.terminal_logs.push(l),
                AgentMessage::ProposedAction { action, target, description } => {
                    // Send it to the UAC approval queue!
                    self.pending_action = Some(PendingAction { action, target, description });
                }
            }
        }

        // DESKTOP BACKGROUND
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_rgb(10, 20, 30))) // Deep blue desktop
            .show(ctx, |ui| {
                // Background text/logo
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new("NEXUS OS").size(100.0).color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)));
                });
            });

        // TOP BAR
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(10, 10, 15, 240)).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("NEXUS").color(egui::Color32::from_rgb(0, 200, 255)).strong());
                    ui.separator();
                    ui.toggle_value(&mut self.show_terminal, "Agent Terminal");
                    ui.toggle_value(&mut self.show_dashboard, "System Dashboard");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Shutdown").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.separator();
                        let time_str = Local::now().format("%H:%M:%S").to_string();
                        ui.label(egui::RichText::new(time_str).strong());
                    });
                });
            });

        // FLOATING AGENT TERMINAL WINDOW
        if self.show_terminal {
            egui::Window::new("Agent Core")
                .default_size([400.0, 500.0])
                .default_pos([50.0, 50.0])
                .show(ctx, |ui| {
                    ui.label("Ask the AI agent to manage your system.");
                    ui.separator();
                    
                    egui::ScrollArea::vertical().max_height(ui.available_height() - 40.0).stick_to_bottom(true).show(ui, |ui| {
                        for log in &self.terminal_logs {
                            if log.starts_with(">") {
                                ui.colored_label(egui::Color32::from_rgb(0, 255, 150), log);
                            } else if log.starts_with("[Agent]") {
                                ui.colored_label(egui::Color32::from_rgb(200, 150, 255), log);
                            } else if log.starts_with("[System Error]") || log.starts_with("[Warning]") || log.starts_with("[Agent Error]") {
                                ui.colored_label(egui::Color32::RED, log);
                            } else {
                                ui.label(log);
                            }
                        }
                    });

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.horizontal(|ui| {
                            let response = ui.add_sized(
                                [ui.available_width() - 50.0, 30.0],
                                egui::TextEdit::singleline(&mut self.command_input).hint_text("Command...")
                            );
                            if ui.button("Send").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                if !self.command_input.trim().is_empty() {
                                    self.terminal_logs.push(format!("> {}", self.command_input));
                                    let _ = self.agent_tx.send(self.command_input.clone());
                                    self.command_input.clear();
                                    response.request_focus();
                                }
                            }
                        });
                    });
                });
        }

        // FLOATING SYSTEM DASHBOARD WINDOW
        if self.show_dashboard {
            egui::Window::new("System Dashboard")
                .default_size([600.0, 400.0])
                .default_pos([500.0, 50.0])
                .show(ctx, |ui| {
                    let cpu = self.sys.global_cpu_info().cpu_usage();
                    let used_ram = self.sys.used_memory() / 1024 / 1024;
                    let total_ram = self.sys.total_memory() / 1024 / 1024;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("CPU: {:.1}%", cpu)).strong().color(egui::Color32::LIGHT_BLUE));
                        ui.separator();
                        ui.label(egui::RichText::new(format!("RAM: {}/{} MB", used_ram, total_ram)).strong().color(egui::Color32::LIGHT_GREEN));
                    });
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("process_grid")
                            .num_columns(4)
                            .spacing([40.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("PID").strong());
                                ui.label(egui::RichText::new("Name").strong());
                                ui.label(egui::RichText::new("CPU %").strong());
                                ui.label(egui::RichText::new("Memory (MB)").strong());
                                ui.end_row();

                                let mut processes: Vec<_> = self.sys.processes().iter().collect();
                                processes.sort_by(|a, b| b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap_or(core::cmp::Ordering::Equal));

                                for (pid, process) in processes.iter().take(25) {
                                    ui.label(pid.to_string());
                                    ui.label(process.name());
                                    ui.label(format!("{:.1}", process.cpu_usage()));
                                    ui.label(format!("{:.1}", (process.memory() as f64) / 1024.0 / 1024.0));
                                    ui.end_row();
                                }
                            });
                    });
                });
        }

        // SECURITY GUARDRAIL (UAC MODAL)
        if let Some(action) = self.pending_action.clone() {
            egui::Window::new("⚠️ Security Verification Required")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("The AI Agent is requesting permission to execute an OS action.").color(egui::Color32::YELLOW));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(&action.description).strong().size(16.0));
                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        if ui.button("🚫 Deny").clicked() {
                            self.terminal_logs.push("[Security] Action blocked by User.".to_string());
                            self.pending_action = None;
                        }
                        if ui.button("✅ Approve").clicked() {
                            // EXECUTE APPROVED ACTION
                            if action.action == "KILL_PID" {
                                if let Some(pid_val) = action.target {
                                    let pid = Pid::from(pid_val);
                                    self.sys.refresh_processes();
                                    if let Some(process) = self.sys.process(pid) {
                                        process.kill();
                                        self.terminal_logs.push(format!("[System] Sent SIGKILL to PID {}", pid_val));
                                    } else {
                                        self.terminal_logs.push(format!("[System] Failed: PID {} not found", pid_val));
                                    }
                                }
                            } else if action.action == "GET_STATS" {
                                let cpu = self.sys.global_cpu_info().cpu_usage();
                                let used_ram = self.sys.used_memory() / 1024 / 1024;
                                self.terminal_logs.push(format!("[System Stats] CPU: {:.1}%, RAM: {} MB used", cpu, used_ram));
                            }
                            self.pending_action = None;
                        }
                    });
                });
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100)); // Higher refresh rate for smooth windows
    }
}
