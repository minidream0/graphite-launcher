use crate::{err, text, App, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::atomic::Ordering;
#[derive(Clone)]
pub struct Session {
    pub name: String,
    pub token: String,
    pub uuid: String,
    pub expires: Value,
    pub generation: String,
}
impl App {
    pub fn auth_status(&self) -> Value {
        match self.session.lock() {
            Ok(session) => session
                .as_ref()
                .map(|s| json!({"signedIn":true,"name":s.name,"expires":s.expires}))
                .unwrap_or(json!({"signedIn":false})),
            Err(_) => json!({"signedIn":false}),
        }
    }
    pub fn twitchland(&self, method: &str, p: &Value) -> Result<Value> {
        match method {
            "status" => Ok(self.auth_status()),
            "signout" => {
                *self.session.lock().map_err(err)? = None;
                self.selected.store(false, Ordering::Relaxed);
                Ok(self.auth_status())
            }
            "signin" => {
                let login = text(p, "login").trim();
                let password = text(p, "password");
                let code = text(p, "code").trim();
                if login.is_empty()
                    || password.is_empty()
                    || login.len() > 1024
                    || password.len() > 4096
                    || code.len() > 64
                {
                    return Err("Введите логин и пароль".into());
                }
                let hwid = hardware_id()?;
                let response=self.auth_http.post("https://tw1tchland.ru/api/v1/integrations/auth/signin").header("X-HWID",hwid).json(&json!({"Login":login,"Password":password,"TwoFactorCode":code,"twoFactorCode":code,"otp_code":code,"AccessToken":""})).send().map_err(|_|"Не удалось связаться с TwitchLand")?;
                let status = response.status();
                let body: Value = response
                    .json()
                    .map_err(|_| "TwitchLand вернул некорректный ответ")?;
                let data = body
                    .get("Data")
                    .or_else(|| body.get("data"))
                    .unwrap_or(&Value::Null);
                let message = body
                    .get("Message")
                    .or_else(|| body.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Ошибка входа");
                let needs = data
                    .get("Has2Fa")
                    .or_else(|| data.get("has2Fa"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    || (!status.is_success() && message.to_lowercase().contains("2fa"));
                if needs && code.is_empty() {
                    return Ok(json!({"signedIn":false,"needs2fa":true}));
                }
                if !status.is_success() {
                    return Err(message.chars().take(300).collect());
                }
                self.select_offline(None)?;
                *self.session.lock().map_err(err)? =
                    Some(parse_session(data, uuid::Uuid::new_v4().to_string())?);
                self.selected.store(true, Ordering::Relaxed);
                Ok(self.auth_status())
            }
            _ => Err("Неизвестное действие TwitchLand".into()),
        }
    }
    fn credentials(&self) -> Result<Value> {
        let previous = self
            .session
            .lock()
            .map_err(err)?
            .clone()
            .ok_or("Войди в TwitchLand снова")?;
        let response = self
            .auth_http
            .post("https://tw1tchland.ru/api/v1/integrations/auth/checkToken")
            .json(&json!({"AccessToken":previous.token}))
            .send()
            .map_err(|_| "Не удалось проверить сессию TwitchLand")?;
        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 401 || status.as_u16() == 403 {
                let mut session = self.session.lock().map_err(err)?;
                if session
                    .as_ref()
                    .is_some_and(|s| s.generation == previous.generation)
                {
                    *session = None;
                    self.selected.store(false, Ordering::Relaxed);
                }
                return Err("Сессия TwitchLand истекла. Войди снова".into());
            }
            return Err("Не удалось проверить сессию TwitchLand".into());
        }
        let body: Value = response
            .json()
            .map_err(|_| "Некорректный ответ TwitchLand")?;
        let data = body
            .get("Data")
            .or_else(|| body.get("data"))
            .unwrap_or(&Value::Null);
        let updated = parse_session(data, previous.generation.clone())?;
        let credentials = json!({"name":updated.name,"token":updated.token,"uuid":updated.uuid});
        let mut session = self.session.lock().map_err(err)?;
        if !session
            .as_ref()
            .is_some_and(|s| s.generation == previous.generation)
        {
            return Err("Сессия изменилась. Повтори запуск".into());
        }
        *session = Some(updated);
        Ok(credentials)
    }
    pub fn engine(&self, method: &str, p: Value) -> Result<Value> {
        let _launch_guard = if method == "launch" {
            Some(self.filesystem_gate.lock().map_err(err)?)
        } else {
            None
        };
        let mut result = self.engine_inner(method, p.clone())?;
        if method == "launch" {
            self.record_launch(&p, &result)?;
        }
        if method == "state" {
            self.reconcile_history(&result)?;
            self.merge_queue(&mut result)?;
        }
        Ok(result)
    }
    fn engine_inner(&self, method: &str, mut p: Value) -> Result<Value> {
        if method == "accounts.offline.add" {
            return self.add_offline(text(&p, "name"));
        }
        if method == "accounts.default" && text(&p, "id").starts_with("offline:") {
            self.select_offline(Some(text(&p, "id")))?;
            self.selected.store(false, Ordering::Relaxed);
            return Ok(json!({"ok":true}));
        }
        if method == "accounts.remove" && text(&p, "id").starts_with("offline:") {
            return self.remove_offline(text(&p, "id"));
        }
        if method == "accounts.default" && p["id"] == "twitchland" {
            if self.auth_status()["signedIn"] != true {
                return Err("Войди в TwitchLand снова".into());
            }
            self.select_offline(None)?;
            self.selected.store(true, Ordering::Relaxed);
            return Ok(json!({"ok":true}));
        }
        if method == "accounts.remove" && p["id"] == "twitchland" {
            self.twitchland("signout", &json!({}))?;
            return Ok(json!({"ok":true}));
        }
        if method == "launch" {
            if let Some(name) = self.offline_name()? {
                p.as_object_mut()
                    .ok_or("Некорректная команда")?
                    .insert("offline".into(), json!({"name":name}));
                return self.native.call(method, p);
            }
        }
        if method == "launch" && self.selected.load(Ordering::Relaxed) {
            let file = self
                .native
                .binary
                .parent()
                .ok_or("Prism не найден")?
                .join("authlib-injector-1.2.7.jar");
            crate::storage::regular(&file)?;
            let bytes =
                std::fs::read(file).map_err(|_| "Компонент авторизации TwitchLand не найден")?;
            if format!("{:x}", Sha256::digest(&bytes))
                != "eaf14bc5acffc7d885bd5bd5942b99f36d6299302beae356b2fc5807fe42652b"
            {
                return Err("Компонент авторизации TwitchLand повреждён".into());
            }
            p.as_object_mut()
                .ok_or("Некорректная команда")?
                .insert("twitchland".into(), self.credentials()?);
        }
        let mut result = self.native.call(method, p)?;
        if method == "accounts.default" {
            self.select_offline(None)?;
            self.selected.store(false, Ordering::Relaxed);
        }
        if method == "state" {
            let status = self.auth_status();
            if status["signedIn"] != true {
                self.selected.store(false, Ordering::Relaxed)
            }
            let offline = self.offline.lock().map_err(err)?.clone();
            let selected = self.selected.load(Ordering::Relaxed) && offline.selected.is_none();
            if let Some(accounts) = result["accounts"].as_array_mut() {
                if selected || offline.selected.is_some() {
                    for account in accounts.iter_mut() {
                        account["default"] = json!(false)
                    }
                }
                if status["signedIn"] == true {
                    accounts.push(json!({"id":"twitchland","name":status["name"],"default":selected,"ownsMinecraft":false,"hasProfile":true,"type":"twitchland"}));
                }
                for profile in offline.profiles {
                    accounts.push(json!({"id":profile.id,"name":profile.name,"default":offline.selected.as_ref()==Some(&profile.id),"ownsMinecraft":false,"hasProfile":true,"type":"offline"}));
                }
            }
        }
        Ok(result)
    }
}
fn parse_session(data: &Value, generation: String) -> Result<Session> {
    let get = |a: &str, b: &str| {
        data.get(a)
            .or_else(|| data.get(b))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let session = Session {
        name: get("Name", "name"),
        token: get("AccessToken", "accessToken"),
        uuid: get("Uuid", "uuid"),
        expires: data
            .get("ExpiredDate")
            .or_else(|| data.get("expiredDate"))
            .cloned()
            .unwrap_or(Value::Null),
        generation,
    };
    if session.name.is_empty() || session.token.is_empty() || session.uuid.is_empty() {
        return Err("TwitchLand не вернул данные аккаунта".into());
    }
    Ok(session)
}
fn hardware_id() -> Result<String> {
    let connection =
        wmi::WMIConnection::new().map_err(|_| "Не удалось определить данные устройства")?;
    let query = |class: &str, field: &str| -> Result<Vec<Option<String>>> {
        let rows: Vec<std::collections::HashMap<String, wmi::Variant>> = connection
            .raw_query(format!("SELECT {field} FROM {class}"))
            .map_err(|_| "Не удалось прочитать данные устройства")?;
        Ok(rows
            .iter()
            .map(|row| match row.get(field) {
                Some(wmi::Variant::String(value)) => Some(value.clone()),
                _ => None,
            })
            .collect())
    };
    let cpu = query("Win32_Processor", "ProcessorId")?;
    let board = query("Win32_BaseBoard", "SerialNumber")?;
    let disks = query("Win32_DiskDrive", "SerialNumber")?;
    Ok(format!(
        "{}-{}-{}",
        cpu.first()
            .and_then(Option::as_deref)
            .unwrap_or("NOT_FOUND"),
        board
            .first()
            .and_then(Option::as_deref)
            .unwrap_or("NOT_FOUND"),
        disks
            .into_iter()
            .map(Option::unwrap_or_default)
            .collect::<Vec<_>>()
            .join("-")
    )
    .trim()
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_is_private() {
        let value =
            json!({"Name":"User","AccessToken":"secret","Uuid":"profile","ExpiredDate":"later"});
        let parsed = parse_session(&value, "generation".into()).unwrap();
        assert_eq!(parsed.name, "User");
        assert_eq!(parsed.token, "secret");
        assert!(parse_session(&json!({"Name":"Missing token"}), "g".into()).is_err());
    }
    #[test]
    fn native_device_id_preserves_existing_format() {
        use std::os::windows::process::CommandExt;
        let script="$p=Get-CimInstance Win32_Processor | Select-Object -First 1; $b=Get-CimInstance Win32_BaseBoard | Select-Object -First 1; $d=@(Get-CimInstance Win32_DiskDrive | ForEach-Object { [string]$_.SerialNumber }); if ($null -eq $p.ProcessorId) { $cpu='NOT_FOUND' } else { $cpu=[string]$p.ProcessorId }; if ($null -eq $b.SerialNumber) { $board='NOT_FOUND' } else { $board=[string]$b.SerialNumber }; [Console]::Out.Write($cpu+'-'+$board+'-'+($d -join '-'))";
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(output.status.success());
        let actual = hardware_id().unwrap();
        let expected = String::from_utf8_lossy(&output.stdout);
        assert!(
            actual == expected.trim(),
            "Native device format differs from previous format"
        );
    }
}
