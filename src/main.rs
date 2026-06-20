use eframe::egui;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::{sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time};

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

struct PostmanApp {
    method_index: usize,
    methods: Vec<&'static str>,

    url: String,
    headers: Vec<Header>,
    body: String,

    response_headers: Arc<Mutex<String>>,
    response_status_code: Arc<Mutex<u16>>,
    response: Arc<Mutex<String>>,
    response_time: Arc<Mutex<Option<time::Duration>>>,
    status: Arc<Mutex<String>>,

    is_loading: Arc<AtomicBool>,
}

impl Default for PostmanApp {
    fn default() -> Self {
        Self {
            method_index: 0,
            methods: vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"],
            url: String::new(),
            headers: vec![Header {
                key: "Content-Type".into(),
                value: "application/json".into(),
            }],
            body: String::new(),
            response: Arc::new(Mutex::new(String::new())),
            status: Arc::new(Mutex::new("Ready".into())),
            response_headers: Arc::new(Mutex::new(String::new())),
            response_status_code: Arc::new(Mutex::new(0)),
            response_time: Arc::new(Mutex::new(None)),
            is_loading: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl PostmanApp {
    fn save_request(&self) {
        let request = SavedRequest {
            method: self.methods[self.method_index].to_string(),
            url: self.url.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
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
                    self.url = req.url;
                    self.body = req.body;
                    self.headers = req.headers;

                    if let Some(pos) = self.methods.iter().position(|m| *m == req.method) {
                        self.method_index = pos;
                    }
                }
            }
        }
    }

    fn send_request(&self) {
        let loading = self.is_loading.clone();
        loading.store(true, Ordering::Relaxed);

        let method = self.methods[self.method_index].to_string();
        let url = self.url.clone();
        let body = self.body.clone();
        let headers = self.headers.clone();

        let response_ref = self.response.clone();
        let status_ref = self.status.clone();
        let response_status_code = self.response_status_code.clone();
        let response_headers = self.response_headers.clone();
        let response_time = self.response_time.clone();

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
        if self.is_loading.load(Ordering::Relaxed) {
            ctx.request_repaint();
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Method")
                    .selected_text(self.methods[self.method_index])
                    .show_ui(ui, |ui| {
                        for (idx, method) in self.methods.iter().enumerate() {
                            ui.selectable_value(&mut self.method_index, idx, *method);
                        }
                    });

                ui.text_edit_singleline(&mut self.url);

                // if ui.button("Send").clicked() {
                //     self.send_request();
                // }
                let loading = self.is_loading.load(Ordering::Relaxed);
                if ui.add_enabled(!loading, egui::Button::new("Send")).clicked() {
                    self.send_request();
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
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
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

                            for (i, header) in self.headers.iter_mut().enumerate() {
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
                                self.headers.remove(idx);
                            }
                        });

                    if ui.button("+ Header").clicked() {
                        self.headers.push(Header {
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
                                egui::TextEdit::multiline(&mut self.body)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(20)
                                    .desired_width(f32::INFINITY),
                            );
                        });
                });

                // RESPONSE SIDE
                columns[1].vertical(|ui| {
                    ui.heading("Response");

                    let code = *self.response_status_code.lock().unwrap();

                    let color = match code {
                        200..=299 => egui::Color32::GREEN,

                        300..=399 => egui::Color32::YELLOW,

                        400..=499 => egui::Color32::from_rgb(255, 140, 0),

                        500..=599 => egui::Color32::RED,

                        _ => egui::Color32::GRAY,
                    };
                    ui.horizontal(|ui| {
                        ui.colored_label(color, self.status.lock().unwrap().clone());
                        let res_duration = self.response_time.lock().unwrap();
                        if res_duration.is_some() {
                            ui.label(format!("{} ms",res_duration.unwrap().as_millis()));
                        }
                    });

                    ui.separator();

                    egui::CollapsingHeader::new("Response Headers")
                        .default_open(false)
                        .show(ui, |ui| {
                            let mut headers = self.response_headers.lock().unwrap().clone();

                            ui.add(
                                egui::TextEdit::multiline(&mut headers)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(8)
                                    .interactive(false),
                            );
                        });

                    ui.separator();

                    let mut response = self.response.lock().unwrap().clone();

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

    style.spacing.item_spacing =
        egui::vec2(8.0, 8.0);

    style.visuals = egui::Visuals::dark();

    eframe::run_native(
        "Postchi",
        options,
        Box::new(|_| Ok(Box::new(PostmanApp::default()))),
    )
}
