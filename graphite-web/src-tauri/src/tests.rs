use super::*;
use std::{fs, io::Write};
fn zip_file(path: &std::path::Path, name: &str, content: &str) {
    let f = fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(f);
    zip.start_file(name, zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(content.as_bytes()).unwrap();
    zip.finish().unwrap();
}
#[test]
fn validation_and_diff() {
    for id in ["..", "a/b", "a\\b", "a:b", ""] {
        assert!(storage::component(id).is_err())
    }
    for value in [
        "https://example.org",
        "host:0",
        "host:65536",
        "[bad]",
        "host\n",
    ] {
        assert!(servers::address(value).is_err(), "{value}")
    }
    assert_eq!(servers::address("[::1]:25565").unwrap().host, "::1");
    let diff = catalog::compare(
        &json!({"files":[{"path":"old","hashes":{"sha1":"1"}},{"path":"change","hashes":{"sha1":"1"}}]}),
        &json!({"files":[{"path":"new","hashes":{"sha1":"1"}},{"path":"change","hashes":{"sha1":"2"}}]}),
        vec!["old-setting".into()],
        vec!["new-setting".into()],
    );
    assert_eq!(diff["added"], json!(["new"]));
    assert_eq!(diff["removed"], json!(["old"]));
    assert_eq!(diff["changed"], json!(["change"]));
    assert_eq!(diff["removedOverrides"], json!(["old-setting"]));
}
#[test]
fn rust_backend_parity() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("qa-rust-tests");
    fs::create_dir_all(&base).unwrap();
    let root = base.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&root).unwrap();
    let backend = std::env::var_os("GRAPHITE_TEST_BACKEND")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("build-clang/prismlauncher.exe")
        });
    std::env::set_var("GRAPHITE_DATA", &root);
    std::env::set_var("GRAPHITE_BACKEND", backend);
    let app = Arc::new(App::new().unwrap());
    assert_eq!(
        app.tools("appearance.get", &json!({})).unwrap()["theme"],
        "graphite"
    );
    assert!(app
        .tools("appearance.set", &json!({"theme":"unknown"}))
        .is_err());
    app.tools("appearance.set", &json!({"theme":"smoky"}))
        .unwrap();
    assert_eq!(
        app.tools("appearance.get", &json!({})).unwrap()["theme"],
        "smoky"
    );
    assert_eq!(
        serde_json::from_str::<Value>(
            &fs::read_to_string(root.join("graphite-appearance.json")).unwrap()
        )
        .unwrap()["theme"],
        "smoky"
    );
    app.tools("appearance.set", &json!({"theme":"graphite"}))
        .unwrap();
    assert!(app
        .tools("presentation.version", &json!({}))
        .unwrap()
        .is_null());
    assert!(app
        .tools("presentation.version", &json!({}))
        .unwrap()
        .is_null());
    fs::write(root.join("graphite-seen-version.json"), "\"0.9.0\"").unwrap();
    assert_eq!(
        app.tools("presentation.version", &json!({})).unwrap(),
        "0.9.0"
    );
    assert!(app
        .tools("presentation.version", &json!({}))
        .unwrap()
        .is_null());
    let instance = root.join("instances/test");
    fs::create_dir_all(instance.join("minecraft/saves/world")).unwrap();
    fs::write(
        instance.join("instance.cfg"),
        "[General]\nname=Тест\nInstanceType=OneSix\n",
    )
    .unwrap();
    fs::write(instance.join("mmc-pack.json"),json!({"formatVersion":1,"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.fabricmc.fabric-loader","version":"0.16.9"}]}).to_string()).unwrap();
    fs::write(
        instance.join("minecraft/saves/world/level.dat"),
        b"original-world",
    )
    .unwrap();
    let lib = app.library().unwrap();
    assert_eq!(lib[0]["name"], "Тест");
    assert_eq!(lib[0]["loader"], "Fabric");
    assert!(app.instance("../bad").is_err());
    let state = app.engine("state", json!({})).unwrap();
    assert_eq!(state["visibleWindows"], 0);
    assert_eq!(app.preflight("test").unwrap()["errors"], 0);
    let mods = instance.join("minecraft/mods");
    fs::create_dir_all(&mods).unwrap();
    zip_file(
        &mods.join("dependency.jar"),
        "fabric.mod.json",
        r#"{"id":"dep","version":"1.0.0"}"#,
    );
    zip_file(
        &mods.join("consumer.jar"),
        "fabric.mod.json",
        r#"{"id":"consumer","version":"1.0.0","depends":{"dep":">=2.0","minecraft":"1.20.*"}}"#,
    );
    assert!(app.preflight("test").unwrap()["errors"].as_u64().unwrap() > 0);
    assert!(app
        .engine("launch", json!({"id":"test"}))
        .unwrap_err()
        .contains("AUTH_REQUIRED"));
    zip_file(
        &mods.join("dependency.jar"),
        "fabric.mod.json",
        r#"{"id":"dep","version":"2.0.0"}"#,
    );
    assert_eq!(app.preflight("test").unwrap()["errors"], 0);
    let copied = app
        .tools("diagnostic.create", &json!({"id":"test"}))
        .unwrap();
    let copy_id = text(&copied, "id");
    assert!(!app.game(copy_id).unwrap().join("saves").exists());
    let step = app
        .tools(
            "diagnostic.step",
            &json!({"id":copy_id,"answer":"baseline"}),
        )
        .unwrap();
    assert_eq!(step["disabled"].as_array().unwrap().len(), 1);
    let stop = app
        .tools("diagnostic.step", &json!({"id":copy_id,"answer":"reset"}))
        .unwrap();
    assert_eq!(stop["phase"], "stopped");
    assert!(app
        .game(copy_id)
        .unwrap()
        .join("mods/consumer.jar")
        .exists());
    assert!(app
        .game(copy_id)
        .unwrap()
        .join("mods/dependency.jar")
        .exists());
    assert_eq!(
        fs::read(instance.join("minecraft/saves/world/level.dat")).unwrap(),
        b"original-world"
    );
    let mut seen = false;
    for _ in 0..60 {
        if app.engine("instance.get", json!({"id":copy_id})).is_ok() {
            seen = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(seen, "Prism did not load diagnostic copy");
    let report = app.tools("disk", &json!({})).unwrap();
    assert!(report["instances"].as_array().unwrap().len() >= 2);
    assert!(app
        .tools(
            "disk.clean",
            &json!({"keys":["instances"],"confirmed":true})
        )
        .is_err());
    assert!(app.tools("disk.clean", &json!({"keys":["cache"]})).is_err());
    app.record_launch(&json!({"id":"test"}), &json!({"job":"fixture-finished"}))
        .unwrap();
    app.reconcile_history(
        &json!({"jobs":[{"id":"fixture-finished","state":"failed","error":"fixture crash"}]}),
    )
    .unwrap();
    let history: Value =
        serde_json::from_str(&fs::read_to_string(root.join("graphite-history.json")).unwrap())
            .unwrap();
    assert_eq!(history[0]["state"], "failed");
    for name in ["consumer.jar", "dependency.jar"] {
        fs::remove_file(mods.join(name)).unwrap();
    }
    app.engine("instance.manage", json!({"id":copy_id,"action":"trash"}))
        .unwrap();
    app.queue_worker
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let first = app.enqueue("fixture-invalid".into(), vec![]).unwrap();
    let second = app.enqueue("fixture-invalid".into(), vec![]).unwrap();
    {
        let mut q = app.queue.lock().unwrap();
        q[0].view["state"] = json!("running");
        q[0].view["instance"] = json!("test");
    }
    assert!(
        app.with_instance("test", || Ok(())).is_ok(),
        "A Rust queue entry must not block its own file operation"
    );
    app.queue.lock().unwrap()[0].view["state"] = json!("queued");
    app.queue_action(&[json!("cancel"), first["job"].clone()])
        .unwrap();
    assert_eq!(
        app.engine("state", json!({})).unwrap()["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|j| j["id"] == first["job"])
            .unwrap()["state"],
        "cancelled"
    );
    app.queue_worker
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = app.enqueue("fixture-invalid".into(), vec![]).unwrap();
    for _ in 0..100 {
        if !app.queue_worker.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        app.engine("state", json!({})).unwrap()["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|j| j["id"] == second["job"])
            .unwrap()["state"],
        "failed"
    );
    let resource = root.join("resource.zip");
    zip_file(&resource, "pack.mcmeta", "{}");
    assert!(storage::zip_info(&resource, "shader").is_err());
    app.install_local("test", "resourcepack", &[resource.clone()])
        .unwrap();
    assert!(app
        .install_local("test", "resourcepack", &[resource.clone()])
        .is_err());
    let list = app
        .file_action("toggle", "test", "resourcepack", "resource.zip")
        .unwrap();
    assert_eq!(list[0]["enabled"], false);
    assert!(instance
        .join("minecraft/resourcepacks/resource.zip.disabled")
        .exists());
    app.file_action("toggle", "test", "resourcepack", "resource.zip.disabled")
        .unwrap();
    let jar = root.join("test.jar");
    zip_file(
        &jar,
        "fabric.mod.json",
        r#"{"name":"Local Fabric","version":"1.2.3"}"#,
    );
    assert!(app
        .dispatch("drop-files", vec![json!([jar]), json!("test")])
        .is_err());
    app.drops.lock().unwrap().insert(jar.clone());
    let dropped = app
        .dispatch("drop-files", vec![json!([jar]), json!("test")])
        .unwrap();
    assert_eq!(dropped["count"], 1);
    let mods = app.files("test", "mod").unwrap();
    assert_eq!(mods[0]["name"], "Local Fabric");
    assert_eq!(mods[0]["version"], "1.2.3");
    let saved = app.with_instance("test", || app.backup("test")).unwrap();
    fs::write(
        instance.join("minecraft/saves/world/level.dat"),
        b"changed-world",
    )
    .unwrap();
    assert_eq!(
        fs::read(PathBuf::from(text(&saved, "path")).join("minecraft/saves/world/level.dat"))
            .unwrap(),
        b"original-world"
    );
    assert_eq!(app.backups("test").unwrap().as_array().unwrap().len(), 1);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let status_address = listener.local_addr().unwrap().to_string();
    let status_worker = std::thread::spawn(move || {
        use std::io::Read;
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 512];
        stream.read(&mut request).unwrap();
        let data = json!({"version":{"name":"1.20.1"},"players":{"online":7,"max":20},"description":{"text":"Cache QA"}}).to_string().into_bytes();
        assert!(data.len() + 2 < 128);
        let mut packet = vec![(data.len() + 2) as u8, 0, data.len() as u8];
        packet.extend(data);
        stream.write_all(&packet).unwrap();
    });
    let favorite = app
        .servers(
            "save",
            &json!({"name":"Local","address":status_address,"instanceId":"test"}),
        )
        .unwrap();
    let id = &favorite[0]["id"];
    let status = app.servers("status", &json!({"id":id})).unwrap();
    status_worker.join().unwrap();
    assert_eq!(status["online"], true);
    assert_eq!(status["players"], 7);
    assert!(status["checkedAt"].is_i64());
    assert!(root.join("graphite-server-status.json").exists());
    assert_eq!(
        app.servers("cached-status", &json!({})).unwrap()[id.as_str().unwrap()],
        status
    );
    let changed = app
        .servers(
            "save",
            &json!({"id":id,"name":"Renamed","address":"[::1]:25565","instanceId":"test"}),
        )
        .unwrap();
    assert_eq!(changed[0]["name"], "Renamed");
    assert_eq!(app.servers("cached-status", &json!({})).unwrap(), json!({}));
    assert_eq!(
        app.servers("list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    app.servers("remove", &json!({"id":id})).unwrap();
    assert_eq!(app.servers("list", &json!({})).unwrap(), json!([]));
    assert_eq!(app.servers("cached-status", &json!({})).unwrap(), json!({}));
    fs::write(
        root.join("graphite-server-status.json"),
        "corrupted derived cache",
    )
    .unwrap();
    assert_eq!(app.servers("cached-status", &json!({})).unwrap(), json!({}));
    assert!(app
        .dispatch(
            "engine",
            vec![
                json!("launch"),
                json!({"id":"test","twitchland":{"token":"bad"}})
            ]
        )
        .is_err());
    assert!(app
        .dispatch("engine", vec![json!("not-allowed"), json!({})])
        .is_err());
    app.busy.lock().unwrap().insert("test".into());
    assert!(app
        .dispatch("engine", vec![json!("launch"), json!({"id":"test"})])
        .is_err());
    app.busy.lock().unwrap().clear();
    assert!(app
        .engine("launch", json!({"id":"test","server":"localhost:25565"}))
        .unwrap_err()
        .contains("AUTH_REQUIRED"));
    *app.session.lock().unwrap() = Some(auth::Session {
        name: "Example".into(),
        token: "must-not-leak".into(),
        uuid: "uuid".into(),
        expires: Value::Null,
        generation: "g".into(),
    });
    assert!(!app.auth_status().to_string().contains("must-not-leak"));
    assert!(!app
        .engine("state", json!({}))
        .unwrap()
        .to_string()
        .contains("must-not-leak"));
    app.twitchland("signout", &json!({})).unwrap();
    assert!(app.add_offline("bad name").is_err());
    assert!(app.add_offline("слишком").is_err());
    assert!(app.add_offline("12345678901234567").is_err());
    let added = app.add_offline("OfflineQA").unwrap();
    let offline_id = text(&added, "id");
    assert!(app.add_offline("offlineqa").is_err());
    assert_eq!(app.offline_name().unwrap().as_deref(), Some("OfflineQA"));
    let restored = crate::offline::load(&root).unwrap();
    assert_eq!(restored.selected.as_deref(), Some(offline_id));
    assert_eq!(restored.profiles[0].name, "OfflineQA");
    let account_state = app.engine("state", json!({})).unwrap();
    assert!(account_state["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["id"] == offline_id && a["type"] == "offline" && a["default"] == true));
    assert!(app
        .dispatch(
            "engine",
            vec![
                json!("launch"),
                json!({"id":"test","offline":{"name":"Injected"}})
            ]
        )
        .is_err());
    app.select_offline(None).unwrap();
    assert!(app.offline_name().unwrap().is_none());
    app.engine("accounts.default", json!({"id":offline_id}))
        .unwrap();
    assert_eq!(app.offline_name().unwrap().as_deref(), Some("OfflineQA"));
    let launch = app.engine("launch", json!({"id":"test"})).unwrap();
    let job = text(&launch, "job");
    assert!(!job.is_empty());
    let _ = app.engine("cancel", json!({"id":job}));
    app.engine("accounts.remove", json!({"id":offline_id}))
        .unwrap();
    assert!(app.offline_name().unwrap().is_none());
    assert!(crate::offline::load(&root).unwrap().profiles.is_empty());
    let duplicate = app
        .engine("instance.manage", json!({"id":"test","action":"duplicate"}))
        .unwrap();
    wait_job(&app, text(&duplicate, "job"));
    let library = app.library().unwrap();
    let copy = library
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "Тест (копия)")
        .unwrap();
    app.engine("instance.manage", json!({"id":copy["id"],"action":"trash"}))
        .unwrap();
    assert_eq!(app.library().unwrap().as_array().unwrap().len(), 1);
    let archive = root.join("prism.zip");
    let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
    for (name, bytes) in [
        (
            "payload/instance.cfg",
            fs::read(instance.join("instance.cfg")).unwrap(),
        ),
        (
            "payload/mmc-pack.json",
            fs::read(instance.join("mmc-pack.json")).unwrap(),
        ),
        (
            "payload/minecraft/saves/world/level.dat",
            b"imported-world".to_vec(),
        ),
    ] {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
    let picked = app.register_import(archive).unwrap();
    let imported = app
        .enqueue(
            "import-instance".into(),
            vec![picked["token"].clone(), json!("Imported Rust"), json!(true)],
        )
        .unwrap();
    let second_import = app
        .enqueue(
            "import-instance".into(),
            vec![
                picked["token"].clone(),
                json!("Second queued import"),
                json!(true),
            ],
        )
        .unwrap();
    wait_job(&app, text(&imported, "job"));
    wait_job(&app, text(&second_import, "job"));
    assert!(app
        .library()
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["name"] == "Second queued import"));
    let library = app.library().unwrap();
    let imported = library
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "Imported Rust")
        .unwrap();
    assert_eq!(
        fs::read(
            app.game(text(imported, "id"))
                .unwrap()
                .join("saves/world/level.dat")
        )
        .unwrap(),
        b"imported-world"
    );
    app.native.close();
    drop(app);
    assert_eq!(root.parent(), Some(base.as_path()));
    fs::remove_dir_all(root).unwrap();
}
fn wait_job(app: &App, id: &str) {
    for _ in 0..500 {
        let state = app.engine("state", json!({})).unwrap();
        assert_eq!(state["visibleWindows"], 0);
        if let Some(job) = state["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|j| j["id"] == id)
        {
            if !matches!(text(job, "state"), "running" | "queued") {
                assert_eq!(job["state"], "succeeded", "{}", job["error"]);
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("Native task timed out")
}

#[test]
#[ignore = "requires GRAPHITE_LIVE_DATA: isolated prepared pack-update fixture"]
fn live_pack_update() {
    let root =
        PathBuf::from(std::env::var_os("GRAPHITE_LIVE_DATA").expect("Set GRAPHITE_LIVE_DATA"));
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("qa-rust-live-"));
    std::env::set_var("GRAPHITE_DATA", &root);
    let backend = std::env::var_os("GRAPHITE_TEST_BACKEND").expect("Set GRAPHITE_TEST_BACKEND");
    std::env::set_var("GRAPHITE_BACKEND", backend);
    let app = App::new().unwrap();
    let preview = app.pack_preview("test").unwrap();
    assert!(preview["token"].is_string());
    assert!(preview["changes"]["added"].as_array().unwrap().len() > 0);
    let result = app.pack_update("test", text(&preview, "token")).unwrap();
    let game = app.game(text(&result, "id")).unwrap();
    assert_eq!(
        fs::read(game.join("saves/keep/level.dat")).unwrap(),
        b"preserve-world"
    );
    assert_eq!(
        fs::read(game.join("config/keep.txt")).unwrap(),
        b"preserve-setting"
    );
    assert_eq!(
        fs::read(app.game("test").unwrap().join("saves/keep/level.dat")).unwrap(),
        b"preserve-world"
    );
    assert_eq!(app.backups("test").unwrap().as_array().unwrap().len(), 1);
    let projects = app.api("project/complementary-reimagined", vec![]).unwrap();
    let versions = app
        .modrinth(
            "versions",
            &json!({"kind":"shader","instanceId":"test","projectId":projects["id"]}),
        )
        .unwrap();
    assert!(!versions.as_array().unwrap().is_empty());
    app.modrinth("install",&json!({"kind":"shader","instanceId":"test","projectId":projects["id"],"versionId":versions[0]["id"]})).unwrap();
    assert_eq!(
        app.files("test", "shader")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    app.native.close();
}
