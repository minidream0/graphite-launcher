use crate::{err, text, App, Result};
use serde_json::{json, Value};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

pub struct Entry {
    pub view: Value,
    channel: String,
    args: Vec<Value>,
}
pub fn queued(channel: &str, args: &[Value]) -> bool {
    let method = args.first().and_then(Value::as_str).unwrap_or("");
    matches!(
        (channel, method),
        ("engine", "create") | ("modrinth", "install") | ("servers", "prepare")
    ) || channel == "import-instance"
        || channel == "pack-update" && method == "update"
}
impl App {
    pub fn enqueue(self: &Arc<Self>, channel: String, args: Vec<Value>) -> Result<Value> {
        let _gate = self.filesystem_gate.lock().map_err(err)?;
        let id = format!("queue-{}", uuid::Uuid::new_v4());
        let label = match channel.as_str() {
            "engine" => "Install Minecraft",
            "import-instance" => "Import instance",
            "servers" => "Подготовка сборки сервера",
            "pack-update" => "Обновление сборки Modrinth",
            _ => match text(args.get(1).unwrap_or(&Value::Null), "kind") {
                "modpack" => "Install Modrinth pack",
                "mod" => "Install Modrinth mods",
                "shader" => "Установка шейдера",
                _ => "Установка ресурс-пака",
            },
        };
        {
            let mut queue = self.queue.lock().map_err(err)?;
            if queue
                .iter()
                .filter(|e| e.view["state"] == "queued" || e.view["state"] == "running")
                .count()
                >= 32
            {
                return Err("В очереди уже 32 задачи".into());
            }
            if queue.len() >= 128 {
                if let Some(i) = queue
                    .iter()
                    .position(|e| !matches!(text(&e.view, "state"), "queued" | "running"))
                {
                    queue.remove(i);
                }
            }
            queue.push(Entry { view: json!({"id":id,"label":label,"state":"queued","queued":true,"retryable":channel!="pack-update","status":"В очереди","instance":args.get(1).and_then(|p|p.get("instanceId")).cloned().unwrap_or(Value::Null),"created":chrono::Utc::now().to_rfc3339()}), channel, args });
        }
        if !self.queue_worker.swap(true, Ordering::SeqCst) {
            let app = self.clone();
            std::thread::spawn(move || app.work_queue());
        }
        Ok(json!({"job":id,"queued":true}))
    }
    fn work_queue(self: Arc<Self>) {
        loop {
            let next = {
                let mut queue = match self.queue.lock() {
                    Ok(q) => q,
                    Err(_) => {
                        self.queue_worker.store(false, Ordering::SeqCst);
                        return;
                    }
                };
                if let Some(e) = queue.iter_mut().find(|e| e.view["state"] == "queued") {
                    e.view["state"] = json!("running");
                    e.view["status"] = json!("Подготовка файлов…");
                    Some((
                        text(&e.view, "id").to_string(),
                        e.channel.clone(),
                        e.args.clone(),
                    ))
                } else {
                    self.queue_worker.store(false, Ordering::SeqCst);
                    None
                }
            };
            let Some((id, channel, args)) = next else {
                break;
            };
            let result = (|| -> Result<Value> {
                let result = self.dispatch(&channel, args)?;
                let native = text(&result, "job");
                if native.is_empty() {
                    return Ok(result);
                }
                self.queue_update(&id, |v| v["nativeJob"] = json!(native));
                loop {
                    std::thread::sleep(Duration::from_millis(350));
                    let state = self.native.call("state", json!({}))?;
                    let job = state["jobs"]
                        .as_array()
                        .and_then(|a| a.iter().find(|j| j["id"] == native))
                        .ok_or("Задача движка исчезла")?;
                    self.queue_update(&id, |v| {
                        for key in ["status", "current", "total", "instance"] {
                            v[key] = job[key].clone();
                        }
                    });
                    match text(job, "state") {
                        "running" => {}
                        "succeeded" => {
                            if !text(&result, "serverId").is_empty() {
                                let before =
                                    result["before"].as_array().ok_or("Нет списка сборок")?;
                                let library = self.library()?;
                                let added: Vec<_> = library
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .filter(|i| !before.contains(&i["id"]))
                                    .collect();
                                if added.len() != 1 {
                                    return Err("Сборка установлена. Выбери её в настройках сервера вручную".into());
                                }
                                let items = self.servers("list", &json!({}))?;
                                let mut server = items
                                    .as_array()
                                    .and_then(|a| a.iter().find(|s| s["id"] == result["serverId"]))
                                    .cloned()
                                    .ok_or("Сервер удалён во время установки")?;
                                server["instanceId"] = added[0]["id"].clone();
                                self.servers("save", &server)?;
                            }
                            return Ok(result);
                        }
                        "cancelled" => return Err("Загрузка отменена".into()),
                        _ => return Err(text(job, "error").to_string()),
                    }
                }
            })();
            self.queue_update(&id, |v| match result {
                Ok(value) => {
                    v["state"] = json!("succeeded");
                    v["result"] = value;
                }
                Err(error) => {
                    v["state"] = json!(if v["cancelRequested"] == true {
                        "cancelled"
                    } else {
                        "failed"
                    });
                    v["error"] = json!(error);
                }
            });
        }
    }
    fn queue_update(&self, id: &str, f: impl FnOnce(&mut Value)) {
        if let Ok(mut q) = self.queue.lock() {
            if let Some(e) = q.iter_mut().find(|e| e.view["id"] == id) {
                f(&mut e.view);
            }
        }
    }
    pub fn merge_queue(&self, state: &mut Value) -> Result<()> {
        let queue = self.queue.lock().map_err(err)?;
        if let Some(jobs) = state["jobs"].as_array_mut() {
            jobs.retain(|j| !queue.iter().any(|e| e.view["nativeJob"] == j["id"]));
            jobs.extend(queue.iter().map(|e| e.view.clone()));
        }
        Ok(())
    }
    pub fn queue_action(self: &Arc<Self>, args: &[Value]) -> Result<Value> {
        let method = args.first().and_then(Value::as_str).unwrap_or("");
        let id = args.get(1).and_then(Value::as_str).unwrap_or("");
        let (channel, original) = {
            let mut queue = self.queue.lock().map_err(err)?;
            let e = queue
                .iter_mut()
                .find(|e| e.view["id"] == id)
                .ok_or("Задача не найдена")?;
            if method == "cancel" {
                if e.view["state"] == "queued" {
                    e.view["state"] = json!("cancelled");
                    return Ok(Value::Null);
                }
                if e.view["state"] != "running" {
                    return Err("Задача уже завершена".into());
                }
                let native = text(&e.view, "nativeJob").to_string();
                if native.is_empty() {
                    return Err(
                        "Идёт подготовка или запись файлов. Дождись завершения этапа".into(),
                    );
                }
                e.view["cancelRequested"] = json!(true);
                drop(queue);
                return self.native.call("cancel", json!({"id":native}));
            }
            if method != "retry" || !matches!(text(&e.view, "state"), "failed" | "cancelled") {
                return Err("Повтор доступен после ошибки или отмены".into());
            }
            if e.view["retryable"] == false {
                return Err("Заново проверь обновление сборки перед повтором".into());
            }
            (e.channel.clone(), e.args.clone())
        };
        self.enqueue(channel, original)
    }
}
