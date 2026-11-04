use crate::{err, storage, App, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::Path, sync::atomic::Ordering};
#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Profiles {
    pub version: u8,
    pub profiles: Vec<Profile>,
    pub selected: Option<String>,
}
impl Default for Profiles {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: Vec::new(),
            selected: None,
        }
    }
}
pub fn name(value: &str) -> Result<&str> {
    let name = value.trim();
    if name.is_empty()
        || name.len() > 16
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("Ник: от 1 до 16 латинских букв, цифр или _".into());
    }
    Ok(name)
}
pub fn load(root: &Path) -> Result<Profiles> {
    let file = root.join("graphite-offline-accounts.json");
    if !file.exists() {
        return Ok(Profiles::default());
    }
    let profiles: Profiles = serde_json::from_str(&storage::read(&file, 65536)?)
        .map_err(|_| "Не удалось прочитать graphite-offline-accounts.json")?;
    if profiles.version != 1 || profiles.profiles.len() > 32 {
        return Err("Некорректный файл офлайн-профилей".into());
    }
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for p in &profiles.profiles {
        let key =
            p.id.strip_prefix("offline:")
                .ok_or("Некорректный ID профиля")?;
        if uuid::Uuid::parse_str(key).is_err()
            || !ids.insert(&p.id)
            || !names.insert(p.name.to_ascii_lowercase())
            || name(&p.name)? != p.name
        {
            return Err("Некорректный офлайн-профиль".into());
        }
    }
    if profiles
        .selected
        .as_ref()
        .is_some_and(|id| !ids.contains(id))
    {
        return Err("Выбранный офлайн-профиль не найден".into());
    }
    Ok(profiles)
}
fn save(root: &Path, value: &Profiles) -> Result<()> {
    let temp =
        crate::catalog::Temporary(root.join(format!(".offline-{}.tmp", uuid::Uuid::new_v4())));
    storage::write_new(&temp.0, &serde_json::to_vec_pretty(value).map_err(err)?)?;
    std::fs::rename(&temp.0, root.join("graphite-offline-accounts.json")).map_err(err)
}
impl App {
    pub fn select_offline(&self, id: Option<&str>) -> Result<()> {
        let mut profiles = self.offline.lock().map_err(err)?;
        if let Some(id) = id {
            if !profiles.profiles.iter().any(|p| p.id == id) {
                return Err("Офлайн-профиль не найден".into());
            }
        }
        let selected = id.map(str::to_owned);
        if profiles.selected == selected {
            return Ok(());
        }
        let mut updated = profiles.clone();
        updated.selected = selected;
        save(&self.data, &updated)?;
        *profiles = updated;
        Ok(())
    }
    pub fn add_offline(&self, value: &str) -> Result<Value> {
        let name = name(value)?;
        let mut profiles = self.offline.lock().map_err(err)?;
        if profiles.profiles.len() >= 32 {
            return Err("Можно сохранить до 32 офлайн-профилей".into());
        }
        if profiles
            .profiles
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(name))
        {
            return Err("Этот ник уже добавлен".into());
        }
        let mut updated = profiles.clone();
        let id = format!("offline:{}", uuid::Uuid::new_v4());
        updated.profiles.push(Profile {
            id: id.clone(),
            name: name.into(),
        });
        updated.selected = Some(id.clone());
        save(&self.data, &updated)?;
        *profiles = updated;
        self.selected.store(false, Ordering::Relaxed);
        Ok(json!({"id":id,"name":name,"type":"offline"}))
    }
    pub fn remove_offline(&self, id: &str) -> Result<Value> {
        let mut profiles = self.offline.lock().map_err(err)?;
        if !profiles.profiles.iter().any(|p| p.id == id) {
            return Err("Офлайн-профиль не найден".into());
        }
        let mut updated = profiles.clone();
        updated.profiles.retain(|p| p.id != id);
        if updated.selected.as_deref() == Some(id) {
            updated.selected = None;
        }
        save(&self.data, &updated)?;
        *profiles = updated;
        Ok(json!({"ok":true}))
    }
    pub fn offline_name(&self) -> Result<Option<String>> {
        let profiles = self.offline.lock().map_err(err)?;
        Ok(profiles
            .selected
            .as_ref()
            .and_then(|id| profiles.profiles.iter().find(|p| &p.id == id))
            .map(|p| p.name.clone()))
    }
}
