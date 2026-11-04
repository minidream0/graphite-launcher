#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod auth;
mod catalog;
mod engine;
mod icons;
mod offline;
mod porthole;
mod queue;
mod servers;
mod storage;
#[cfg(test)]
mod tests;
mod tools;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
pub type Result<T> = std::result::Result<T, String>;
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}
pub struct App {
    pub data: PathBuf,
    pub native: engine::Native,
    pub http: reqwest::blocking::Client,
    pub auth_http: reqwest::blocking::Client,
    pub cache: Mutex<HashMap<String, (Instant, Value)>>,
    pub plans: Mutex<HashMap<String, Value>>,
    pub imports: Mutex<HashMap<String, PathBuf>>,
    pub drops: Mutex<HashSet<PathBuf>>,
    pub busy: Mutex<HashSet<String>>,
    pub servers_lock: Mutex<()>,
    pub porthole: porthole::Porthole,
    pub session: Mutex<Option<auth::Session>>,
    pub selected: AtomicBool,
    pub offline: Mutex<offline::Profiles>,
    pub activity: Mutex<()>,
    pub filesystem_gate: Mutex<()>,
    pub queue: Mutex<Vec<queue::Entry>>,
    pub queue_worker: AtomicBool,
}
impl App {
    fn new() -> Result<Self> {
        let root = std::env::current_exe()
            .map_err(err)?
            .parent()
            .ok_or("Нет папки приложения")?
            .to_path_buf();
        let data = std::env::var_os("GRAPHITE_DATA")
            .map(PathBuf::from)
            .unwrap_or(root.join("data"));
        storage::directory(&data)?;
        icons::install(&data)?;
        let binary = std::env::var_os("GRAPHITE_BACKEND")
            .map(PathBuf::from)
            .unwrap_or(root.join("engine/prismlauncher.exe"));
        storage::regular(&binary)?;
        let config = data.join("prismlauncher.cfg");
        if !config.exists() {
            storage::write_new(&config,b"[General]\nApplicationTheme=dark\nIconTheme=flat_white\nLanguage=ru\nAutomaticJavaDownload=true\nAutomaticJavaSwitch=true\nUserAskedAboutAutomaticJavaDownload=true\n")?;
        }
        let translations = data.join("translations");
        storage::directory(&translations)?;
        for name in ["mmc_ru.qm", "index_v2.json"] {
            let from = binary.parent().unwrap().join("translations").join(name);
            let to = translations.join(name);
            if from.exists() && !to.exists() {
                storage::copy_new(&from, &to)?;
            }
        }
        let client = |timeout| {
            reqwest::blocking::Client::builder()
                .user_agent(concat!(
                    "Graphite/",
                    env!("CARGO_PKG_VERSION"),
                    " (PrismLauncher desktop fork)"
                ))
                .timeout(Duration::from_secs(timeout))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(err)
        };
        let offline = offline::load(&data)?;
        Ok(Self {
            native: engine::Native::new(binary, data.clone())?,
            porthole: porthole::Porthole::new(&data),
            data,
            http: client(20)?,
            auth_http: client(15)?,
            cache: Mutex::new(HashMap::new()),
            plans: Mutex::new(HashMap::new()),
            imports: Mutex::new(HashMap::new()),
            drops: Mutex::new(HashSet::new()),
            busy: Mutex::new(HashSet::new()),
            servers_lock: Mutex::new(()),
            session: Mutex::new(None),
            selected: AtomicBool::new(false),
            offline: Mutex::new(offline),
            activity: Mutex::new(()),
            filesystem_gate: Mutex::new(()),
            queue: Mutex::new(Vec::new()),
            queue_worker: AtomicBool::new(false),
        })
    }
    pub fn check_busy(&self, id: &str) -> Result<()> {
        if self.busy.lock().map_err(err)?.contains(id) {
            Err("Сборка изменяется. Дождись завершения операции".into())
        } else {
            Ok(())
        }
    }
    pub fn with_instance<T>(&self, id: &str, operation: impl FnOnce() -> Result<T>) -> Result<T> {
        self.instance(id)?;
        {
            let mut busy = self.busy.lock().map_err(err)?;
            if !busy.insert(id.into()) {
                return Err("Сборка изменяется. Дождись завершения операции".into());
            }
        }
        struct Guard<'a> {
            app: &'a App,
            id: String,
        }
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Ok(mut busy) = self.app.busy.lock() {
                    busy.remove(&self.id);
                }
            }
        }
        let _guard = Guard {
            app: self,
            id: id.into(),
        };
        let state = self.native.call("state", json!({}))?;
        if state["jobs"].as_array().is_some_and(|a| {
            a.iter()
                .any(|j| j["state"] == "running" && j["instance"] == id)
        }) {
            return Err("Закрой игру и дождись завершения задач сборки".into());
        }
        operation()
    }
    fn register_import(&self, path: PathBuf) -> Result<Value> {
        storage::regular(&path)?;
        let name = path
            .file_name()
            .ok_or("Нет имени файла")?
            .to_string_lossy()
            .to_string();
        if !name.to_lowercase().ends_with(".zip") && !name.to_lowercase().ends_with(".mrpack") {
            return Err("Выбери .mrpack или ZIP".into());
        }
        let token = uuid::Uuid::new_v4().to_string();
        let mut files = self.imports.lock().map_err(err)?;
        if files.len() >= 100 {
            return Err(
                "Выбрано слишком много архивов. Перезапусти лаунчер после завершения очереди"
                    .into(),
            );
        }
        files.insert(token.clone(), path);
        Ok(json!({"token":token,"name":name}))
    }
    fn dispatch(&self, channel: &str, args: Vec<Value>) -> Result<Value> {
        let a = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        let s = |i: usize| args.get(i).and_then(Value::as_str).unwrap_or("");
        match channel {
            "porthole" => self.porthole(s(0), &a(1)),
            "tools" => self.tools(s(0), &a(1)),
            "library" => self.library(),
            "versions" => self.versions(
                if s(0).is_empty() {
                    "net.minecraft"
                } else {
                    s(0)
                },
                s(1),
                a(2) == true,
            ),
            "engine" => {
                let method = s(0);
                let params = if a(1).is_null() { json!({}) } else { a(1) };
                if ![
                    "state",
                    "settings.get",
                    "settings.set",
                    "icons.list",
                    "instance.get",
                    "instance.set",
                    "instance.log",
                    "instance.manage",
                    "create",
                    "launch",
                    "login",
                    "cancel",
                    "accounts.default",
                    "accounts.offline.add",
                    "accounts.remove",
                ]
                .contains(&method)
                    || !params.is_object()
                    || params.get("twitchland").is_some()
                    || params.get("offline").is_some()
                    || params.to_string().len() > 20000
                {
                    return Err("Недопустимая команда".into());
                }
                if method.starts_with("instance.") || method == "launch" {
                    self.instance(text(&params, "id"))?;
                }
                if matches!(method, "launch" | "instance.set" | "instance.manage") {
                    self.check_busy(text(&params, "id"))?;
                }
                self.engine(method, params)
            }
            "twitchland" => self.twitchland(s(0), &a(1)),
            "modrinth" => {
                let p = a(1);
                if p.to_string().len() > 12000 {
                    return Err("Слишком большой запрос Modrinth".into());
                }
                if s(0) == "install" && !text(&p, "instanceId").is_empty() {
                    self.check_busy(text(&p, "instanceId"))?;
                }
                self.modrinth(s(0), &p)
            }
            "servers" => {
                let p = if a(1).is_null() { json!({}) } else { a(1) };
                if !p.is_object() || p.to_string().len() > 3000 {
                    return Err("Недопустимый запрос".into());
                }
                self.servers(s(0), &p)
            }
            "installed-mods" => self.file_action(s(0), s(1), "mod", s(2)),
            "resources" => {
                let method = s(0);
                let id = s(1);
                let kind = s(2);
                if method == "pick" {
                    let files = rfd::FileDialog::new()
                        .set_title("Выбери ZIP")
                        .add_filter("ZIP", &["zip"])
                        .pick_files();
                    match files {
                        Some(files) => self.install_local(id, kind, &files),
                        None => self.files(id, kind),
                    }
                } else {
                    self.file_action(method, id, kind, s(3))
                }
            }
            "pack-update" => match s(0) {
                "preview" => self.pack_preview(s(1)),
                "update" => self.pack_update(s(1), s(2)),
                _ => Err("Неизвестное действие".into()),
            },
            "backups" => {
                let id = s(1);
                match s(0) {
                    "list" => self.backups(id),
                    "create" => self.with_instance(id, || {
                        self.backup(id)?;
                        self.backups(id)
                    }),
                    "folder" => {
                        self.instance(id)?;
                        let dir = self.data.join("backups");
                        storage::directory(&dir)?;
                        open::that(dir).map_err(err)?;
                        Ok(Value::Null)
                    }
                    _ => Err("Неизвестное действие".into()),
                }
            }
            "pick-import" => match rfd::FileDialog::new()
                .set_title("Выбери сборку")
                .add_filter("Сборки Minecraft", &["mrpack", "zip"])
                .pick_file()
            {
                Some(file) => self.register_import(file),
                None => Ok(Value::Null),
            },
            "import-instance" => {
                if a(2) != true {
                    return Err("Подтверди доверие к архиву".into());
                }
                let path = self
                    .imports
                    .lock()
                    .map_err(err)?
                    .get(s(0))
                    .cloned()
                    .ok_or("Сначала выбери файл")?;
                self.engine(
                    "instance.import",
                    json!({"path":path,"name":a(1),"trusted":true}),
                )
            }
            "drop-files" => {
                let values = a(0);
                let files = values.as_array().ok_or("Нет файлов")?;
                if files.is_empty() || files.len() > 64 {
                    return Err("Перетащи один архив или до 64 файлов".into());
                }
                let paths: Vec<_> = files
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map(PathBuf::from)
                            .ok_or("Некорректный файл".to_string())
                    })
                    .collect::<Result<_>>()?;
                {
                    let mut approved = self.drops.lock().map_err(err)?;
                    if paths.iter().any(|p| !approved.contains(p)) {
                        return Err("Файлы должны быть выбраны или перетащены в окно".into());
                    }
                    for p in &paths {
                        approved.remove(p);
                    }
                }
                let kind = s(2);
                if kind == "resourcepack" || kind == "shader" {
                    self.install_local(s(1), kind, &paths)?;
                    return Ok(json!({"kind":"resources","resourceKind":kind,"id":a(1)}));
                }
                if paths.len() == 1
                    && paths[0].extension().is_some_and(|e| {
                        ["zip", "mrpack"].contains(&e.to_string_lossy().to_lowercase().as_str())
                    })
                {
                    return Ok(
                        json!({"kind":"import","file":self.register_import(paths[0].clone())?}),
                    );
                }
                let id = s(1);
                if id.is_empty() {
                    return Err("Сначала выбери сборку для модов".into());
                }
                self.install_local(id, "mod", &paths)?;
                let lib = self.library()?;
                let item = lib
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|x| x["id"] == id)
                    .ok_or("Сборка не найдена")?;
                Ok(json!({"kind":"mods","count":paths.len(),"name":item["name"]}))
            }
            "action" => {
                let path = match s(0) {
                    "folder" => self.instance(s(1))?,
                    "gameFolder" => self.game(s(1))?,
                    "dataFolder" => self.data.clone(),
                    _ => return Err("Неизвестное действие".into()),
                };
                open::that(path).map_err(err)?;
                Ok(Value::Null)
            }
            "choose-java" => Ok(rfd::FileDialog::new()
                .set_title("Java")
                .add_filter("Java", &["exe"])
                .pick_file()
                .map(|v| json!(v))
                .unwrap_or(Value::Null)),
            "copy-log" => {
                self.instance(s(0))?;
                let log = self.engine("instance.log", json!({"id":s(0)}))?;
                clipboard(text(&log, "text"))?;
                Ok(Value::Null)
            }
            "login-action" => {
                let state = self.engine("state", json!({}))?;
                let job = state["jobs"]
                    .as_array()
                    .and_then(|a| {
                        a.iter()
                            .find(|j| j["id"] == s(1) && j["state"] == "running")
                    })
                    .ok_or("Код входа истёк")?;
                let auth = &job["auth"];
                if auth["expires"].as_f64().unwrap_or(0.)
                    < (chrono::Utc::now().timestamp_millis() as f64)
                {
                    return Err("Код входа истёк".into());
                }
                match s(0) {
                    "copy" => clipboard(text(auth, "code"))?,
                    "copy-url" | "open" => {
                        let url = reqwest::Url::parse(text(auth, "url")).map_err(err)?;
                        if url.scheme() != "https"
                            || ![
                                "microsoft.com",
                                "www.microsoft.com",
                                "login.microsoftonline.com",
                                "login.live.com",
                                "microsoftonline.com",
                            ]
                            .contains(&url.host_str().unwrap_or(""))
                            || !url.username().is_empty()
                            || url.password().is_some()
                        {
                            return Err("Недопустимый адрес входа".into());
                        }
                        if s(0) == "open" {
                            open::that(url.as_str()).map_err(err)?
                        } else {
                            clipboard(url.as_str())?
                        }
                    }
                    _ => return Err("Неизвестное действие".into()),
                }
                Ok(Value::Null)
            }
            _ => Err("Неизвестная команда".into()),
        }
    }
}
fn clipboard(text: &str) -> Result<()> {
    arboard::Clipboard::new()
        .map_err(err)?
        .set_text(text)
        .map_err(err)
}
fn activate_window(window: &tauri::WebviewWindow) -> Result<()> {
    window.unminimize().map_err(err)?;
    window.show().map_err(err)?;
    window.set_focus().map_err(err)
}
#[tauri::command]
async fn rpc(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<App>>,
    channel: String,
    args: Vec<Value>,
) -> Result<Value> {
    if window.label() != "main"
        || args.len() > 8
        || serde_json::to_vec(&args).map_err(err)?.len() > 100000
    {
        return Err("Недопустимый запрос".into());
    }
    if channel == "window" {
        match args.first().and_then(Value::as_str).unwrap_or("") {
            "ready" => activate_window(&window)?,
            "minimize" => window.minimize().map_err(err)?,
            "maximize" => {
                if window.is_maximized().map_err(err)? {
                    window.unmaximize().map_err(err)?
                } else {
                    window.maximize().map_err(err)?
                }
            }
            "drag" => window.start_dragging().map_err(err)?,
            "close" => window.close().map_err(err)?,
            _ => return Err("Неизвестное действие окна".into()),
        }
        return Ok(Value::Null);
    }
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if channel == "queue" {
            return app.queue_action(&args);
        }
        if queue::queued(&channel, &args) {
            return app.enqueue(channel, args);
        }
        app.dispatch(&channel, args)
    })
    .await
    .map_err(err)?
}
fn main() {
    let app = match App::new() {
        Ok(app) => Arc::new(app),
        Err(error) => {
            rfd::MessageDialog::new()
                .set_title("Graphite")
                .set_description(&error)
                .set_level(rfd::MessageLevel::Error)
                .show();
            return;
        }
    };
    let state = app.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = activate_window(&window);
            }
        }))
        .manage(app)
        .invoke_handler(tauri::generate_handler![rpc])
        .on_webview_event(|window, event| {
            if let tauri::WebviewEvent::DragDrop(event) = event {
                let state = window.state::<Arc<App>>();
                if state
                    .queue_worker
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    let _ = window.emit(
                        "graphite-error",
                        "Дождись завершения очереди или отмени задачи",
                    );
                    return;
                }
                let payload = match event {
                    tauri::DragDropEvent::Enter { .. } => json!({"type":"enter"}),
                    tauri::DragDropEvent::Over { .. } => json!({"type":"over"}),
                    tauri::DragDropEvent::Leave => json!({"type":"leave"}),
                    tauri::DragDropEvent::Drop { paths, .. } => {
                        if paths.len() > 64 {
                            let _ = window.emit("graphite-error", "Перетащи до 64 файлов");
                            return;
                        }
                        if let Ok(mut approved) = state.drops.lock() {
                            approved.clear();
                            approved.extend(paths.iter().cloned());
                        }
                        json!({"type":"drop","paths":paths})
                    }
                    _ => return,
                };
                let _ = window.emit("graphite-file-drag", payload);
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let state = window.state::<Arc<App>>();
                if state
                    .queue_worker
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    let _ = window.emit(
                        "graphite-error",
                        "Дождись завершения очереди или отмени задачи",
                    );
                    return;
                }
                if state.busy.lock().map(|b| !b.is_empty()).unwrap_or(true) {
                    let _ = window.emit("graphite-error", "Дождись завершения операций с файлами");
                    return;
                }
                let state = state.inner().clone();
                let handle = window.app_handle().clone();
                std::thread::spawn(move || {
                    state.porthole.stop();
                    state.native.close();
                    handle.exit(0);
                });
            }
        })
        .setup(move |app| {
            let window =
                tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                    .on_navigation(|url| {
                        url.scheme() == "tauri"
                            || url.host_str() == Some("tauri.localhost")
                            || url.as_str() == "about:blank"
                    })
                    .icon(
                        app.default_window_icon()
                            .cloned()
                            .ok_or("Missing Graphite window icon")?,
                    )?
                    .build()?;
            if let Some(monitor) = window.current_monitor()? {
                let size = monitor.size();
                let scale = monitor.scale_factor();
                let width = (size.width as f64 / scale - 32.).min(1440.).max(860.);
                let height = (size.height as f64 / scale - 64.).min(940.).max(620.);
                window.set_size(tauri::LogicalSize::new(width, height))?;
                window.center()?;
            }
            let _ = state;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Graphite");
}
