use crate::{err, storage, text, App, Result};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::Path,
};

fn load(path: &Path, fallback: Value) -> Result<Value> {
    if !path.exists() {
        return Ok(fallback);
    }
    serde_json::from_str(&storage::read(path, 8 * 1024 * 1024)?).map_err(err)
}
fn save(path: &Path, value: &Value) -> Result<()> {
    let temp = crate::catalog::Temporary(
        path.with_file_name(format!(".graphite-{}.tmp", uuid::Uuid::new_v4())),
    );
    storage::write_new(&temp.0, &serde_json::to_vec_pretty(value).map_err(err)?)?;
    fs::rename(&temp.0, path).map_err(err)
}
fn issue(level: &str, title: impl Into<String>, detail: impl Into<String>) -> Value {
    json!({"level":level,"title":title.into(),"detail":detail.into()})
}
// Unknown version syntax stays unknown instead of falsely rejecting a working mod.
fn version_matches(version: &str, rule: &str) -> Option<bool> {
    fn numbers(v: &str) -> Option<Vec<u64>> {
        v.split('+')
            .next()?
            .split('.')
            .map(|x| x.parse().ok())
            .collect()
    }
    if rule == "*" || rule.is_empty() {
        return Some(true);
    }
    if rule.contains("||") {
        let r: Vec<_> = rule
            .split("||")
            .map(|r| version_matches(version, r.trim()))
            .collect();
        return if r.contains(&Some(true)) {
            Some(true)
        } else if r.contains(&None) {
            None
        } else {
            Some(false)
        };
    }
    if rule.split_whitespace().count() > 1 {
        let r: Vec<_> = rule
            .split_whitespace()
            .map(|r| version_matches(version, r))
            .collect();
        return if r.contains(&Some(false)) {
            Some(false)
        } else if r.contains(&None) {
            None
        } else {
            Some(true)
        };
    }
    let (op, v) = [">=", "<=", ">", "<", "=", "~", "^"]
        .into_iter()
        .find_map(|op| rule.strip_prefix(op).map(|v| (op, v)))
        .unwrap_or(("=", rule));
    let mut actual = numbers(version)?;
    if v.ends_with(".*") || v.ends_with(".x") {
        if op != "=" {
            return None;
        }
        return Some(
            version == &v[..v.len() - 2] || version.starts_with(&format!("{}.", &v[..v.len() - 2])),
        );
    }
    let mut wanted = numbers(v)?;
    let n = actual.len().max(wanted.len());
    actual.resize(n, 0);
    wanted.resize(n, 0);
    Some(match op {
        ">=" => actual >= wanted,
        "<=" => actual <= wanted,
        ">" => actual > wanted,
        "<" => actual < wanted,
        "~" => {
            actual >= wanted
                && actual.first() == wanted.first()
                && (v.split('.').count() < 2 || actual.get(1) == wanted.get(1))
        }
        "^" => {
            let pivot = wanted
                .iter()
                .position(|x| *x != 0)
                .unwrap_or(v.split('.').count() - 1);
            actual >= wanted && actual[..=pivot] == wanted[..=pivot]
        }
        _ => actual == wanted,
    })
}
fn constraint(version: &str, rule: &Value) -> Option<bool> {
    if let Some(s) = rule.as_str() {
        return version_matches(version, s);
    }
    if let Some(a) = rule.as_array() {
        let r: Vec<_> = a.iter().map(|r| constraint(version, r)).collect();
        return if r.contains(&Some(true)) {
            Some(true)
        } else if r.contains(&None) {
            None
        } else {
            Some(false)
        };
    }
    None
}
fn jar(path: &Path) -> Result<Vec<Value>> {
    fn inspect(
        bytes: impl Read + std::io::Seek,
        depth: u8,
        budget: &mut u64,
    ) -> Result<Vec<Value>> {
        let mut zip = zip::ZipArchive::new(bytes).map_err(err)?;
        let mut result = Vec::new();
        for name in ["fabric.mod.json", "quilt.mod.json"] {
            if !result.is_empty() {
                break;
            }
            if let Ok(mut f) = zip.by_name(name) {
                if f.size() > 1024 * 1024 {
                    return Err("Метаданные мода слишком большие".into());
                }
                let mut bytes = String::new();
                f.read_to_string(&mut bytes).map_err(err)?;
                let v: Value = serde_json::from_str(&bytes).map_err(err)?;
                if !v.is_object() {
                    return Err("Метаданные мода должны быть JSON-объектом".into());
                }
                if name == "fabric.mod.json" {
                    let mut v = v;
                    v["_nested"] = json!(depth > 0);
                    result.push(v)
                } else {
                    let q = &v["quilt_loader"];
                    result.push(json!({"id":q["id"],"version":q["version"],"name":q["metadata"]["name"],"quilt":true,"_nested":depth>0}))
                }
            }
        }
        let nested: Vec<String> = result
            .iter()
            .flat_map(|v| {
                v["jars"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|j| j["file"].as_str().map(str::to_string))
            })
            .collect();
        if depth < 3 {
            for name in nested {
                let mut f = zip.by_name(&name).map_err(err)?;
                if f.size() > *budget {
                    return Err("Вложенные моды слишком большие".into());
                }
                *budget -= f.size();
                let mut data = Vec::new();
                f.read_to_end(&mut data).map_err(err)?;
                result.extend(inspect(std::io::Cursor::new(data), depth + 1, budget)?);
            }
        }
        Ok(result)
    }
    storage::regular(path)?;
    inspect(
        fs::File::open(path).map_err(err)?,
        0,
        &mut (64 * 1024 * 1024),
    )
}
fn tree_size(path: &Path, depth: usize, count: &mut usize) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    if depth > 64 || *count > 250000 {
        return Err("Слишком много файлов для одного сканирования".into());
    }
    *count += 1;
    let m = fs::symlink_metadata(path).map_err(err)?;
    if storage::linked(&m) {
        return Ok(0);
    }
    if m.is_file() {
        return Ok(m.len());
    }
    let mut size = 0;
    for e in fs::read_dir(path).map_err(err)? {
        size += tree_size(&e.map_err(err)?.path(), depth + 1, count)?
    }
    Ok(size)
}
impl App {
    pub fn preflight(&self, id: &str) -> Result<Value> {
        let root = self.instance(id)?;
        let pack = load(&root.join("mmc-pack.json"), json!({}))?;
        let components = pack["components"]
            .as_array()
            .ok_or("Повреждён mmc-pack.json")?;
        let mut installed: HashMap<String, String> = HashMap::new();
        let mut issues = Vec::new();
        for c in components {
            let name = match text(c, "uid") {
                "net.minecraft" => "minecraft",
                "net.fabricmc.fabric-loader" => "fabricloader",
                "org.quiltmc.quilt-loader" => "quilt_loader",
                _ => continue,
            };
            installed.insert(name.into(), text(c, "version").into());
        }
        if !installed.contains_key("minecraft") {
            issues.push(issue(
                "error",
                "Не указана версия Minecraft",
                "Выбери версию в настройках сборки",
            ))
        }
        let settings = load_ini(&root.join("instance.cfg"))?;
        let global = load_ini(&self.data.join("prismlauncher.cfg"))?;
        let j = if settings
            .get("OverrideJavaLocation")
            .is_some_and(|v| v == "true")
        {
            &settings
        } else {
            &global
        };
        if let Some(path) = j.get("JavaPath").filter(|p| !p.is_empty()) {
            if !Path::new(path).is_file()
                && !global
                    .get("AutomaticJavaSwitch")
                    .is_some_and(|v| v == "true")
            {
                issues.push(issue(
                    "warning",
                    "Путь Java недоступен",
                    "Включи автоматический выбор Java или укажи установленную Java",
                ))
            }
        }
        let game = self.game(id)?;
        let dependency_override = game.join("config/fabric_loader_dependencies.json").exists();
        let diagnostic = root.join(".graphite-diagnostic.json").exists();
        let mods = game.join("mods");
        let mut metadata = Vec::new();
        let mut metadata_bytes = 0usize;
        let mut scanned = 0;
        let mut unknown = 0;
        let mut top_ids = HashSet::new();
        if mods.exists() {
            storage::directory(&mods)?;
            for entry in fs::read_dir(&mods).map_err(err)? {
                let p = entry.map_err(err)?.path();
                if !p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
                    .ends_with(".jar")
                {
                    continue;
                }
                scanned += 1;
                if scanned > 1500 {
                    return Err("В сборке больше 1500 JAR-файлов".into());
                }
                let file = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                match jar(&p) {
                    Ok(rows) => {
                        if rows.is_empty() {
                            unknown += 1
                        }
                        for v in rows {
                            metadata_bytes += v.to_string().len();
                            if metadata_bytes > 128 * 1024 * 1024 {
                                return Err("Метаданные модов превышают 128 МБ".into());
                            }
                            if v["environment"] == "server" {
                                continue;
                            }
                            let key = text(&v, "id");
                            if key.is_empty() {
                                unknown += 1;
                                continue;
                            }
                            if v["_nested"] != true && !top_ids.insert(key.to_string()) {
                                issues.push(issue(
                                    "error",
                                    format!("Повторяется мод {key}"),
                                    file.clone(),
                                ))
                            }
                            if v["_nested"] == true && installed.contains_key(key) {
                                continue;
                            }
                            installed.insert(key.into(), text(&v, "version").into());
                            if let Some(provides) = v["provides"].as_array() {
                                for alias in provides.iter().filter_map(Value::as_str) {
                                    installed
                                        .entry(alias.into())
                                        .or_insert_with(|| text(&v, "version").into());
                                }
                            }
                            metadata.push((file.clone(), v));
                        }
                    }
                    Err(e) => issues.push(issue("error", format!("Не читается {file}"), e)),
                }
            }
        }
        for (file, v) in &metadata {
            if let Some(deps) = v["depends"].as_object() {
                for (dep, rule) in deps {
                    if dep == "java" {
                        continue;
                    }
                    if let Some(version) = installed.get(dep) {
                        match constraint(version, rule) {
                            Some(false) => issues.push(issue(
                                "error",
                                format!("{} требует {dep} {rule}", text(v, "id")),
                                format!("Установлено: {version} · {file}"),
                            )),
                            None => issues.push(issue(
                                "warning",
                                format!("Проверь версию зависимости {dep}"),
                                format!("{file} · условие {rule}"),
                            )),
                            _ => {}
                        }
                    } else {
                        issues.push(issue(
                            if unknown > 0 { "warning" } else { "error" },
                            format!("{}: не найдена зависимость {dep}", text(v, "id")),
                            format!("Требуется {rule} · {file}"),
                        ))
                    }
                }
            }
            for key in ["breaks", "conflicts"] {
                if let Some(deps) = v[key].as_object() {
                    for (dep, rule) in deps {
                        if installed
                            .get(dep)
                            .is_some_and(|version| constraint(version, rule) == Some(true))
                        {
                            issues.push(issue(
                                if key == "breaks" { "error" } else { "warning" },
                                format!("{} несовместим с {dep}", text(v, "id")),
                                file.clone(),
                            ))
                        }
                    }
                }
            }
        }
        if unknown > 0 || installed.contains_key("quilt_loader") {
            issues.push(issue("warning","Часть зависимостей требует проверки при запуске",format!("JAR без Fabric-метаданных: {unknown}. Проверка Forge/NeoForge/Quilt ограничена структурой архивов.")))
        }
        if dependency_override || diagnostic {
            for i in &mut issues {
                if text(i, "title").contains("требует")
                    || text(i, "title").contains("зависимость")
                    || text(i, "title").contains("несовместим")
                {
                    i["level"] = json!("warning")
                }
            }
            issues.push(issue(
                "warning",
                "Зависимости окончательно проверит загрузчик",
                if diagnostic {
                    "Диагностическая копия: часть модов намеренно отключена"
                } else {
                    "Найден fabric_loader_dependencies.json с пользовательскими правилами"
                },
            ));
        }
        let errors = issues.iter().filter(|v| v["level"] == "error").count();
        Ok(
            json!({"id":id,"scanned":scanned,"errors":errors,"issues":issues,"checked":chrono::Utc::now().to_rfc3339()}),
        )
    }
    pub fn record_launch(&self, p: &Value, result: &Value) -> Result<()> {
        let _lock = self.activity.lock().map_err(err)?;
        let file = self.data.join("graphite-history.json");
        let mut history = load(&file, json!([]))?;
        let a = history.as_array_mut().ok_or("Повреждена история")?;
        let id = text(p, "id");
        let library = self.library()?;
        let name = library
            .as_array()
            .and_then(|a| a.iter().find(|i| i["id"] == id))
            .map(|i| text(i, "name"))
            .unwrap_or(id);
        a.insert(0,json!({"id":uuid::Uuid::new_v4().to_string(),"job":result["job"],"instance":id,"name":name,"server":p["server"],"started":chrono::Utc::now().timestamp_millis(),"state":"running"}));
        a.truncate(300);
        save(&file, &history)
    }
    pub fn reconcile_history(&self, state: &Value) -> Result<()> {
        let _lock = self.activity.lock().map_err(err)?;
        let file = self.data.join("graphite-history.json");
        if !file.exists() {
            return Ok(());
        }
        let mut history = load(&file, json!([]))?;
        let mut changed = false;
        for item in history.as_array_mut().ok_or("Повреждена история")? {
            if item["state"] != "running" {
                continue;
            }
            let job = state["jobs"]
                .as_array()
                .and_then(|a| a.iter().find(|j| j["id"] == item["job"]));
            if job.is_some_and(|j| j["state"] == "running") {
                continue;
            }
            item["state"] = job
                .map(|j| j["state"].clone())
                .unwrap_or(json!("interrupted"));
            item["error"] = job.map(|j| j["error"].clone()).unwrap_or(Value::Null);
            item["ended"] = json!(chrono::Utc::now().timestamp_millis());
            changed = true;
        }
        if changed {
            save(&file, &history)?
        }
        Ok(())
    }
    pub fn tools(&self, method: &str, p: &Value) -> Result<Value> {
        let id = text(p, "id");
        match method {
            "appearance.get" => {
                let value = load(
                    &self.data.join("graphite-appearance.json"),
                    json!({"theme":"graphite"}),
                )?;
                Ok(json!({"theme":if value["theme"]=="smoky" {"smoky"} else {"graphite"}}))
            }
            "appearance.set" => {
                let theme = text(p, "theme");
                if !["graphite", "smoky"].contains(&theme) {
                    return Err("Неизвестная тема".into());
                }
                let _lock = self.activity.lock().map_err(err)?;
                let value = json!({"theme":theme});
                save(&self.data.join("graphite-appearance.json"), &value)?;
                Ok(value)
            }
            "presentation.version" => {
                let _lock = self.activity.lock().map_err(err)?;
                let file = self.data.join("graphite-seen-version.json");
                let previous = load(&file, Value::Null)?;
                let current = json!(env!("CARGO_PKG_VERSION"));
                save(&file, &current)?;
                Ok(if previous.is_string() && previous != current {
                    previous
                } else {
                    Value::Null
                })
            }
            "preflight" => self.preflight(id),
            "history" => {
                self.engine("state", json!({}))?;
                load(&self.data.join("graphite-history.json"), json!([]))
            }
            "diagnostic.get" => load(
                &self.instance(id)?.join(".graphite-diagnostic.json"),
                json!(null),
            ),
            "diagnostic.create" => self.with_instance(id, || self.diagnostic_copy(id)),
            "diagnostic.step" => {
                self.with_instance(id, || self.diagnostic_step(id, text(p, "answer")))
            }
            "disk" => self.disk_report(),
            "disk.clean" => self.disk_clean(p),
            _ => Err("Неизвестное действие".into()),
        }
    }
    fn diagnostic_copy(&self, id: &str) -> Result<Value> {
        let source = self.instance(id)?;
        let game = self.game(id)?;
        let key = format!("diagnostic-{}", uuid::Uuid::new_v4().simple());
        let dest = self.data.join("instances").join(&key);
        storage::directory(&dest)?;
        let result = (|| -> Result<Value> {
            for e in fs::read_dir(&source).map_err(err)? {
                let e = e.map_err(err)?;
                let name = e.file_name();
                if ["minecraft", ".minecraft", ".graphite-diagnostic.json"]
                    .contains(&name.to_string_lossy().as_ref())
                {
                    continue;
                }
                storage::copy_tree(&e.path(), &dest.join(name), false)?
            }
            let target = dest.join("minecraft");
            storage::directory(&target)?;
            for e in fs::read_dir(&game).map_err(err)? {
                let e = e.map_err(err)?;
                let name = e.file_name();
                if [
                    "saves",
                    "logs",
                    "crash-reports",
                    "screenshots",
                    "server-resource-packs",
                ]
                .contains(&name.to_string_lossy().as_ref())
                {
                    continue;
                }
                storage::copy_tree(&e.path(), &target.join(name), false)?
            }
            let cfg = dest.join("instance.cfg");
            let content = storage::read(&cfg, 1024 * 1024)?;
            let content = content
                .lines()
                .filter(|l| !l.starts_with("name="))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(
                &cfg,
                format!("{content}\n[General]\nname=Диагностика · {id}\n"),
            )
            .map_err(err)?;
            let mods = self.files(&key, "mod")?;
            let candidates: Vec<_> = mods
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["enabled"] == true)
                .map(|m| m["file"].clone())
                .collect();
            let data = json!({"source":id,"candidates":candidates,"disabled":[],"round":0,"phase":"baseline"});
            save(&dest.join(".graphite-diagnostic.json"), &data)?;
            Ok(json!({"id":key,"diagnostic":data}))
        })();
        if result.is_err() {
            let _ = trash::delete(&dest);
        }
        result
    }
    fn diagnostic_step(&self, id: &str, answer: &str) -> Result<Value> {
        let root = self.instance(id)?;
        let file = root.join(".graphite-diagnostic.json");
        let mut d = load(&file, json!(null))?;
        if d.is_null() {
            return Err("Это не диагностическая копия".into());
        }
        let mut candidates: Vec<String> =
            serde_json::from_value(d["candidates"].clone()).map_err(err)?;
        let disabled: Vec<String> = serde_json::from_value(d["disabled"].clone()).map_err(err)?;
        if !matches!(answer, "baseline" | "same" | "fixed" | "reset") {
            return Err("Выбери результат проверки".into());
        }
        let dir = self.resource_dir(id, "mod")?;
        if answer == "reset" {
            switch_mods(&dir, &disabled, &[])?;
            d["disabled"] = json!([]);
            d["phase"] = json!("stopped");
            if let Err(e) = save(&file, &d) {
                let _ = switch_mods(&dir, &[], &disabled);
                return Err(e);
            }
            return Ok(d);
        }
        if d["phase"] == "baseline" && answer != "baseline" {
            return Err("Сначала воспроизведи исходный сбой".into());
        }
        if d["phase"] != "baseline" && answer == "baseline"
            || !["baseline", "testing"].contains(&text(&d, "phase"))
        {
            return Err("Этот поиск уже завершён или остановлен".into());
        }
        if answer == "fixed" {
            candidates.retain(|n| disabled.contains(n))
        }
        if answer == "same" {
            candidates.retain(|n| !disabled.contains(n))
        }
        let test: Vec<_> = if candidates.len() > 1 {
            candidates[..candidates.len().div_ceil(2)].to_vec()
        } else {
            Vec::new()
        };
        switch_mods(&dir, &disabled, &test)?;
        d["candidates"] = json!(candidates);
        d["disabled"] = json!(test);
        d["round"] = json!(d["round"].as_u64().unwrap_or(0) + 1);
        d["phase"] = json!(if candidates.len() > 1 {
            "testing"
        } else {
            "result"
        });
        if let Err(e) = save(&file, &d) {
            let _ = switch_mods(&dir, &test, &disabled);
            return Err(e);
        }
        Ok(d)
    }
    fn disk_report(&self) -> Result<Value> {
        let mut rows = Vec::new();
        let mut count = 0;
        for i in self.library()?.as_array().ok_or("Нет библиотеки")? {
            let id = text(i, "id");
            let root = self.instance(id)?;
            let game = self.game(id)?;
            let total = tree_size(&root, 0, &mut count)?;
            let worlds = tree_size(&game.join("saves"), 0, &mut count)?;
            rows.push(json!({"id":id,"name":i["name"],"total":total,"worlds":worlds}));
        }
        rows.sort_by_key(|r| std::cmp::Reverse(r["total"].as_u64().unwrap_or(0)));
        let mut clean = Vec::new();
        for (key, label) in [
            (".graphite-downloads", "Временные загрузки"),
            ("cache", "Кеш Prism"),
        ] {
            let path = self.data.join(key);
            let bytes = tree_size(&path, 0, &mut count)?;
            if bytes > 0 {
                let mut files = Vec::new();
                let complete = cache_files(&path, &path, &mut files, 0)?;
                clean.push(json!({"key":key,"name":label,"size":bytes,"files":files,"truncated":!complete}))
            }
        }
        Ok(
            json!({"instances":rows,"backups":tree_size(&self.data.join("backups"),0,&mut count)?,"cleanable":clean,"files":count}),
        )
    }
    fn disk_clean(&self, p: &Value) -> Result<Value> {
        let _gate = self.filesystem_gate.lock().map_err(err)?;
        if p["confirmed"] != true {
            return Err("Подтверди очистку выбранных файлов".into());
        }
        if self.queue_worker.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("Сначала заверши очередь загрузок".into());
        }
        let _busy = self.busy.lock().map_err(err)?;
        if !_busy.is_empty() {
            return Err("Дождись операций со сборками".into());
        }
        let state = self.native.call("state", json!({}))?;
        if state["jobs"]
            .as_array()
            .is_some_and(|a| a.iter().any(|j| j["state"] == "running"))
        {
            return Err("Закрой игру и дождись завершения задач".into());
        }
        let keys = p["keys"].as_array().ok_or("Выбери кеш для очистки")?;
        let mut paths = Vec::new();
        for key in keys {
            let key = key.as_str().ok_or("Некорректный кеш")?;
            if ![".graphite-downloads", "cache"].contains(&key) {
                return Err("Эту папку нельзя очищать".into());
            }
            let path = self.data.join(key);
            if path.exists() {
                storage::directory(&path)?;
                let mut count = 0;
                tree_size(&path, 0, &mut count)?;
                paths.push(path)
            }
        }
        for path in paths {
            trash::delete(path).map_err(err)?
        }
        drop(_busy);
        self.disk_report()
    }
}
fn load_ini(path: &Path) -> Result<HashMap<String, String>> {
    if path.exists() {
        Ok(storage::ini(&storage::read(path, 1024 * 1024)?))
    } else {
        Ok(HashMap::new())
    }
}
fn cache_files(path: &Path, root: &Path, files: &mut Vec<Value>, depth: usize) -> Result<bool> {
    if files.len() >= 500 {
        return Ok(false);
    }
    if depth > 64 {
        return Err("Слишком большая вложенность кеша".into());
    }
    let m = fs::symlink_metadata(path).map_err(err)?;
    if storage::linked(&m) {
        return Ok(true);
    }
    if m.is_file() {
        files.push(
            json!({"path":path.strip_prefix(root).map_err(err)?.to_string_lossy(),"size":m.len()}),
        );
        return Ok(true);
    }
    let mut complete = true;
    for e in fs::read_dir(path).map_err(err)? {
        if !cache_files(&e.map_err(err)?.path(), root, files, depth + 1)? {
            complete = false;
            break;
        }
    }
    Ok(complete)
}
fn switch_mods(dir: &Path, old: &[String], next: &[String]) -> Result<()> {
    let mut moves = Vec::new();
    for name in old.iter().filter(|n| !next.contains(n)) {
        storage::component(name)?;
        moves.push((dir.join(format!("{name}.disabled")), dir.join(name)));
    }
    for name in next.iter().filter(|n| !old.contains(n)) {
        storage::component(name)?;
        moves.push((dir.join(name), dir.join(format!("{name}.disabled"))));
    }
    for (from, to) in &moves {
        storage::regular(from)?;
        if to.exists() {
            return Err("Состояние модов изменилось. Создай новую диагностическую копию".into());
        }
    }
    for (index, (from, to)) in moves.iter().enumerate() {
        if let Err(error) = fs::rename(from, to) {
            for (from, to) in moves[..index].iter().rev() {
                let _ = fs::rename(to, from);
            }
            return Err(err(error));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_constraints() {
        for (v, r, want) in [
            ("1.20.1", ">=1.20 <1.21", Some(true)),
            ("1.21.1", "1.20.*", Some(false)),
            ("1.20.1", "1.20.*", Some(true)),
            ("0.16.9", "^0.16.0", Some(true)),
            ("0.17.0", "^0.16.0", Some(false)),
            ("0.0.1", "^0.0.0", Some(false)),
            ("1.21.1", "~1.20", Some(false)),
            ("1.20.2", "~1.20", Some(true)),
            ("1.20.1", ">1.x", None),
            ("26.1", "26.x", Some(true)),
            ("1.0-beta", ">=1", None),
        ] {
            assert_eq!(version_matches(v, r), want, "{v} {r}")
        }
    }
    #[test]
    fn unreadable_jar_is_not_silently_accepted() {
        let p = std::env::temp_dir().join(format!("graphite-{}.jar", uuid::Uuid::new_v4()));
        fs::write(&p, b"broken").unwrap();
        assert!(jar(&p).is_err());
        let mut writer = zip::ZipWriter::new(fs::File::create(&p).unwrap());
        use std::io::Write;
        writer
            .start_file("fabric.mod.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"[]").unwrap();
        writer.finish().unwrap();
        assert!(jar(&p).is_err(), "Malformed JSON metadata must not panic");
        fs::remove_file(p).unwrap();
    }
}
