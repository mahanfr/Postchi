use eframe::egui;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{self},
};

#[derive(Clone, Serialize, Deserialize)]
struct Header {
    key: String,
    value: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct SavedRequest {
    method: String,
    url: String,
    headers: Vec<Header>,
    body: String,
}

#[derive(Clone)]
struct RequestTab {
    #[allow(dead_code)]
    id: u64,
    name: String,

    method_index: usize,
    url: String,

    headers: Vec<Header>,
    body: String,

    response: Arc<Mutex<String>>,
    response_headers: Arc<Mutex<String>>,
    response_status: Arc<Mutex<String>>,
    response_status_code: Arc<Mutex<u16>>,
    response_time: Arc<Mutex<Option<time::Duration>>>,

    is_loading: Arc<AtomicBool>,
}

impl RequestTab {
    fn new(id: u64) -> Self {
        Self {
            id,
            name: format!("Request {}", id),

            method_index: 0,
            url: String::new(),

            headers: vec![Header {
                key: "Content-Type".into(),
                value: "application/json".into(),
            }],

            body: String::new(),

            response: Arc::new(Mutex::new(String::new())),
            response_headers: Arc::new(Mutex::new(String::new())),
            response_status: Arc::new(Mutex::new("Ready".into())),
            response_status_code: Arc::new(Mutex::new(0)),
            response_time: Arc::new(Mutex::new(None)),

            is_loading: Arc::new(AtomicBool::new(false)),
        }
    }
}

struct PostmanApp {
    methods: Vec<&'static str>,
    tabs: Vec<RequestTab>,
    active_tab: usize,
    next_tab_id: u64,
}

impl Default for PostmanApp {
    fn default() -> Self {
        Self {
            methods: vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"],
            tabs: vec![RequestTab::new(1)],
            active_tab: 0,
            next_tab_id: 2,
        }
    }
}

impl PostmanApp {
    fn active_tab_mut(&mut self) -> &mut RequestTab {
        &mut self.tabs[self.active_tab]
    }

    fn active_tab(&self) -> &RequestTab {
        &self.tabs[self.active_tab]
    }

    fn save_request(&self) {
        let tab = self.active_tab();
        let request = SavedRequest {
            method: self.methods[tab.method_index].to_string(),
            url: tab.url.clone(),
            headers: tab.headers.clone(),
            body: tab.body.clone(),
        };

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("json", &["json"])
            .save_file()
        {
            let _ = std::fs::write(path, serde_json::to_string_pretty(&request).unwrap());
        }
    }

    fn load_request(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("json", &["json"])
            .pick_file()
        {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(req) = serde_json::from_str::<SavedRequest>(&content) {
                    let methods = self.methods.clone();
                    let tab = self.active_tab_mut();
                    tab.url = req.url;
                    tab.body = req.body;
                    tab.headers = req.headers;

                    if let Some(pos) = methods.iter().position(|m| *m == req.method) {
                        tab.method_index = pos;
                    }
                }
            }
        }
    }
    fn send_request(&self, tab_index: usize) {
        let tab = self.tabs[tab_index].clone();
        let loading = tab.is_loading.clone();
        loading.store(true, Ordering::Relaxed);

        let method = self.methods[tab.method_index].to_string();
        let url = tab.url.clone();
        let body = tab.body.clone();
        let headers = tab.headers.clone();

        let response_ref = tab.response.clone();
        let status_ref = tab.response_status.clone();
        let response_status_code = tab.response_status_code.clone();
        let response_headers = tab.response_headers.clone();
        let response_time = tab.response_time.clone();

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let start_time = time::Instant::now();

            runtime.block_on(async move {
                let client = reqwest::Client::new();

                let mut builder =
                    client.request(Method::from_bytes(method.as_bytes()).unwrap(), &url);

                for h in headers {
                    if !h.key.is_empty() {
                        builder = builder.header(h.key, h.value);
                    }
                }

                if !body.is_empty() {
                    builder = builder.body(body);
                }
                match builder.send().await {
                    Ok(resp) => {
                        let status = resp.status();

                        *response_status_code.lock().unwrap() = status.as_u16();

                        let mut header_string = String::new();

                        for (key, value) in resp.headers() {
                            header_string.push_str(&format!(
                                "{}: {}\n",
                                key,
                                value.to_str().unwrap_or("")
                            ));
                        }

                        *response_headers.lock().unwrap() = header_string;

                        let text = resp.text().await.unwrap_or_default();

                        let pretty = match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(json) => serde_json::to_string_pretty(&json).unwrap_or(text),
                            Err(_) => text,
                        };

                        *response_ref.lock().unwrap() = pretty;

                        *status_ref.lock().unwrap() = format!(
                            "{} {}",
                            status.as_u16(),
                            status.canonical_reason().unwrap_or("")
                        );
                        *response_time.lock().unwrap() = Some(time::Instant::now() - start_time);
                    }

                    Err(e) => {
                        *status_ref.lock().unwrap() = format!("Error: {}", e);
                    }
                }

                loading.store(false, Ordering::Relaxed);
            });
        });
    }
}

impl eframe::App for PostmanApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        if self.tabs[self.active_tab].is_loading.load(Ordering::Relaxed) {
            ctx.request_repaint();
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut close_idx = None;

                for (i, tab) in self.tabs.iter().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let selected = self.active_tab == i;

                            if ui.selectable_label(selected, &tab.name).clicked() {
                                self.active_tab = i;
                            }

                            if self.tabs.len() > 1 {
                                if ui.small_button("×").clicked() {
                                    close_idx = Some(i);
                                }
                            }
                        });
                    });
                }

                if ui.button("+").clicked() {
                    self.tabs.push(RequestTab::new(self.next_tab_id));

                    self.active_tab = self.tabs.len() - 1;

                    self.next_tab_id += 1;
                }

                if let Some(idx) = close_idx {
                    self.tabs.remove(idx);

                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    }
                }
            });
            ui.horizontal(|ui| {
                let methods = &self.methods;
                {
                    let tab = &mut self.tabs[self.active_tab];
                    egui::ComboBox::from_label("Method")
                        .selected_text(methods[tab.method_index])
                        .show_ui(ui, |ui| {
                            for (idx, method) in methods.iter().enumerate() {
                                ui.selectable_value(&mut tab.method_index, idx, *method);
                            }
                        });

                    ui.text_edit_singleline(&mut tab.url);

                    // if ui.button("Send").clicked() {
                    //     self.send_request();
                    // }
                    let loading = tab.is_loading.load(Ordering::Relaxed);
                    if ui.add_enabled(!loading, egui::Button::new("Send")).clicked() {
                        let idx = self.active_tab;
                        if tab.url.len() > 14 {
                            tab.name = format!("[{}] ...{}" ,methods[tab.method_index], tab.url.split_at(tab.url.len() - 10).1.to_string());
                        }
                        self.send_request(idx);
                    }
                    if loading {
                        ui.add(egui::Spinner::new());
                        ui.label("Sending...");
                    }

                    if ui.button("Save").clicked() {
                        self.save_request();
                    }

                    if ui.button("Load").clicked() {
                        self.load_request();
                    }
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let tab = &mut self.tabs[self.active_tab];
            ui.columns(2, |columns| {
                // REQUEST SIDE
                columns[0].vertical(|ui| {
                    ui.heading("Request");

                    ui.separator();

                    ui.label("Headers");

                    egui::ScrollArea::vertical()
                        .id_salt("response_headers_scroll")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            let mut remove = None;

                            for (i, header) in tab.headers.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [140.0, 24.0],
                                        egui::TextEdit::singleline(&mut header.key),
                                    );

                                    ui.add_sized(
                                        [220.0, 24.0],
                                        egui::TextEdit::singleline(&mut header.value),
                                    );

                                    if ui.button("❌").clicked() {
                                        remove = Some(i);
                                    }
                                });
                            }

                            if let Some(idx) = remove {
                                tab.headers.remove(idx);
                            }
                        });

                    if ui.button("+ Header").clicked() {
                        tab.headers.push(Header {
                            key: String::new(),
                            value: String::new(),
                        });
                    }

                    ui.separator();

                    ui.label("Body");

                    egui::ScrollArea::vertical()
                        .id_salt("response_body_scroll")
                        .max_height(500.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut tab.body)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(20)
                                    .desired_width(f32::INFINITY),
                            );
                        });
                });

                // RESPONSE SIDE
                columns[1].vertical(|ui| {
                    ui.heading("Response");

                    let code = *tab.response_status_code.lock().unwrap();

                    let color = match code {
                        200..=299 => egui::Color32::GREEN,

                        300..=399 => egui::Color32::YELLOW,

                        400..=499 => egui::Color32::from_rgb(255, 140, 0),

                        500..=599 => egui::Color32::RED,

                        _ => egui::Color32::GRAY,
                    };
                    ui.horizontal(|ui| {
                        ui.colored_label(color, tab.response_status.lock().unwrap().clone());
                        let res_duration = tab.response_time.lock().unwrap();
                        if res_duration.is_some() {
                            ui.label(format!("{} ms", res_duration.unwrap().as_millis()));
                        }
                    });

                    ui.separator();

                    egui::CollapsingHeader::new("Response Headers")
                        .default_open(false)
                        .show(ui, |ui| {
                            let mut headers = tab.response_headers.lock().unwrap().clone();

                            ui.add(
                                egui::TextEdit::multiline(&mut headers)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(8)
                                    .interactive(false),
                            );
                        });

                    ui.separator();

                    let mut response = tab.response.lock().unwrap().clone();

                    egui::ScrollArea::vertical()
                        .id_salt("response_scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut response)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(40)
                                    .interactive(false),
                            );
                        });
                });
            });
        });
    }
}
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 900.0])
            .with_min_inner_size([1200.0, 700.0]),
        ..Default::default()
    };

    let mut style = egui::Style::default();

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);

    style.visuals = egui::Visuals::dark();

    eframe::run_native(
        "Postchi",
        options,
        Box::new(|_| Ok(Box::new(PostmanApp::default()))),
    )
}
