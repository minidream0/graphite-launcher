use crate::{err, Result};
use serde_json::{json, Value};
use std::os::windows::process::CommandExt;
use std::{
    io,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::ClientOptions,
};
pub struct Native {
    pub binary: PathBuf,
    dir: PathBuf,
    pipe: String,
    child: Mutex<Option<Child>>,
    runtime: tokio::runtime::Runtime,
}
impl Native {
    pub fn new(binary: PathBuf, dir: PathBuf) -> Result<Self> {
        Ok(Self {
            binary,
            dir,
            pipe: format!("graphite-{}", uuid::Uuid::new_v4().simple()),
            child: Mutex::new(None),
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(err)?,
        })
    }
    fn request(&self, method: &str, params: Value) -> Result<Value> {
        self.runtime.block_on(async {
            let result = tokio::time::timeout(Duration::from_secs(65), async {
                let mut pipe = ClientOptions::new().open(format!(r"\\.\pipe\{}", self.pipe))?;
                pipe.write_all(
                    format!("{}\n", json!({"method":method,"params":params})).as_bytes(),
                )
                .await?;
                let mut bytes = Vec::new();
                let mut block = [0u8; 4096];
                loop {
                    let n = pipe.read(&mut block).await?;
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&block[..n]);
                    if bytes.len() > 1_048_576 {
                        return Err(io::Error::other("Ответ движка слишком большой"));
                    }
                    if bytes.contains(&b'\n') {
                        break;
                    }
                }
                Ok::<_, io::Error>(bytes)
            })
            .await
            .map_err(|_| "Движок не ответил за 65 секунд".to_string())?
            .map_err(err)?;
            let response: Value = serde_json::from_slice(&result).map_err(err)?;
            if response["ok"] == true {
                Ok(response["result"].clone())
            } else {
                Err(response["error"].as_str().unwrap_or("Ошибка движка").into())
            }
        })
    }
    pub fn call(&self, method: &str, params: Value) -> Result<Value> {
        {
            let mut child = self.child.lock().map_err(err)?;
            if child
                .as_mut()
                .is_some_and(|c| c.try_wait().ok().flatten().is_some())
            {
                *child = None
            }
            if child.is_none() {
                let process = Command::new(&self.binary)
                    .args([
                        "--dir",
                        self.dir.to_str().ok_or("Некорректный путь данных")?,
                        "--design-service",
                        &self.pipe,
                    ])
                    .current_dir(self.binary.parent().ok_or("Движок не найден")?)
                    .creation_flags(0x08000000)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| format!("Не удалось запустить Prism: {e}"))?;
                *child = Some(process);
                let mut ready = false;
                for _ in 0..100 {
                    if child.as_mut().unwrap().try_wait().map_err(err)?.is_some() {
                        *child = None;
                        return Err(
                            "Prism завершился. Возможно, папка уже открыта другим лаунчером".into(),
                        );
                    }
                    if self.request("state", json!({})).is_ok() {
                        ready = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(150))
                }
                if !ready {
                    if let Some(c) = child.as_mut() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    *child = None;
                    return Err("Не удалось запустить движок".into());
                }
            }
        }
        self.request(method, params)
    }
    pub fn close(&self) {
        if self.child.lock().map(|c| c.is_some()).unwrap_or(false) {
            let active = self.request("state", json!({})).ok().is_some_and(|v| {
                v["jobs"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|j| j["state"] == "running"))
            });
            let _ = self.request("shutdown", json!({"detach":true}));
            if !active {
                if let Ok(mut child) = self.child.lock() {
                    for _ in 0..40 {
                        if child
                            .as_mut()
                            .is_none_or(|c| c.try_wait().ok().flatten().is_some())
                        {
                            *child = None;
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(25));
                    }
                }
            }
        }
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        self.close()
    }
}
