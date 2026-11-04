use crate::{err, storage, text, App, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{IpAddr, TcpStream, ToSocketAddrs},
    time::{Duration, Instant},
};
pub struct Address {
    pub host: String,
    pub port: u16,
    pub explicit: bool,
    pub value: String,
}
pub fn address(value: &str) -> Result<Address> {
    if value.is_empty()
        || value.len() > 255
        || value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "/\\?#@".contains(c))
    {
        return Err("Укажи адрес сервера без http://".into());
    }
    let (host, port) = if value.starts_with('[') {
        let end = value.find(']').ok_or("Некорректный IPv6")?;
        let host = &value[1..end];
        if host.parse::<std::net::Ipv6Addr>().is_err() {
            return Err("Некорректный IPv6".into());
        }
        let suffix = &value[end + 1..];
        (
            host,
            if suffix.is_empty() {
                None
            } else {
                Some(suffix.strip_prefix(':').ok_or("Некорректный порт")?)
            },
        )
    } else {
        match value.split_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (value, None),
        }
    };
    let host = if let Ok(ip) = host.parse::<IpAddr>() {
        ip.to_string()
    } else {
        let url = reqwest::Url::parse(&format!("http://{host}")).map_err(err)?;
        let normalized = url.host_str().ok_or("Некорректный адрес")?.to_lowercase();
        if normalized.split('.').any(|p| {
            p.is_empty()
                || p.len() > 63
                || p.starts_with('-')
                || p.ends_with('-')
                || !p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        }) {
            return Err("Некорректный адрес".into());
        }
        normalized
    };
    let port_num = match port {
        Some(v) => {
            if v.is_empty() || !v.bytes().all(|c| c.is_ascii_digit()) {
                return Err("Некорректный порт".into());
            }
            let p = v.parse::<u16>().map_err(|_| "Некорректный порт")?;
            if p == 0 {
                return Err("Некорректный порт".into());
            }
            p
        }
        None => 25565,
    };
    let rendered = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.clone()
    };
    Ok(Address {
        host,
        port: port_num,
        explicit: port.is_some(),
        value: if port.is_some() {
            format!("{rendered}:{port_num}")
        } else {
            rendered
        },
    })
}
fn varint(mut n: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let b = (n & 127) as u8;
        n >>= 7;
        bytes.push(if n > 0 { b | 128 } else { b });
        if n == 0 {
            break;
        }
    }
    bytes
}
fn read_varint(reader: &mut impl Read) -> Result<u32> {
    let mut n = 0;
    for i in 0..5 {
        let mut byte = [0];
        reader
            .read_exact(&mut byte)
            .map_err(|_| "Сервер не ответил")?;
        n |= ((byte[0] & 127) as u32) << (i * 7);
        if byte[0] & 128 == 0 {
            return Ok(n);
        }
    }
    Err("Некорректный ответ сервера".into())
}
fn frame(bytes: Vec<u8>) -> Vec<u8> {
    [varint(bytes.len() as u32), bytes].concat()
}
fn motd(value: &Value, depth: u8) -> String {
    if depth > 12 {
        return String::new();
    }
    match value {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(|v| motd(v, depth + 1)).collect(),
        Value::Object(_) => format!(
            "{}{}",
            text(value, "text"),
            motd(&value["extra"], depth + 1)
        ),
        _ => String::new(),
    }
}
pub fn ping(value: &str) -> Result<Value> {
    let target = address(value)?;
    let mut host = target.host.clone();
    let mut port = target.port;
    if !target.explicit && host.parse::<IpAddr>().is_err() {
        if let Ok((config, mut opts)) = hickory_resolver::system_conf::read_system_conf() {
            opts.timeout = Duration::from_millis(1500);
            opts.attempts = 1;
            if let Ok(resolver) = hickory_resolver::Resolver::new(config, opts) {
                if let Ok(records) = resolver.srv_lookup(format!("_minecraft._tcp.{host}")) {
                    if let Some(record) = records.iter().min_by_key(|r| r.priority()) {
                        if record.target().to_utf8() != "." {
                            host = record.target().to_utf8();
                            port = record.port();
                        }
                    }
                }
            }
        }
    }
    let started = Instant::now();
    let addresses = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| "Не удалось определить адрес сервера")?;
    let mut stream = None;
    for addr in addresses.take(4) {
        if let Ok(s) = TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
            stream = Some(s);
            break;
        }
    }
    let mut stream = stream.ok_or("Сервер недоступен")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(err)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(err)?;
    let name = target.host.as_bytes();
    let mut handshake = vec![0];
    handshake.extend(varint(u32::MAX));
    handshake.extend(varint(name.len() as u32));
    handshake.extend(name);
    handshake.extend(target.port.to_be_bytes());
    handshake.push(1);
    stream
        .write_all(&[frame(handshake), vec![1, 0]].concat())
        .map_err(err)?;
    let length = read_varint(&mut stream)?;
    if length > 524288 || length < 2 {
        return Err("Некорректный ответ сервера".into());
    }
    let mut packet = vec![0; length as usize];
    stream
        .read_exact(&mut packet)
        .map_err(|_| "Сервер не ответил")?;
    let mut reader = std::io::Cursor::new(&packet);
    if read_varint(&mut reader)? != 0 {
        return Err("Некорректный ответ сервера".into());
    }
    let jsonlen = read_varint(&mut reader)?;
    if reader.position() + jsonlen as u64 != length as u64 {
        return Err("Некорректный ответ сервера".into());
    }
    let data: Value = serde_json::from_slice(&packet[reader.position() as usize..])
        .map_err(|_| "Некорректный ответ сервера")?;
    let mut chars = motd(&data["description"], 0)
        .chars()
        .collect::<Vec<_>>()
        .into_iter();
    let mut plain = String::new();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            plain.push(c)
        }
    }
    Ok(
        json!({"online":true,"players":data["players"]["online"].as_u64(),"max":data["players"]["max"].as_u64(),"version":text(&data["version"],"name").chars().take(80).collect::<String>(),"motd":plain.chars().take(400).collect::<String>(),"latency":started.elapsed().as_millis()}),
    )
}
impl App {
    fn server_list(&self) -> Result<Vec<Value>> {
        let file = self.data.join("graphite-servers.json");
        if !file.exists() {
            return Ok(Vec::new());
        }
        let value: Vec<Value> = serde_json::from_str(&storage::read(&file, 256000)?)
            .map_err(|_| "Повреждён список серверов")?;
        if value.len() > 100 {
            return Err("Слишком много серверов".into());
        }
        for item in &value {
            if !item["id"].is_string()
                || !item["name"].is_string()
                || !item["instanceId"].is_string()
            {
                return Err("Повреждён список серверов".into());
            }
            address(text(item, "address"))?;
        }
        Ok(value)
    }
    fn server_status_cache(&self) -> Value {
        storage::read(&self.data.join("graphite-server-status.json"), 256000)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}))
    }
    fn cached_server_statuses(&self) -> Result<Value> {
        let cache = self.server_status_cache();
        let mut result = json!({});
        for item in self.server_list()? {
            let entry = &cache[text(&item, "id")];
            if entry["address"] == item["address"]
                && entry["online"].is_boolean()
                && entry["checkedAt"].is_i64()
            {
                result[text(&item, "id")] = entry.clone();
            }
        }
        Ok(result)
    }
    fn cache_server_status(&self, item: &Value, status: &Value) -> Result<()> {
        let _lock = self.servers_lock.lock().map_err(err)?;
        let current = self.server_list()?;
        // A late ping must not repopulate a deleted server or an address edited while it was running.
        if !current
            .iter()
            .any(|s| s["id"] == item["id"] && s["address"] == item["address"])
        {
            return Ok(());
        }
        let mut cache = self.cached_server_statuses()?;
        cache[text(item, "id")] = status.clone();
        let temp = crate::catalog::Temporary(
            self.data
                .join(format!(".server-status-{}.tmp", uuid::Uuid::new_v4())),
        );
        storage::write_new(&temp.0, &serde_json::to_vec_pretty(&cache).map_err(err)?)?;
        fs::rename(&temp.0, self.data.join("graphite-server-status.json")).map_err(err)
    }
    pub fn servers(&self, method: &str, p: &Value) -> Result<Value> {
        if method == "list" {
            return Ok(json!(self.server_list()?));
        }
        if method == "cached-status" {
            return self.cached_server_statuses();
        }
        if method == "save" || method == "remove" {
            let _lock = self.servers_lock.lock().map_err(err)?;
            let mut items = self.server_list()?;
            let id = text(p, "id");
            if !id.is_empty() && !items.iter().any(|s| s["id"] == id) {
                return Err("Сервер не найден".into());
            }
            if method == "remove" {
                items.retain(|s| s["id"] != id)
            } else {
                let name = text(p, "name").trim();
                if name.is_empty()
                    || name.chars().count() > 80
                    || name.chars().any(char::is_control)
                {
                    return Err("Укажи название сервера".into());
                }
                let addr = address(text(p, "address"))?;
                let library = self.library()?;
                if !text(p, "instanceId").is_empty()
                    && !library
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|i| i["id"] == p["instanceId"] && i["error"] != true)
                {
                    return Err("Выбери установленную сборку".into());
                }
                let minecraft = text(p, "minecraft").trim();
                if minecraft.len() > 80 || minecraft.chars().any(char::is_control) {
                    return Err("Недопустимая версия Minecraft".into());
                }
                let project = text(p, "project").trim();
                if !project.is_empty() {
                    crate::catalog::identifier(project)?;
                }
                let key = if id.is_empty() {
                    uuid::Uuid::new_v4().to_string()
                } else {
                    id.into()
                };
                let item = json!({"id":key,"name":name,"address":addr.value,"instanceId":text(p,"instanceId"),"minecraft":minecraft,"project":project});
                if let Some(old) = items.iter_mut().find(|s| s["id"] == id) {
                    *old = item
                } else {
                    if items.len() >= 100 {
                        return Err("Можно сохранить до 100 серверов".into());
                    }
                    items.push(item);
                }
            }
            let temp = crate::catalog::Temporary(
                self.data
                    .join(format!(".servers-{}.tmp", uuid::Uuid::new_v4())),
            );
            storage::write_new(
                &temp.0,
                serde_json::to_vec_pretty(&items).map_err(err)?.as_slice(),
            )?;
            fs::rename(&temp.0, self.data.join("graphite-servers.json")).map_err(err)?;
            return Ok(json!(items));
        }
        let items = self.server_list()?;
        let item = items
            .iter()
            .find(|v| v["id"] == p["id"])
            .ok_or("Сервер не найден")?;
        match method {
            "prepare" => {
                let minecraft = text(item, "minecraft");
                let project = text(item, "project");
                if project.is_empty() {
                    return Err("Укажи ID или slug сборки Modrinth в настройках сервера".into());
                }
                let versions = self.modrinth(
                    "versions",
                    &json!({"kind":"modpack","projectId":project,"minecraft":minecraft}),
                )?;
                let version = versions
                    .as_array()
                    .and_then(|a| a.iter().find(|v| v["type"] == "release"))
                    .ok_or("Не найдена подходящая версия сборки")?;
                let before = self.library()?;
                let info = self.api(
                    &format!("project/{}", crate::catalog::identifier(project)?),
                    vec![],
                )?;
                let started=self.modrinth("install",&json!({"kind":"modpack","projectId":info["id"],"versionId":version["id"],"name":format!("{} · сервер",text(item,"name"))}))?;
                Ok(
                    json!({"job":started["job"],"serverId":item["id"],"before":before.as_array().unwrap().iter().map(|i|i["id"].clone()).collect::<Vec<_>>()}),
                )
            }
            "status" => {
                let mut status = ping(text(item, "address"))
                    .unwrap_or_else(|error| json!({"online":false,"error":error}));
                status["checkedAt"] = json!(chrono::Utc::now().timestamp_millis());
                status["address"] = item["address"].clone();
                if self.cache_server_status(item, &status).is_err() {
                    status["cacheWarning"] =
                        json!("Статус получен, но не удалось сохранить его на диске.");
                }
                Ok(status)
            }
            "launch" => {
                let id = text(item, "instanceId");
                self.instance(id)?;
                self.check_busy(id)?;
                if !text(item, "minecraft").is_empty() {
                    let lib = self.library()?;
                    if lib
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|i| i["id"] == id && i["version"] != item["minecraft"])
                    {
                        return Err("Версия сборки отличается от указанной версии сервера. Выбери подходящую сборку".into());
                    }
                }
                self.engine(
                    "launch",
                    json!({"id":id,"server":address(text(item,"address"))?.value}),
                )
            }
            _ => Err("Неизвестное действие сервера".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_protocol() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 512];
            stream.read(&mut request).unwrap();
            let data=json!({"version":{"name":"1.20.1"},"players":{"online":7,"max":20},"description":{"text":"§aHello ","extra":[{"text":"World"}]}}).to_string().into_bytes();
            let packet = frame([vec![0], varint(data.len() as u32), data].concat());
            stream.write_all(&packet[..2]).unwrap();
            std::thread::sleep(Duration::from_millis(10));
            stream.write_all(&packet[2..]).unwrap();
        });
        let response = ping(&addr.to_string()).unwrap();
        assert_eq!(response["players"], 7);
        assert_eq!(response["motd"], "Hello World");
        assert_eq!(response["version"], "1.20.1");
        worker.join().unwrap();
        assert!(ping(&addr.to_string()).is_err());
    }
}
