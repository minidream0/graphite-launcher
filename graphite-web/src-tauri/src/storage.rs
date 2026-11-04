use crate::{err, text, App, Result};
use serde_json::{json, Value};
use std::os::windows::fs::MetadataExt;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub fn linked(meta: &fs::Metadata) -> bool {
    meta.file_attributes() & 0x400 != 0
}
pub fn directory(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(err)?
    }
    let m = fs::symlink_metadata(path).map_err(err)?;
    if !m.is_dir() || linked(&m) {
        return Err("Папка недоступна или является ссылкой".into());
    }
    Ok(())
}
pub fn regular(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path).map_err(err)?;
    if !m.is_file() || linked(&m) {
        return Err("Нужен обычный файл без ссылок".into());
    }
    Ok(())
}
pub fn component(value: &str) -> Result<&str> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value
            .chars()
            .any(|c| c.is_control() || "\\/:<>\"|?*".contains(c))
    {
        Err("Недопустимое имя файла или сборки".into())
    } else {
        Ok(value)
    }
}
fn enabled_name(name: &str) -> &str {
    if name.to_ascii_lowercase().ends_with(".disabled") {
        &name[..name.len() - 9]
    } else {
        name
    }
}
pub fn read(path: &Path, limit: u64) -> Result<String> {
    regular(path)?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(err)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() as u64 > limit {
        return Err("Файл слишком большой".into());
    }
    String::from_utf8(bytes).map_err(err)
}
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(err)?;
    f.write_all(bytes).map_err(err)?;
    f.sync_all().map_err(err)
}
pub fn copy_new(source: &Path, target: &Path) -> Result<()> {
    regular(source)?;
    let mut from = File::open(source).map_err(err)?;
    let mut to = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(target)
        .map_err(err)?;
    if let Err(e) = std::io::copy(&mut from, &mut to) {
        drop(to);
        let _ = fs::remove_file(target);
        return Err(err(e));
    }
    Ok(())
}
pub fn copy_tree(source: &Path, target: &Path, overwrite: bool) -> Result<()> {
    let m = fs::symlink_metadata(source).map_err(err)?;
    if linked(&m) {
        return Err("В сборке найдена ссылка; копирование остановлено".into());
    }
    if m.is_dir() {
        directory(target)?;
        for entry in fs::read_dir(source).map_err(err)? {
            let e = entry.map_err(err)?;
            copy_tree(&e.path(), &target.join(e.file_name()), overwrite)?
        }
    } else if m.is_file() {
        if target.exists() {
            regular(target)?;
            if !overwrite {
                return Err("Файл уже существует".into());
            }
            fs::copy(source, target).map_err(err)?;
        } else {
            copy_new(source, target)?
        }
    }
    Ok(())
}
pub fn ini(s: &str) -> HashMap<String, String> {
    s.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with(['#', ';', '[']) {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            Some((k.trim().into(), v.trim().trim_matches('"').into()))
        })
        .collect()
}
pub fn zip_info(path: &Path, kind: &str) -> Result<Value> {
    let mut zip = zip::ZipArchive::new(File::open(path).map_err(err)?).map_err(err)?;
    if zip.len() > 20000 {
        return Err("Слишком много файлов в ZIP".into());
    }
    let mut valid = false;
    let mut overrides = Vec::new();
    let mut index = Value::Null;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(err)?;
        if f.enclosed_name().is_none() {
            return Err("Недопустимый путь внутри ZIP".into());
        }
        let name = f.name().to_string();
        if kind == "resourcepack" && name == "pack.mcmeta"
            || kind == "shader" && name.starts_with("shaders/")
        {
            valid = true
        }
        if kind == "modpack" {
            if let Some(name) = name
                .strip_prefix("overrides/")
                .or_else(|| name.strip_prefix("client-overrides/"))
            {
                if !name.ends_with('/') {
                    overrides.push(name.to_string())
                }
            }
            if name == "modrinth.index.json" {
                if !index.is_null() || f.size() > 10 * 1024 * 1024 {
                    return Err("Некорректный индекс сборки".into());
                }
                let mut s = String::new();
                f.by_ref()
                    .take(10 * 1024 * 1024 + 1)
                    .read_to_string(&mut s)
                    .map_err(err)?;
                index = serde_json::from_str(&s).map_err(err)?;
            }
        }
    }
    if kind == "modpack" {
        if index["game"] != "minecraft" || !index["files"].is_array() {
            return Err("Некорректный индекс Modrinth".into());
        }
        return Ok(json!({"index":index,"overrides":overrides}));
    }
    if !valid {
        return Err(if kind == "shader" {
            "В ZIP нет папки shaders"
        } else {
            "В ZIP нет pack.mcmeta"
        }
        .into());
    }
    Ok(Value::Null)
}
impl App {
    pub fn instance(&self, id: &str) -> Result<PathBuf> {
        component(id)?;
        let base = self.data.join("instances");
        directory(&base)?;
        let path = base.join(id);
        let m = fs::symlink_metadata(&path).map_err(err)?;
        if !m.is_dir() || linked(&m) {
            return Err("Сборка недоступна".into());
        }
        Ok(path)
    }
    pub fn game(&self, id: &str) -> Result<PathBuf> {
        let root = self.instance(id)?;
        for name in ["minecraft", ".minecraft"] {
            let dir = root.join(name);
            if dir.exists() {
                directory(&dir)?;
                return Ok(dir);
            }
        }
        let dir = root.join("minecraft");
        directory(&dir)?;
        Ok(dir)
    }
    pub fn library(&self) -> Result<Value> {
        let root = self.data.join("instances");
        if !root.exists() {
            return Ok(json!([]));
        }
        directory(&root)?;
        let mut items = Vec::new();
        for entry in fs::read_dir(&root).map_err(err)? {
            let entry = entry.map_err(err)?;
            let id = entry.file_name().to_string_lossy().to_string();
            let meta = fs::symlink_metadata(entry.path()).map_err(err)?;
            if !meta.is_dir() || linked(&meta) || id.starts_with('.') {
                continue;
            }
            let item = (|| -> Result<Value> {
                let settings = ini(&read(&entry.path().join("instance.cfg"), 1024 * 1024)?);
                let pack_path = entry.path().join("mmc-pack.json");
                let pack: Value = if pack_path.exists() {
                    serde_json::from_str(&read(&pack_path, 1024 * 1024)?).map_err(err)?
                } else {
                    json!({"components":[]})
                };
                let components = pack["components"]
                    .as_array()
                    .ok_or("Повреждена конфигурация")?;
                let version = components
                    .iter()
                    .find(|c| c["uid"] == "net.minecraft")
                    .and_then(|c| c["version"].as_str())
                    .unwrap_or("Не указана");
                let mut loader = "Vanilla";
                for c in components {
                    loader = match c["uid"].as_str().unwrap_or("") {
                        "net.fabricmc.fabric-loader" => "Fabric",
                        "net.minecraftforge" => "Forge",
                        "net.neoforged" => "NeoForge",
                        "org.quiltmc.quilt-loader" => "Quilt",
                        _ => loader,
                    };
                }
                Ok(
                    json!({"id":id,"name":settings.get("name").unwrap_or(&id),"icon":settings.get("iconKey").map(String::as_str).unwrap_or("default"),"version":version,"loader":loader,"playtime":settings.get("totalTimePlayed").and_then(|v|v.parse::<u64>().ok()).unwrap_or(0),"lastPlayed":settings.get("lastLaunchTime").and_then(|v|v.parse::<u64>().ok()).unwrap_or(0)}),
                )
            })();
            items.push(item.unwrap_or_else(|_|json!({"id":id,"name":id,"version":"Ошибка конфигурации","loader":"Неизвестно","error":true,"playtime":0,"lastPlayed":0})));
        }
        items.sort_by(|a, b| {
            b["lastPlayed"]
                .as_u64()
                .cmp(&a["lastPlayed"].as_u64())
                .then(
                    text(a, "name")
                        .to_lowercase()
                        .cmp(&text(b, "name").to_lowercase()),
                )
        });
        Ok(json!(items))
    }
    pub fn resource_dir(&self, id: &str, kind: &str) -> Result<PathBuf> {
        let name = match kind {
            "mod" => "mods",
            "resourcepack" => "resourcepacks",
            "shader" => "shaderpacks",
            _ => return Err("Неизвестный тип ресурсов".into()),
        };
        let dir = self.game(id)?.join(name);
        directory(&dir)?;
        Ok(dir)
    }
    pub fn files(&self, id: &str, kind: &str) -> Result<Value> {
        let dir = self.resource_dir(id, kind)?;
        let mut metadata = HashMap::new();
        let index = dir.join(".index");
        if kind == "mod" && index.exists() {
            directory(&index)?;
            for entry in fs::read_dir(index).map_err(err)? {
                let p = entry.map_err(err)?.path();
                if p.to_string_lossy().ends_with(".pw.toml") {
                    if let Ok(content) = read(&p, 1024 * 1024) {
                        let data: HashMap<_, _> = content
                            .lines()
                            .filter_map(|l| {
                                let (k, v) = l.split_once('=')?;
                                Some((
                                    k.trim().to_string(),
                                    v.trim().trim_matches(['\'', '"']).to_string(),
                                ))
                            })
                            .collect();
                        if let Some(name) = data.get("filename") {
                            metadata.insert(name.clone(), data);
                        }
                    }
                }
            }
        }
        let mut items = Vec::new();
        for entry in fs::read_dir(&dir).map_err(err)? {
            let entry = entry.map_err(err)?;
            let m = fs::symlink_metadata(entry.path()).map_err(err)?;
            if !m.is_file() || linked(&m) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let base = enabled_name(&name);
            if !base
                .to_lowercase()
                .ends_with(if kind == "mod" { ".jar" } else { ".zip" })
            {
                continue;
            }
            if kind == "mod" {
                let meta = metadata.get(base);
                let get = |key: &str| meta.and_then(|m| m.get(key)).cloned().unwrap_or_default();
                let embedded =
                    if get("name").is_empty() || get("x-prismlauncher-version-number").is_empty() {
                        jar_metadata(&entry.path()).unwrap_or_default()
                    } else {
                        HashMap::new()
                    };
                let modname = meta
                    .and_then(|m| m.get("name"))
                    .or_else(|| embedded.get("name"))
                    .cloned()
                    .unwrap_or(base.into());
                let version = meta
                    .and_then(|m| m.get("x-prismlauncher-version-number"))
                    .or_else(|| embedded.get("version"))
                    .cloned()
                    .unwrap_or_default();
                items.push(json!({"file":name,"name":modname,"version":version,"project":get("mod-id"),"versionId":get("version"),"enabled":name==base}));
            } else {
                items.push(json!({"file":name,"size":m.len(),"enabled":name==base}));
            }
        }
        items.sort_by_key(|x| text(x, if kind == "mod" { "name" } else { "file" }).to_lowercase());
        Ok(json!(items))
    }
    pub fn file_action(&self, method: &str, id: &str, kind: &str, name: &str) -> Result<Value> {
        if method == "list" {
            return self.files(id, kind);
        }
        let dir = self.resource_dir(id, kind)?;
        if method == "folder" {
            open::that(&dir).map_err(err)?;
            return Ok(Value::Null);
        }
        self.with_instance(id, || {
            component(name)?;
            let base = enabled_name(name);
            if !base
                .to_lowercase()
                .ends_with(if kind == "mod" { ".jar" } else { ".zip" })
            {
                return Err("Неверный формат файла".into());
            }
            let file = dir.join(name);
            regular(&file)?;
            match method {
                "toggle" => {
                    let target = dir.join(if name == base {
                        format!("{name}.disabled")
                    } else {
                        base.into()
                    });
                    copy_new(&file, &target)?;
                    if let Err(e) = fs::remove_file(&file) {
                        let _ = fs::remove_file(target);
                        return Err(err(e));
                    }
                }
                "remove" => trash::delete(&file).map_err(err)?,
                _ => return Err("Неизвестное действие".into()),
            }
            self.files(id, kind)
        })
    }
    pub fn install_local(&self, id: &str, kind: &str, paths: &[PathBuf]) -> Result<Value> {
        self.with_instance(id, || {
            if paths.is_empty() || paths.len() > 64 {
                return Err("Выбери до 64 файлов".into());
            }
            if kind == "mod" {
                let lib = self.library()?;
                let item = lib
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|x| x["id"] == id)
                    .ok_or("Сборка не найдена")?;
                if item["loader"] == "Vanilla" {
                    return Err("Для модов нужен Fabric, Forge, NeoForge или Quilt".into());
                }
            }
            let dir = self.resource_dir(id, kind)?;
            let mut names: HashSet<_> = fs::read_dir(&dir)
                .map_err(err)?
                .filter_map(|e| {
                    e.ok()
                        .map(|e| e.file_name().to_string_lossy().to_lowercase())
                })
                .collect();
            for p in paths {
                regular(p)?;
                let name = p
                    .file_name()
                    .ok_or("Нет имени файла")?
                    .to_string_lossy()
                    .to_string();
                component(&name)?;
                if !name
                    .to_lowercase()
                    .ends_with(if kind == "mod" { ".jar" } else { ".zip" })
                {
                    return Err("Неверный формат файла".into());
                }
                if kind != "mod" {
                    zip_info(p, kind)?;
                }
                let lower = name.to_lowercase();
                if names.contains(&lower) || names.contains(&format!("{lower}.disabled")) {
                    return Err(format!("Файл уже существует: {name}"));
                }
                names.insert(lower);
            }
            let mut created = Vec::new();
            for p in paths {
                let target = dir.join(p.file_name().unwrap());
                if let Err(e) = copy_new(p, &target) {
                    for file in created {
                        let _ = fs::remove_file(file);
                    }
                    return Err(e);
                }
                created.push(target);
            }
            self.files(id, kind)
        })
    }
    pub fn backup(&self, id: &str) -> Result<Value> {
        let source = self.instance(id)?;
        regular(&source.join("instance.cfg"))?;
        let game = self.game(id)?;
        let root = self.data.join("backups");
        directory(&root)?;
        let key = uuid::Uuid::new_v4().to_string();
        let stage = root.join(format!(".partial-{key}"));
        directory(&stage)?;
        for name in ["instance.cfg", "mmc-pack.json"] {
            let src = source.join(name);
            if src.exists() {
                copy_tree(&src, &stage.join(name), false)?
            }
        }
        let dest = stage.join("minecraft");
        directory(&dest)?;
        for name in [
            "saves",
            "config",
            "options.txt",
            "servers.dat",
            "resourcepacks",
            "shaderpacks",
        ] {
            let src = game.join(name);
            if src.exists() {
                copy_tree(&src, &dest.join(name), false)?
            }
        }
        write_new(&stage.join("graphite-backup.json"),json!({"instance":id,"created":chrono::Utc::now().to_rfc3339(),"gameDirectory":game.file_name().unwrap().to_string_lossy()}).to_string().as_bytes())?;
        let path = root.join(&key);
        fs::rename(stage, &path).map_err(err)?;
        Ok(json!({"id":key,"path":path}))
    }
    pub fn backups(&self, id: &str) -> Result<Value> {
        self.instance(id)?;
        let root = self.data.join("backups");
        directory(&root)?;
        let mut items = Vec::new();
        for entry in fs::read_dir(root).map_err(err)? {
            let e = entry.map_err(err)?;
            let name = e.file_name().to_string_lossy().to_string();
            if uuid::Uuid::parse_str(&name).is_err() {
                continue;
            }
            if let Ok(s) = read(&e.path().join("graphite-backup.json"), 16384) {
                if let Ok(v) = serde_json::from_str::<Value>(&s) {
                    if v["instance"] == id && v["created"].is_string() {
                        items.push(json!({"id":name,"created":v["created"]}))
                    }
                }
            }
        }
        items.sort_by(|a, b| text(b, "created").cmp(text(a, "created")));
        Ok(json!(items))
    }
}
fn jar_metadata(path: &Path) -> Result<HashMap<String, String>> {
    let mut zip = zip::ZipArchive::new(File::open(path).map_err(err)?).map_err(err)?;
    let mut out = HashMap::new();
    for name in [
        "fabric.mod.json",
        "quilt.mod.json",
        "META-INF/mods.toml",
        "META-INF/neoforge.mods.toml",
        "mcmod.info",
    ] {
        if let Ok(mut f) = zip.by_name(name) {
            if f.size() > 131072 {
                continue;
            }
            let mut s = String::new();
            f.by_ref()
                .take(131073)
                .read_to_string(&mut s)
                .map_err(err)?;
            if name.ends_with(".toml") {
                for line in s.lines() {
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        if key == "displayName" || key == "version" {
                            let v = value.trim().trim_matches(['\'', '"']);
                            if !v.starts_with('$') {
                                out.insert(
                                    if key == "displayName" { "name" } else { key }.into(),
                                    v.into(),
                                );
                            }
                        }
                    }
                }
            } else {
                let mut v: Value = serde_json::from_str(&s).map_err(err)?;
                if v.is_array() {
                    v = v[0].clone()
                }
                if name == "quilt.mod.json" {
                    out.insert(
                        "name".into(),
                        text(&v["quilt_loader"]["metadata"], "name").into(),
                    );
                    out.insert("version".into(), text(&v["quilt_loader"], "version").into());
                } else {
                    for key in ["name", "version"] {
                        if let Some(value) = v[key].as_str() {
                            out.insert(key.into(), value.into());
                        }
                    }
                }
            }
            break;
        }
    }
    Ok(out)
}
