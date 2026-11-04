use crate::{err, text, App, Result};
use serde_json::{json, Value};
use std::{
    fs, io::{BufRead, BufReader, Read, Write}, net::{TcpListener, TcpStream},
    os::windows::process::CommandExt, path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio}, sync::{Arc, Mutex}, time::Duration,
};

const APP_ID: &str = "4963920";
struct Session { child: Child, input: ChildStdin, local_port: Option<u16> }
pub struct Porthole {
    binary: Mutex<Option<PathBuf>>,
    session: Mutex<Option<Session>>,
    state: Arc<Mutex<Value>>,
}

fn hidden(binary: &Path) -> Command {
    let mut cmd = Command::new(binary);
    cmd.creation_flags(0x08000000);
    cmd
}
fn valid_binary(path: &Path) -> bool {
    path.is_file() && path.file_name().is_some_and(|n|
        ["porthole.exe", "porthole-gnu.exe"].contains(&n.to_string_lossy().to_ascii_lowercase().as_str()))
}
fn find_binary(data: &Path) -> Option<PathBuf> {
    if let Ok(bytes) = fs::read(data.join("graphite-porthole.json")) {
        if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
            let p = PathBuf::from(text(&v, "path"));
            if valid_binary(&p) { return Some(p) }
        }
    }
    let mut roots = vec![PathBuf::from(r"C:\Program Files (x86)\Steam")];
    if let Ok(out) = hidden(Path::new("reg.exe")).args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]).output() {
        if let Some(path) = String::from_utf8_lossy(&out.stdout).lines()
            .find_map(|line| line.split_once("REG_SZ").map(|(_,p)| p.trim().to_string())) {
            roots.insert(0, PathBuf::from(path));
        }
    }
    let mut libraries = roots.clone();
    for root in roots {
        if let Ok(vdf) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            for line in vdf.lines().filter(|line| line.trim_start().starts_with("\"path\"")) {
                if let Some(p) = line.split('"').nth(3) {
                    libraries.push(PathBuf::from(p.replace("\\\\", "\\")));
                }
            }
        }
    }
    for root in libraries {
        if let Ok(manifest) = fs::read_to_string(root.join(format!("steamapps/appmanifest_{APP_ID}.acf"))) {
            if let Some(dir) = manifest.lines().filter(|l| l.trim_start().starts_with("\"installdir\""))
                .find_map(|l| l.split('"').nth(3)) {
                if !dir.is_empty() && !dir.contains(['/', '\\', ':']) && dir != "." && dir != ".." {
                    let p = root.join("steamapps/common").join(dir).join("porthole.exe");
                    if valid_binary(&p) { return Some(p) }
                }
            }
        }
    }
    None
}

fn port(p: &Value) -> Result<u16> {
    p["port"].as_u64().and_then(|p| u16::try_from(p).ok()).filter(|p| *p > 0)
        .ok_or("Укажи порт от 1 до 65535".into())
}
fn target(value: &str) -> Result<String> {
    let s = value.trim();
    let valid = if let Some(id) = s.strip_prefix("lobby:").or_else(|| s.strip_prefix("peer:")) {
        id.len() == 17 && id.bytes().all(|b| b.is_ascii_digit()) && id.parse::<u64>().is_ok()
    } else { (6..=128).contains(&s.len()) && s.as_bytes()[0].is_ascii_alphanumeric() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') };
    if !valid { return Err("Вставь код комнаты Porthole или адрес lobby:ID / peer:ID".into()) }
    Ok(s.to_string())
}

// Confirm that the local listener belongs to this Porthole process, not an unrelated service.
#[link(name = "iphlpapi")]
extern "system" {
    fn GetExtendedTcpTable(table: *mut std::ffi::c_void, size: *mut u32, sorted: i32, family: u32, class: u32, reserved: u32) -> u32;
}
fn listeners() -> Vec<(u16, u32, u32)> {
    let mut size = 0u32;
    unsafe { GetExtendedTcpTable(std::ptr::null_mut(), &mut size, 0, 2, 3, 0); }
    if size == 0 || size > 4_000_000 { return vec![] }
    let mut table = vec![0u32; (size as usize + 3) / 4];
    let result = unsafe { GetExtendedTcpTable(table.as_mut_ptr().cast(), &mut size, 0, 2, 3, 0) };
    if result != 0 { return vec![] }
    table[1..].chunks_exact(6).take(table[0] as usize)
        .map(|row| (u16::from_be(row[2] as u16), row[5], row[1])).collect()
}
fn apply_event(state: &mut Value, e: Value) {
    match text(&e, "event") {
        "steam_id" => state["steamId"] = e["steam_id"].clone(),
        "static_code" | "share_code" => state["code"] = e["code"].clone(),
        "lobby_ready" | "server_ready" => {
            state["phase"] = json!("hosting");
            if e["code"].is_string() { state["code"] = e["code"].clone() }
            state["lobby"] = e["lobby_id"].clone();
        },
        "roster" => state["members"] = e["members"].clone(),
        "member" => {
            let id = e["steam_id"].clone();
            if let Some(members) = state["members"].as_array_mut() {
                members.retain(|m| m != &id);
                if e["present"] == true { members.push(id); }
            }
        },
        "ready" if state["mode"] == "join" => {
            state["linked"] = json!(true);
            state["phase"] = json!("connecting");
            state["error"] = Value::Null;
        },
        "port_offered" if e["proto"] == "tcp" => state["offeredPort"] = e["port"].clone(),
        "link" => {
            if e["ping_ms"].as_i64().is_some_and(|v| v >= 0) {
                state["ping"] = e["ping_ms"].clone();
                state["relayed"] = e["relayed"].clone();
            }
        },
        "denied" => {
            state["linked"] = json!(false);
            state["phase"] = json!("error");
            state["error"] = json!("Хозяин комнаты отклонил соединение. Проверь код, пароль и доступ в Porthole.");
        },
        "error" => {
            state["linked"] = json!(false);
            state["phase"] = json!("error");
            state["error"] = e["message"].as_str().map(|s| json!(s.chars().take(1000).collect::<String>())).unwrap_or(json!("Ошибка Porthole"));
        },
        "info" => {
            if text(&e, "message").contains("link is down") {
                state["linked"] = json!(false);
                state["ping"] = Value::Null;
                state["phase"] = json!("connecting");
                state["address"] = Value::Null;
            }
        },
        _ => {},
    }
    if !["tick", "link"].contains(&text(&e, "event")) {
        if let Some(log) = state["events"].as_array_mut() {
            log.push(e);
            if log.len() > 24 { log.remove(0); }
        }
    }
}

impl Porthole {
    pub fn new(data: &Path) -> Self {
        Self { binary: Mutex::new(find_binary(data)), session: Mutex::new(None),
            state: Arc::new(Mutex::new(json!({"active":false,"phase":"idle","events":[]}))) }
    }
    fn snapshot(&self) -> Result<Value> {
        let mut session = self.session.lock().map_err(err)?;
        let mut state = self.state.lock().map_err(err)?;
        if let Some(s) = session.as_mut() {
            if let Some(exit) = s.child.try_wait().map_err(err)? {
                state["active"] = json!(false);
                state["address"] = Value::Null;
                state["phase"] = json!("error");
                state["generation"] = json!(uuid::Uuid::new_v4().to_string());
                if text(&state, "error").is_empty() {
                    state["error"] = json!(format!("Porthole завершился ({exit}). Открой Steam и проверь вход в аккаунт."));
                }
                *session = None;
            } else if let Some(port) = s.local_port {
                if let Some((_,_,address)) = listeners().into_iter().find(|(p,pid,_)| *p == port && *pid == s.child.id()) {
                    if address != u32::from_ne_bytes([127,0,0,1]) {
                        let _ = s.child.kill();
                        return Err("В настройках Porthole выбери адрес привязки 127.0.0.1".into())
                    }
                    if state["linked"] == true && text(&state,"phase") != "error" {
                        state["phase"] = json!("connected");
                        state["address"] = json!(format!("127.0.0.1:{port}"));
                    }
                } else {
                    state["address"] = Value::Null;
                    if text(&state,"phase") == "connected" { state["phase"] = json!("connecting") }
                }
            }
        }
        let path = self.binary.lock().map_err(err)?;
        state["installed"] = json!(path.as_ref().is_some_and(|p| valid_binary(p)));
        state["path"] = json!(path.as_ref().map(|p| p.to_string_lossy().to_string()));
        Ok(state.clone())
    }
    fn start(&self, mode: &str, p: &Value) -> Result<Value> {
        let number = port(p)?;
        let target = if mode == "join" { Some(target(text(p,"target"))?) } else { None };
        let password = text(p,"password");
        if password.len() > 128 || password.contains(['\n','\r','\0']) { return Err("Пароль слишком длинный или содержит перенос строки".into()) }
        let binary = self.binary.lock().map_err(err)?.clone().filter(|p| valid_binary(p))
            .ok_or("Установи Porthole из Steam или укажи путь к porthole.exe".to_string())?;
        let mut session = self.session.lock().map_err(err)?;
        if session.is_some() { return Err("Сначала закрой текущее соединение".into()) }
        let mut cmd = hidden(&binary);
        cmd.current_dir(binary.parent().ok_or("Нет папки Porthole")?).arg("--json");
        let mut local_port = None;
        if let Some(target) = target {
            let socket = TcpListener::bind(("127.0.0.1",0)).map_err(err)?;
            let local = socket.local_addr().map_err(err)?.port();
            local_port = Some(local);
            cmd.args(["connect",&target,"--auto-approve-ports",&format!("tcp/{number}"),"--remap",&format!("{number}:{local}")]);
        } else {
            TcpStream::connect_timeout(&format!("127.0.0.1:{number}").parse().map_err(err)?,Duration::from_millis(700))
                .map_err(|_| "На этом порту нет запущенного мира. Открой Minecraft для сети и укажи его порт.".to_string())?;
            cmd.args(["expose",&format!("tcp/127.0.0.1:{number}"),"--visibility","private","--game-name","Minecraft · Graphite"]);
        }
        if !password.is_empty() { cmd.args(["--password", password]); }
        let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(err)?;
        let input = child.stdin.take().ok_or("Не открыт канал управления Porthole")?;
        let stdout = child.stdout.take().ok_or("Не открыт вывод Porthole")?;
        let stderr = child.stderr.take().ok_or("Не открыт журнал Porthole")?;
        let generation = uuid::Uuid::new_v4().to_string();
        *self.state.lock().map_err(err)? = json!({"generation":generation,"active":true,"phase":if mode=="host"{"starting"}else{"connecting"},"mode":mode,"port":number,"events":[],"members":[]});
        *session = Some(Session{child,input,local_port});
        for stream in [Box::new(stdout) as Box<dyn Read+Send>, Box::new(stderr) as Box<dyn Read+Send>] {
            let state = self.state.clone();
            let generation = generation.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream);
                loop {
                    let mut line = Vec::new();
                    match reader.by_ref().take(65537).read_until(b'\n', &mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) if line.len() > 65536 => break,
                        Ok(_) => {},
                    }
                    if let Ok(e) = serde_json::from_slice::<Value>(&line) {
                        if !e.is_object() { continue }
                        if let Ok(mut s) = state.lock() {
                            if s["generation"] != generation { break }
                            apply_event(&mut s,e);
                        }
                    }
                }
            });
        }
        drop(session);
        self.snapshot()
    }
    pub fn stop(&self) {
        if let Ok(mut session) = self.session.lock() {
            if let Some(mut s) = session.take() {
                if let Ok(mut state) = self.state.lock() {
                    *state = json!({"active":false,"phase":"idle","events":[]});
                }
                let _ = s.input.write_all(b"quit\n");
                let _ = s.input.flush();
                for _ in 0..20 {
                    if s.child.try_wait().ok().flatten().is_some() { return }
                    std::thread::sleep(Duration::from_millis(25));
                }
                let _ = s.child.kill();
                let _ = s.child.wait();
            }
        }
    }
}
impl Drop for Porthole { fn drop(&mut self) { self.stop() } }

impl App {
    pub fn porthole(&self, method: &str, p: &Value) -> Result<Value> {
        match method {
            "status" => self.porthole.snapshot(),
            "refresh" => {
                *self.porthole.binary.lock().map_err(err)? = find_binary(&self.data);
                self.porthole.snapshot()
            },
            "pick" => {
                if let Some(path) = rfd::FileDialog::new().set_title("Porthole").add_filter("Porthole", &["exe"]).pick_file() {
                    if !valid_binary(&path) { return Err("Выбери porthole.exe из папки Porthole".into()) }
                    fs::write(self.data.join("graphite-porthole.json"), serde_json::to_vec(&json!({"path":path})).map_err(err)?).map_err(err)?;
                    *self.porthole.binary.lock().map_err(err)? = Some(path);
                }
                self.porthole.snapshot()
            },
            "install" | "open" | "steam" => {
                open::that(match method {"install"=>"steam://install/4963920","open"=>"steam://run/4963920",_=>"steam://open/main"}).map_err(err)?;
                Ok(Value::Null)
            },
            "host" | "join" => self.porthole.start(method,p),
            "stop" => { self.porthole.stop(); self.porthole.snapshot() },
            "copy" => {
                let s = self.porthole.snapshot()?;
                let value = text(&s, if text(p,"kind")=="address" {"address"} else {"code"});
                if value.is_empty() { return Err("Соединение ещё не готово".into()) }
                crate::clipboard(value)?;
                Ok(Value::Null)
            },
            "detect" => {
                let id = text(p,"id");
                self.instance(id)?;
                let log = self.engine("instance.log",json!({"id":id}))?;
                let ports = listeners();
                let found = text(&log,"text").lines().rev().filter(|line| {
                    line.contains("Started serving on") || line.contains("Local game hosted on port")
                }).find_map(|line| line.split(|c:char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).next_back()
                    .and_then(|s| s.parse::<u16>().ok()).filter(|p| ports.iter().any(|(port,_,_)| port == p)));
                found.map(|port| json!({"port":port})).ok_or("Открой мир для сети в Minecraft. Порт также можно ввести вручную.".into())
            },
            "launch" => {
                let s = self.porthole.snapshot()?;
                if s["active"] != true || s["phase"] != "connected" { return Err("Сначала дождись подключения к комнате".into()) }
                let id = text(p,"id");
                self.instance(id)?; self.check_busy(id)?;
                self.engine("launch",json!({"id":id,"server":text(&s,"address")}))
            },
            _ => Err("Неизвестное действие Porthole".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validate_inputs_and_listener_owner() {
        for bad in ["", "abc\nquit", "--help", "a b", "peer:1", "lobby:../../x", "abc;cmd"] { assert!(target(bad).is_err(),"{bad}") }
        assert!(target("5GA564A").is_ok());
        assert!(target("peer:76561198000000000").is_ok());
        for bad in [json!({"port":0}),json!({"port":65536}),json!({"port":-1}),json!({"port":"25565"})] { assert!(port(&bad).is_err()) }
        let listener = TcpListener::bind(("127.0.0.1",0)).unwrap();
        let number = listener.local_addr().unwrap().port();
        assert!(listeners().iter().any(|(p,pid,_)| *p == number && *pid == std::process::id()));
    }
    #[test]
    fn real_cli_events_are_not_fake_progress() {
        let mut s=json!({"phase":"starting","events":[]});
        apply_event(&mut s,json!({"event":"static_code","code":"5GA564A"}));
        assert_eq!(s["phase"],"starting");
        apply_event(&mut s,json!({"event":"lobby_ready","lobby_id":123}));
        assert_eq!(s["phase"],"hosting"); assert_eq!(s["code"],"5GA564A");
        apply_event(&mut s,json!({"event":"denied","reason":"refused"}));
        assert_eq!(s["phase"],"error");
        s["mode"]=json!("join");
        apply_event(&mut s,json!({"event":"ready"}));
        assert_eq!(s["linked"],true);
        apply_event(&mut s,json!({"event":"info","message":"the link is down; joining again in 2 s"}));
        assert_eq!(s["linked"],false);
        assert_eq!(s["phase"],"connecting");
    }
}
