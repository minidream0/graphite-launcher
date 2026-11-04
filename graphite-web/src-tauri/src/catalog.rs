use crate::{err, storage, text, App, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256, Sha512};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
pub fn identifier(value: &str) -> Result<&str> {
    if value.is_empty()
        || value.len() > 100
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err("Недопустимый ID Modrinth".into());
    }
    Ok(value)
}
pub fn download_file(version: &Value, kind: &str) -> Result<Value> {
    let ext = match kind {
        "modpack" => ".mrpack",
        "mod" => ".jar",
        _ => ".zip",
    };
    let files = version["files"].as_array().ok_or("Нет файлов версии")?;
    let candidates: Vec<_> = files
        .iter()
        .filter(|f| text(f, "filename").to_lowercase().ends_with(ext))
        .collect();
    let file = candidates
        .iter()
        .find(|f| f["primary"] == true)
        .copied()
        .or_else(|| candidates.first().copied())
        .ok_or("Нет подходящего файла")?;
    let name = text(file, "filename");
    storage::component(name)?;
    if name.len() > 200
        || [
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ]
        .contains(&name.split('.').next().unwrap_or("").to_lowercase().as_str())
    {
        return Err("Недопустимое имя файла".into());
    }
    let value = json!({"filename":name,"url":file["url"],"sha512":file["hashes"]["sha512"]});
    validate_download(&value)?;
    Ok(value)
}
fn validate_download(file: &Value) -> Result<()> {
    let url = reqwest::Url::parse(text(file, "url")).map_err(err)?;
    let hash = text(file, "sha512");
    if url.scheme() != "https"
        || url.host_str() != Some("cdn.modrinth.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || !url.path().starts_with("/data/")
        || hash.len() != 128
        || !hash.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Недопустимый файл Modrinth".into());
    }
    Ok(())
}
pub struct Temporary(pub PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
impl App {
    pub fn json_url(&self, url: reqwest::Url) -> Result<Value> {
        let key = url.to_string();
        if let Some((at, value)) = self.cache.lock().map_err(err)?.get(&key) {
            if at.elapsed() < Duration::from_secs(60) {
                return Ok(value.clone());
            }
        }
        let mut response = self
            .http
            .get(url)
            .send()
            .map_err(|_| "Сервис временно недоступен")?;
        if !response.status().is_success() {
            return Err(if response.status().as_u16() == 429 {
                "Сервис ограничил частоту запросов".into()
            } else {
                format!("Сервис недоступен ({})", response.status().as_u16())
            });
        }
        let mut bytes = Vec::new();
        response
            .by_ref()
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Ответ сервиса слишком большой".into());
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(err)?;
        let mut cache = self.cache.lock().map_err(err)?;
        if cache.len() > 200 {
            cache.clear()
        }
        cache.insert(key, (Instant::now(), value.clone()));
        Ok(value)
    }
    pub fn api(&self, route: &str, params: Vec<(&str, String)>) -> Result<Value> {
        let mut url =
            reqwest::Url::parse(&format!("https://api.modrinth.com/v2/{route}")).map_err(err)?;
        for (k, v) in params {
            url.query_pairs_mut().append_pair(k, &v);
        }
        self.json_url(url)
    }
    pub fn download(&self, file: &Value, limit: u64) -> Result<Temporary> {
        validate_download(file)?;
        let root = self.data.join(".graphite-downloads");
        storage::directory(&root)?;
        let name = storage::component(text(file, "filename"))?;
        let path = root.join(format!("{}-{name}", uuid::Uuid::new_v4()));
        let temp = Temporary(path);
        let mut out = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp.0)
            .map_err(err)?;
        let mut response = self
            .http
            .get(text(file, "url"))
            .timeout(Duration::from_secs(180))
            .send()
            .map_err(|_| "Не удалось скачать файл")?;
        if !response.status().is_success() {
            return Err(format!(
                "Загрузка недоступна ({})",
                response.status().as_u16()
            ));
        }
        if response.content_length().is_some_and(|v| v > limit) {
            return Err("Файл превышает допустимый размер".into());
        }
        let mut hash = Sha512::new();
        let mut total = 0u64;
        let mut buf = [0u8; 65536];
        loop {
            let n = response.read(&mut buf).map_err(err)?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > limit {
                return Err("Файл превышает допустимый размер".into());
            }
            hash.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(err)?;
        }
        drop(out);
        if format!("{:x}", hash.finalize()) != text(file, "sha512").to_lowercase() {
            return Err("Контрольная сумма файла не совпала".into());
        }
        Ok(temp)
    }
    pub fn versions(&self, loader: &str, minecraft: &str, all: bool) -> Result<Value> {
        if ![
            "net.minecraft",
            "net.fabricmc.fabric-loader",
            "net.minecraftforge",
            "net.neoforged",
            "org.quiltmc.quilt-loader",
        ]
        .contains(&loader)
        {
            return Err("Неизвестный загрузчик".into());
        }
        let url = reqwest::Url::parse(&format!(
            "https://meta.prismlauncher.org/v1/{loader}/index.json"
        ))
        .map_err(err)?;
        let index = self.json_url(url)?;
        let values = index["versions"].as_array().ok_or("Некорректный каталог")?;
        Ok(json!(values
            .iter()
            .filter(|v| if loader == "net.minecraft" {
                all || v["type"] == "release"
            } else {
                minecraft.is_empty()
                    || !v["requires"].as_array().is_some_and(|a| {
                        a.iter().any(|r| {
                            r["uid"] == "net.minecraft"
                                && r["equals"].is_string()
                                && r["equals"] != minecraft
                        })
                    })
            })
            .map(|v| json!({"version":v["version"],"type":v["type"],"date":v["releaseTime"]}))
            .collect::<Vec<_>>()))
    }
    fn target(&self, id: &str, kind: &str) -> Result<Value> {
        let lib = self.library()?;
        let item = lib
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id && x["error"] != true)
            .ok_or("Сборка не найдена")?
            .clone();
        if kind == "mod"
            && !matches!(
                text(&item, "loader"),
                "Fabric" | "Forge" | "NeoForge" | "Quilt"
            )
        {
            return Err("Для модов нужен Fabric, Forge, NeoForge или Quilt".into());
        }
        Ok(item)
    }
    fn project_versions(
        &self,
        project: &str,
        kind: &str,
        target: Option<&Value>,
        filters: &Value,
    ) -> Result<Vec<Value>> {
        identifier(project)?;
        let mut params = vec![("include_changelog", "false".into())];
        let version = target
            .map(|t| text(t, "version"))
            .unwrap_or(text(filters, "minecraft"));
        if !version.is_empty() {
            params.push(("game_versions", json!([version]).to_string()))
        }
        let loader = if kind == "mod" {
            target
                .map(|t| text(t, "loader").to_lowercase())
                .unwrap_or_default()
        } else if target.is_none() {
            text(filters, "loader").into()
        } else {
            String::new()
        };
        if !loader.is_empty() {
            params.push(("loaders", json!([loader]).to_string()))
        }
        let result = self.api(&format!("project/{project}/version"), params)?;
        let values = result.as_array().ok_or("Некорректный список версий")?;
        Ok(values
            .iter()
            .filter(|v| {
                target.map(|t| compatible(v, t, kind)).unwrap_or(true)
                    && v["files"].as_array().is_some_and(|a| {
                        a.iter().any(|f| {
                            text(f, "filename").to_lowercase().ends_with(match kind {
                                "mod" => ".jar",
                                "modpack" => ".mrpack",
                                _ => ".zip",
                            })
                        })
                    })
            })
            .cloned()
            .collect())
    }
    pub fn modrinth(&self, method: &str, p: &Value) -> Result<Value> {
        let kind = text(p, "kind");
        if !matches!(kind, "mod" | "modpack" | "resourcepack" | "shader") {
            return Err("Неизвестный тип проекта".into());
        }
        let target = if kind != "modpack" {
            Some(self.target(text(p, "instanceId"), kind)?)
        } else {
            None
        };
        if method == "search" {
            let mut facets = vec![json!([format!("project_type:{kind}")])];
            let version = target
                .as_ref()
                .map(|t| text(t, "version"))
                .unwrap_or(text(p, "minecraft"));
            if !version.is_empty() {
                facets.push(json!([format!("versions:{version}")]))
            }
            let loader = if kind == "mod" {
                target
                    .as_ref()
                    .map(|t| text(t, "loader").to_lowercase())
                    .unwrap_or_default()
            } else if kind == "modpack" {
                text(p, "loader").into()
            } else {
                String::new()
            };
            if !loader.is_empty() {
                facets.push(json!([format!("categories:{loader}")]))
            }
            let query: String = text(p, "query").chars().take(120).collect();
            let offset = p["offset"].as_u64().unwrap_or(0);
            if offset > 10000 {
                return Err("Недопустимая страница".into());
            }
            let result = self.api(
                "search",
                vec![
                    ("query", query.clone()),
                    ("facets", json!(facets).to_string()),
                    (
                        "index",
                        if query.trim().is_empty() {
                            "downloads"
                        } else {
                            "relevance"
                        }
                        .into(),
                    ),
                    ("limit", "20".into()),
                    ("offset", offset.to_string()),
                ],
            )?;
            let hits = result["hits"]
                .as_array()
                .ok_or("Некорректный ответ Modrinth")?;
            return Ok(
                json!({"total":result["total_hits"],"offset":offset,"hits":hits.iter().map(|v|json!({"id":v["project_id"],"slug":v["slug"],"title":v["title"],"description":v["description"],"author":v["author"],"downloads":v["downloads"],"icon":if text(v,"icon_url").starts_with("https://cdn.modrinth.com/"){text(v,"icon_url")}else{""}})).collect::<Vec<_>>() }),
            );
        }
        let project = identifier(text(p, "projectId"))?;
        if method == "versions" {
            let rows = self.project_versions(project, kind, target.as_ref(), p)?;
            return Ok(json!(rows.iter().map(|v|json!({"id":v["id"],"name":v["name"],"number":v["version_number"],"minecraft":v["game_versions"],"loaders":v["loaders"],"date":v["date_published"],"type":v["version_type"]})).collect::<Vec<_>>()));
        }
        if method != "install" {
            return Err("Неизвестная команда Modrinth".into());
        }
        let version = identifier(text(p, "versionId"))?;
        let selected = self.api(&format!("version/{version}"), vec![])?;
        if selected["project_id"] != project {
            return Err("Версия относится к другому проекту".into());
        }
        let info = self.api(&format!("project/{project}"), vec![])?;
        if info["project_type"] != kind {
            return Err("Неверный тип проекта".into());
        }
        if kind == "modpack" {
            let name = if text(p, "name").trim().is_empty() {
                text(&info, "title")
            } else {
                text(p, "name")
            };
            return self.engine("modrinth.pack",json!({"project":project,"version":version,"name":name.chars().take(100).collect::<String>(),"optional":p["optional"]!=false,"file":download_file(&selected,kind)?}));
        }
        let target = target.unwrap();
        if !compatible(&selected, &target, kind) {
            return Err("Версия не подходит для этой сборки".into());
        }
        let id = text(&target, "id");
        if kind != "mod" {
            return self.with_instance(id, || {
                let file = download_file(&selected, kind)?;
                let dir = self.resource_dir(id, kind)?;
                no_collision(&dir, text(&file, "filename"))?;
                let temp = self.download(&file, 512 * 1024 * 1024)?;
                storage::zip_info(&temp.0, kind)?;
                storage::copy_new(&temp.0, &dir.join(text(&file, "filename")))?;
                Ok(json!({"installed":true}))
            });
        }
        let mut selected_ids = HashMap::new();
        let mut ordered = Vec::new();
        self.mod_plan(selected, &target, &mut selected_ids, &mut ordered)?;
        for v in &ordered {
            if let Some(deps) = v["dependencies"].as_array() {
                for dep in deps {
                    if dep["dependency_type"] == "incompatible" {
                        if let Some(id) = selected_ids.get(text(dep, "project_id")) {
                            if text(dep, "version_id").is_empty() || dep["version_id"] == *id {
                                return Err("Выбранные моды объявлены несовместимыми".into());
                            }
                        }
                    }
                }
            }
        }
        for v in &mut ordered {
            v.as_object_mut().unwrap().remove("dependencies");
        }
        let payload = json!({"id":id,"files":ordered});
        if payload.to_string().len() > 58000 {
            return Err("Слишком большой план установки".into());
        }
        self.engine("modrinth.mods", payload)
    }
    fn mod_plan(
        &self,
        v: Value,
        target: &Value,
        selected: &mut HashMap<String, String>,
        ordered: &mut Vec<Value>,
    ) -> Result<()> {
        if !compatible(&v, target, "mod") {
            return Err(format!(
                "Мод {} не подходит для этой сборки",
                text(&v, "name")
            ));
        }
        let project = identifier(text(&v, "project_id"))?.to_string();
        let id = identifier(text(&v, "id"))?.to_string();
        if let Some(existing) = selected.get(&project) {
            if existing != &id {
                return Err("Конфликт обязательных версий зависимости".into());
            }
            return Ok(());
        }
        if selected.len() >= 64 {
            return Err("Слишком много зависимостей".into());
        }
        selected.insert(project.clone(), id.clone());
        let info = self.api(&format!("project/{project}"), vec![])?;
        if info["project_type"] != "mod" || info["client_side"] == "unsupported" {
            return Err("Мод не предназначен для клиента".into());
        }
        if let Some(deps) = v["dependencies"].as_array() {
            for dep in deps {
                if dep["dependency_type"] != "required" {
                    continue;
                }
                let child = if !text(dep, "version_id").is_empty() {
                    let version = identifier(text(dep, "version_id"))?;
                    self.api(&format!("version/{version}"), vec![])?
                } else if !text(dep, "project_id").is_empty() {
                    let p = identifier(text(dep, "project_id"))?;
                    if selected.contains_key(p) {
                        continue;
                    }
                    self.project_versions(p, "mod", Some(target), &json!({}))?
                        .into_iter()
                        .next()
                        .ok_or("Нет совместимой обязательной зависимости")?
                } else {
                    return Err("Не найдена обязательная зависимость".into());
                };
                if !text(dep, "project_id").is_empty() && child["project_id"] != dep["project_id"] {
                    return Err("Неверный проект зависимости".into());
                }
                self.mod_plan(child, target, selected, ordered)?;
            }
        }
        let mut file = download_file(&v, "mod")?;
        let obj = file.as_object_mut().unwrap();
        for (k, val) in [
            ("project", json!(project)),
            ("slug", json!(text(&info, "slug"))),
            ("version", json!(id)),
            ("title", info["title"].clone()),
            ("versionName", v["version_number"].clone()),
            ("gameVersions", v["game_versions"].clone()),
            ("loaders", v["loaders"].clone()),
            ("dependencies", v["dependencies"].clone()),
        ] {
            obj.insert(k.into(), val);
        }
        ordered.push(file);
        Ok(())
    }
    pub fn pack_preview(&self, id: &str) -> Result<Value> {
        let folder = self.instance(id)?;
        let info = self.engine("instance.get", json!({"id":id}))?;
        let project = text(&info, "managedId");
        if info["managedType"] != "modrinth"
            || project.is_empty()
            || text(&info, "managedVersion").is_empty()
        {
            return Err("Нет данных Modrinth для обновления этой сборки".into());
        }
        let versions = self.project_versions(project, "modpack", None, &json!({}))?;
        let latest = versions.first().ok_or("Нет доступных версий")?;
        if latest["id"] == info["managedVersion"] {
            return Ok(json!({"current":true,"version":latest["version_number"]}));
        }
        let file = download_file(latest, "modpack")?;
        let temp = self.download(&file, 128 * 1024 * 1024)?;
        let archive = storage::zip_info(&temp.0, "modpack")?;
        let index = storage::read(&folder.join("mrpack/modrinth.index.json"), 10 * 1024 * 1024)?;
        let old: Value = serde_json::from_str(&index).map_err(err)?;
        let old_overrides =
            storage::read(&folder.join("mrpack/overrides.txt"), 1024 * 1024).unwrap_or_default();
        let changes = compare(
            &old,
            &archive["index"],
            old_overrides.lines().map(str::to_string).collect(),
            archive["overrides"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
        );
        let token = uuid::Uuid::new_v4().to_string();
        let mut plans = self.plans.lock().map_err(err)?;
        plans.clear();
        plans.insert(token.clone(),json!({"id":id,"project":project,"current":info["managedVersion"],"version":latest["id"],"name":info["name"],"oldHash":format!("{:x}",Sha256::digest(index.as_bytes()))}));
        Ok(json!({"token":token,"version":latest["version_number"],"changes":changes}))
    }
    pub fn pack_update(&self, id: &str, token: &str) -> Result<Value> {
        let plan = self
            .plans
            .lock()
            .map_err(err)?
            .remove(token)
            .ok_or("Сначала проверь изменения")?;
        if plan["id"] != id {
            return Err("Неверная сборка обновления".into());
        }
        self.with_instance(id,||{let info=self.engine("instance.get",json!({"id":id}))?;let index=storage::read(&self.instance(id)?.join("mrpack/modrinth.index.json"),10*1024*1024)?;if info["managedVersion"]!=plan["current"]||info["managedId"]!=plan["project"]||format!("{:x}",Sha256::digest(index.as_bytes()))!=text(&plan,"oldHash"){return Err("Сборка изменилась. Проверь обновление заново".into())}let backup=self.backup(id)?;let name=format!("{} · обновление {}",text(&plan,"name").chars().take(65).collect::<String>(),&uuid::Uuid::new_v4().simple().to_string()[..8]);let started=self.modrinth("install",&json!({"kind":"modpack","projectId":plan["project"],"versionId":plan["version"],"name":name,"optional":true}))?;let job=text(&started,"job");let mut success=false;let mut missing=0;for _ in 0..2400{std::thread::sleep(Duration::from_secs(1));let state=self.engine("state",json!({}))?;if let Some(item)=state["jobs"].as_array().and_then(|a|a.iter().find(|x|x["id"]==job)){if item["state"]!="running"{if item["state"]!="succeeded"{return Err(if text(item,"error").is_empty(){"Установка не завершена".into()}else{text(item,"error").into()})}success=true;break}}else{missing+=1;if missing>=10{return Err("Задача установки прервалась; исходная сборка сохранена".into())}}}if !success{return Err("Превышено время установки".into())}let lib=self.library()?;let found:Vec<_>=lib.as_array().unwrap().iter().filter(|x|x["name"]==name).collect();if found.len()!=1{return Err("Не удалось определить новую сборку; копия сохранена".into())}let target=text(found[0],"id");self.with_instance(target,||{let from=PathBuf::from(text(&backup,"path")).join("minecraft");let to=self.game(target)?;for e in fs::read_dir(from).map_err(err)?{let e=e.map_err(err)?;storage::copy_tree(&e.path(),&to.join(e.file_name()),true)?}Ok(json!({"id":target,"name":name,"backup":backup["id"]}))})})
    }
}
fn compatible(v: &Value, target: &Value, kind: &str) -> bool {
    v["game_versions"]
        .as_array()
        .is_some_and(|a| a.contains(&target["version"]))
        && (kind != "mod"
            || v["loaders"]
                .as_array()
                .is_some_and(|a| a.contains(&json!(text(target, "loader").to_lowercase()))))
}
fn no_collision(dir: &std::path::Path, name: &str) -> Result<()> {
    let lower = name.to_lowercase();
    for e in fs::read_dir(dir).map_err(err)? {
        let old = e.map_err(err)?.file_name().to_string_lossy().to_lowercase();
        if old == lower || old == format!("{lower}.disabled") {
            return Err(format!("Файл уже существует: {name}"));
        }
    }
    Ok(())
}
pub fn compare(
    old: &Value,
    next: &Value,
    old_overrides: Vec<String>,
    overrides: Vec<String>,
) -> Value {
    let files = |v: &Value| -> HashMap<String, String> {
        v["files"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|f| {
                (
                    text(f, "path").into(),
                    f["hashes"]["sha512"]
                        .as_str()
                        .or_else(|| f["hashes"]["sha1"].as_str())
                        .unwrap_or("")
                        .into(),
                )
            })
            .collect()
    };
    let oldfiles = files(old);
    let newfiles = files(next);
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    for (path, hash) in &newfiles {
        if !oldfiles.contains_key(path) {
            added.push(path.clone())
        } else if hash.is_empty() || oldfiles.get(path) != Some(hash) {
            changed.push(path.clone())
        }
    }
    for path in oldfiles.keys() {
        if !newfiles.contains_key(path) {
            removed.push(path.clone())
        }
    }
    added.sort();
    removed.sort();
    changed.sort();
    let removed_overrides: Vec<_> = old_overrides
        .into_iter()
        .filter(|x| !overrides.contains(x))
        .collect();
    json!({"added":added,"removed":removed,"changed":changed,"oldDependencies":old["dependencies"].as_object().cloned().unwrap_or_default(),"newDependencies":next["dependencies"].as_object().cloned().unwrap_or_default(),"overrides":overrides,"removedOverrides":removed_overrides})
}
