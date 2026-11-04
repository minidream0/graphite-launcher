# Porthole integration

Graphite controls an independently installed Porthole through its CLI. Do not bundle Porthole binaries or load its Steam SDK into Graphite.

Verified version: Porthole 3.0.2, Steam build 26120468.

Host:

```text
porthole.exe --json expose tcp/127.0.0.1:PORT --visibility private --game-name "Minecraft · Graphite"
```

Guest:

```text
porthole.exe --json connect CODE --auto-approve-ports tcp/REMOTE_PORT --remap REMOTE_PORT:LOCAL_PORT
```

Only the selected TCP port is automatically approved. Other offered ports are not approved. Target codes, port numbers and passwords are validated before process creation; commands are passed as individual arguments, not through a shell.

Porthole emits a static_code before lobby_ready. A code is not proof that the lobby is ready. A guest emits ready followed by port_offered and port_ready. Graphite also checks that the IPv4 loopback listener belongs to its own child PID before enabling launch. A listener can survive a lost remote connection, so ready/link-down events gate that check.

The CLI accepts quit on stdin. Graphite closes only its own helper on disconnect or normal application exit. It leaves a separately opened Porthole GUI alone. Session codes and event history stay in memory; only a manually selected executable path is saved.

The built application uses the existing Steam session through Porthole. It does not implement Steam OpenID or register a Graphite Steamworks application. Minecraft accounts remain separate.

Run the native integration check after building the Rust release:

```powershell
node .\porthole-check.cjs
```

Set GRAPHITE_PORTHOLE_TEST if the test helper is not in the default Steam library, and GRAPHITE_QA_ENGINE to select a clean Prism engine. Close other Graphite windows first. The test uses an isolated data directory and a real TCP echo through Porthole between a Steam client and a separate anonymous host on the same PC. It does not establish cross-network Minecraft compatibility.
