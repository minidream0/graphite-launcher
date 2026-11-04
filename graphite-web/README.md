# Graphite 0.11.2

CurseForge is not exposed in Graphite. Its bundled Prism API key is now empty in the engine source and release CMake configuration. Microsoft authentication and Modrinth remain unchanged. The engine still contains upstream CurseForge implementation code, but Graphite does not configure a CurseForge credential or offer that integration.

## Controls in both themes

Shared controls.css defines softer shapes and distinct action levels in Graphite and Smoky. Main actions are filled; secondary actions have quiet surfaces; instance tools are two columns of unboxed actions. The hero readiness/status badge has been removed in both themes. Fields, selectors, navigation and dialogs share the same shape system. Press feedback uses transform, keyboard focus remains visible, and reduced motion disables transitions. Icon assets, theme colors and artwork are preserved.

## Appearance themes

Settings → Appearance offers Graphite and Smoky (Дымчатый). Rust persists the selection in data/graphite-appearance.json and loads it before mounting/showing the UI, without browser localStorage. Selection applies immediately and does not submit or change Java/memory settings. The original Graphite art, logo, icon catalog and startup remain available.

Smoky has warm charcoal-brown surfaces, milk text and muted peach accents. A new generated window/Minecraft-valley background supports an editable faceted smoky-gray cat. It sleeps, blinks and moves its tail; watches installations, stretches on launch, perks its ears after task failures, and looks up when petted. It pauses when hidden and respects reduced motion. A separate SVG/CSS startup nudges an actual logo piece into place and fades into the library; Skip, Escape/Enter, minimize and close remain available. It does not allocate a WebGL context.

All 80 feature icons have a separately generated Smoky counterpart in five transparent PNG atlases, mapped to the same semantic IDs and clipped to measured glyph bounds. User-provided custom icons remain intact. Simple utility arrows/close controls remain crisp vectors. Generated art and prompts: design/smoky-art-prompt.md and design/smoky-icon-prompts.md. Native QA: node ../smoky-native-check.cjs. This tests real settings switching, keyboard focus, disk persistence/full restart, all 80 icons, compact layout, reduced motion and unchanged instance/world/memory settings.

## Favorite servers

Server forms validate inline, retain drafts on errors and require only a name and address. Version and Modrinth project are explicitly optional. A unique installed instance matching the entered Minecraft version is selected automatically; multiple matches require a choice. Existing favorites can switch bindings directly on their card. Finding an installed version alone does not establish mod compatibility.

Last known status, players, latency and MOTD persist in data/graphite-server-status.json with address and checkedAt. Opening the server panel immediately shows the cached result and refreshes it automatically, then every 30 seconds while visible, with at most four requests at a time. Closing/hidden panels stop scheduling additional batches. UI shows the check time and pending refresh. Cache records are ignored for changed addresses/deleted favorites; late responses cannot restore them. Cache failures do not replace a successful live status with an offline error.

## Operation scenes

Six real Three.js scenes accompany launch (portal), server launch (route), pack installation/import/update (assembly), mods (joining pieces), preflight (scanner) and a changed installed Graphite version (core replacement). A single nonmodal surface follows observed RPC requests and native job states; waiting queue entries do not animate. Renderer initialization markers in the current launch log finish launch visuals, not a timer or server ping. This does not establish that a player joined a server. Unknown old/modded clients can hide the visual; active tasks are no longer arbitrarily hidden at two minutes. No fabricated percentages or artificial task delays.

Action scenes now use dimension-preserving rounded boxes and verified clearances. The update core fits inside an actual cavity; the scanning beam stays in front of solid geometry. Portal frames and route pulses recycle only at zero opacity. A stable portal host moves into/out of the active modal without recreating its canvas or renderer. The animation clock pauses while hidden, bounds delayed frames, and resumes instead of restarting. Completion decelerates from the current pose; show/hide uses smooth WAAPI transitions. Geometry and loop tests cover five simulated minutes; browser QA can run a live 45-second portal with GRAPHITE_LONG_MOTION_QA=1. Startup animation sources remain unchanged.

The surface stays inside the active modal for keyboard/mouse accessibility. Hide never cancels work. Reduced motion and unavailable/lost WebGL use a static logo. Hidden documents pause rendering; closing the surface frees geometry, materials, environment maps and the context. Version receipts are stored by Rust in data/graphite-seen-version.json, independent of WebView browser storage. First run records a baseline; a changed version shows the replacement scene once. This is a post-update visual, not an automatic updater.

## Startup scene

Startup.jsx mounts the existing Workspace immediately behind an inert cinematic overlay. startup-scene.js dynamically imports Three.js and builds extruded front silhouettes from the approved graphite.svg coordinates, with beveled matte materials, procedural studio lighting and a coral joint. A final SVG crossfade preserves the exact application mark while it moves to the header. The scene runs for about 3.3 seconds after initialization; Esc, Enter and Skip dismiss it immediately. Reduced-motion never creates a WebGL context. Unsupported/lost WebGL and a five-second watchdog release the interface. Resize and visibility changes are handled, and all GPU resources and RAF work are disposed on exit.

## Instance tools

Play launches Minecraft directly. The additional JAR/dependency preflight runs only when explicitly requested through Check instance; it is not an automatic launch gate. Prism retains its normal game-file, Java and authentication handling.

Pre-launch checks inspect JAR integrity, duplicate top-level mod IDs and Fabric dependency/version declarations, including nested JARs. Unsupported constraints, dependency overrides and unsupported loaders are reported without claiming full compatibility. Prism remains responsible for Java and game startup.

Favorite servers can specify an exact Minecraft version and a Modrinth project ID/slug. Preparing installs a new compatible pack and binds it after successful installation. The server address alone does not reveal its required mods; version matching does not prove mod compatibility.

The Rust queue serializes Minecraft creation, imports, Modrinth content, server preparation and pack updates. Waiting tasks can be cancelled; native downloads can be cancelled once a native job exists. File-copy/API preparation stages finish before continuing. Failed/cancelled tasks can be retried; pack updates require a new preview. Queue state is session-only and interrupted downloads restart.

`graphite-history.json` stores up to 300 launch records, including preparation time and terminal state. An unobserved completion is labelled unknown. History opens the current instance log; it is not a per-session log archive.

Crash isolation creates a separate diagnostic instance without worlds, logs or screenshots. The user reproduces the same failure, then reports whether disabling half the candidates removed it. Different failures/dependency errors stop the search and restore test-disabled mods. The result is a suspect, not proof; multiple interacting faults are not automatically resolved.

Disk tools show instance/world sizes, backup size and two explicitly cleanable folders: `cache` and `.graphite-downloads`. Before cleanup, the UI lists the selected cache files (up to 500 per folder), requires confirmation and moves the whole chosen cache folder to the Windows Recycle Bin. Reparse points are skipped during size scanning; active jobs and file operations block cleanup. Shared libraries, assets, Java and instance data are not cleanup targets.

Windows launcher with a Rust desktop backend, system WebView2, the existing React/Tailwind interface and a Prism C++ worker. The released app contains no Electron or Node.js runtime.

## Runtime

Run `Graphite.exe` with its `engine` directory alongside it. UI assets, fonts and styles are embedded in the EXE. Minecraft data remains in `data` beside the EXE, using the same format and location as the previous Electron build. Microsoft accounts, instances, backups and favorite servers are preserved. TwitchLand sessions remain in memory and require sign-in after restarting, as before.

Windows 10/11 and Microsoft Edge WebView2 Runtime are required. The system WebView remains web-based rendering; this is not a Qt/Rust-widget UI. Tailwind and the existing visual design are retained.

## Build

The selected logo 06 is maintained in `public/graphite.svg`. Run `node build_icons.cjs` with Microsoft Edge installed (or set `GRAPHITE_EDGE`) to regenerate transparent PNG sizes and the multi-frame Windows ICO. The window, executable and installer share these assets. Preview exports are written to `outputs/logo-06` relative to the workspace root.

Install stable Rust with the MSVC target, Microsoft C++ Build Tools/Windows SDK, Node.js and WebView2. Run `npm ci`, `npm run build`, then `cargo build --release --manifest-path src-tauri/Cargo.toml`. Node.js is only a development tool.

The Prism engine stays in C++. Version 0.8 adds a local-profile adapter to its headless service and skips account locking for local identities while retaining normal game-file downloads. Package the matching rebuilt worker with the complete Qt runtime and authentication agent; an older 0.7 worker cannot launch local profiles.

`BUILD-RUST.ps1 -Backend C:\Graphite\engine\prismlauncher.exe -Run` builds and starts the app for development. `GRAPHITE_DATA` overrides the data directory; use an isolated directory when testing. `GRAPHITE_BACKEND` overrides the worker executable. Release defaults are `data` and `engine/prismlauncher.exe` next to the launcher EXE.

## Tests

`npm test` checks snapshot identity and polling disposal/coalescing. Set `GRAPHITE_TEST_BACKEND` to the matching bundled Prism EXE and run `cargo test --manifest-path src-tauri/Cargo.toml` for Rust filesystem, archive, offline-profile persistence/selection/validation, account privacy, device-ID parity, favorite-server, status-protocol and native-worker integration checks. These tests create isolated fixtures; never point them at a user's data directory.

The ignored `live_pack_update` integration test additionally requires `GRAPHITE_LIVE_DATA`, a disposable prepared managed-pack fixture; it downloads a real Modrinth update and shader, checks preservation of worlds/settings and the original instance, and runs explicitly with `cargo test live_pack_update -- --ignored`. The completed migration ran this check successfully using isolated existing asset caches.

`package_rust.py --engine C:\Graphite\engine --output C:\Build\outputs` packages the release EXE and existing engine. Supply `--native-source` to include the corresponding Prism source in the source archive.

## Architecture and boundaries

- `src/icon-catalog.json` maps 80 icons to exact regions of the three approved, unmodified PNG sheets. The web UI renders those regions through inline SVG viewports; labels and neighbouring icons are excluded. `src/icons.jsx` shares these icons across screens. Small directional/close controls retain simple vector glyphs.
- Rust installs the sheets and SVG views together under `icons/graphite-v1` before starting Prism, respecting an existing `IconsDir` setting. Relative image references keep them portable. Filenames become native icon keys such as `graphite-v1-categories-10`. Existing files are never overwritten. Keep this v1 namespace and frame mapping stable; use a new version for changed source imagery.
- The headless `icons.list` bridge excludes this frontend-rendered namespace from its image response so the 150-item budget remains available for existing icons. Prism's native icon registry still accepts and persists all 80 keys. The Minecraft/Java/authentication engine remains C++.

Offline profiles store only nicknames and the selected profile in `data/graphite-offline-accounts.json`. The backend generates the standard deterministic `OfflinePlayer:<name>` UUID; the renderer cannot inject launch credentials. Local profiles support singleplayer and servers allowing offline identities, not Microsoft-authenticated servers. Existing Microsoft/TwitchLand accounts remain separate.

- Rust handles named-pipe IPC, files, backups, ZIP validation, Modrinth queries/download checksums/dependency planning, favorites/status, file dialogs, clipboard, device identification and TwitchLand sessions.
- Prism C++ still handles Minecraft installation/launch, Java, Microsoft authentication and its existing instance tasks. The packaged engine SHA-256 is recorded in `release.json` and checked during packaging.
- The renderer uses an explicit Rust command allowlist; remote navigation is rejected. Authentication tokens stay in the backend. Dropped file paths require approval from the native drag event. Mutations reject linked paths and active instances; existing files are not overwritten by local installs.
- Panels load on demand. Unchanged snapshots retain React identity. Status polls run every 5 seconds while idle and 750 ms during active tasks; hidden windows slow down. Library reads skip hidden windows and run less often while idle.

See `PERFORMANCE.md` in the portable release for measured scope and limitations. A full authenticated game session and every possible mod combination are not established by launcher tests.

## Installer

`python build_installer.py --output C:\Build\outputs --tools C:\Build\installer-tools` packages the clean portable release using NSIS 3.08 and the signed Microsoft WebView2 bootstrapper. Run `package_rust.py` first. The installer runs without administrator rights, defaults to `%LOCALAPPDATA%\Programs\Graphite`, and includes the corresponding source ZIP. It checks WebView2 and installs it if absent (Internet required). Exit codes: 2 for running launcher/engine, 3 for missing WebView2, 4 for file write failure. Silent installation uses `/S /D=C:\Target` with `/D` last. Uninstall/update retain `data` and unknown files. To move an existing portable library to a fresh installation, close Graphite and copy its `data` directory to the installed directory.

## License

Graphite changes: GPL-3.0-only. Prism and Qt notices remain in `engine`. Rust dependency versions and source provenance are recorded by `Cargo.lock`. Phosphor is MIT; IBM Plex Sans is SIL OFL. `public/graphite.svg` reproduces the existing Graphite app mark for clean rendering at different sizes. Custom instance images remain intact.
