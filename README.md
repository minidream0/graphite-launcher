<img src="graphite-web/public/graphite.svg" width="52" alt="Graphite">

# Graphite Launcher

Graphite installs and launches Minecraft Java Edition, manages modpacks and keeps multiple instances in one library. It runs on Windows and uses the Prism Launcher engine.

[Download for Windows](https://github.com/minidream0/graphite-launcher/releases/latest) · [Русский](README.ru.md) · [GitHub Issues](https://github.com/minidream0/graphite-launcher/issues)

![Graphite instance library](docs/images/graphite.png)

## Instances and mods

Each instance has its own Minecraft version, mod loader, Java and memory settings. Vanilla Minecraft and modpacks keep separate worlds and settings. Instances can be renamed, duplicated and given custom icons.

The Modrinth catalog includes modpacks, individual mods, resource packs and shaders. Installed mods can be enabled, disabled, removed and checked for updates. You can also open an instance folder directly from the launcher.

Import existing Prism instances or `.mrpack` files. Dragging a `.mrpack` or ZIP into the window imports an instance; dragging a `.jar` adds a mod to the selected instance.

## Worlds and updates

Back up worlds and settings before making changes. When updating a Modrinth modpack, Graphite shows the changes and creates a separate instance with your worlds and settings carried over. The original instance stays in place.

The download queue tracks files being downloaded. Disk management shows how much space launcher files use.

## Accounts and servers

Graphite supports Microsoft sign-in and local profiles. Save favorite servers, check their status and choose an instance to join with.

Prepare an instance with the Minecraft version a server needs and check it before launching.

## Logs and history

Launch logs are available inside the launcher. After a failed launch, you can read the error and copy the log. Diagnostics help investigate Java, dependency and mod problems.

Session history records which instances you played and how long each session lasted.

## Themes

Graphite has two themes. The default uses dark graphite tones. Smoky has a quieter palette, a cat illustration and its own icons.

Launch and installation screens use 3D animations. Motion can be reduced in settings.

<details>
<summary>View the Smoky theme</summary>

![Smoky theme](docs/images/smoky.png)

</details>

## Install

The [release page](https://github.com/minidream0/graphite-launcher/releases/latest) has an installer and a portable archive for Windows x64.

The installer sets up Graphite and downloads WebView2 if needed. For portable, extract the archive and run `portable/Graphite.exe`. Keep the `engine` folder beside the EXE and install WebView2 separately.

Accounts, worlds and settings are stored in `data` beside Graphite.exe. Keep that folder when updating manually.

## Development

The interface uses React and Tailwind. The Rust/Tauri host uses system WebView2, and a modified Prism Launcher C++ engine handles Minecraft installation and launch. Its source is included in this repository.

[Build instructions](BUILD.md) · [Third-party components](THIRD-PARTY.md)

Graphite is based on [Prism Launcher](https://github.com/PrismLauncher/PrismLauncher) and is licensed under [GPL-3.0-only](LICENSE).
