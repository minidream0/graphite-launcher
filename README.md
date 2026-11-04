<img src="graphite-web/public/graphite.svg" width="52" alt="Graphite logo">

# Graphite Launcher

English | [Русский](README.ru.md)

Minecraft: Java Edition launcher for Windows. Manage instances, install content from Modrinth and keep separate settings for each instance.

**0.11.2 beta · Windows x64**

[Download installer](https://github.com/minidream0/graphite-launcher/releases/download/v0.11.2/Graphite-Setup-0.11.2.exe) · [Download portable](https://github.com/minidream0/graphite-launcher/releases/download/v0.11.2/Graphite-0.11.2-portable.zip) · [Latest release](https://github.com/minidream0/graphite-launcher/releases/latest)

![Graphite library](docs/images/graphite.png)

<details>
<summary>Smoky theme</summary>

![Smoky theme](docs/images/smoky.png)

</details>

## Install

Run the installer, or extract the portable ZIP into a writable folder and run `portable/Graphite.exe`. Keep the `engine` folder beside it.

Graphite needs Microsoft Edge WebView2 Runtime. The installer downloads it if it is missing, which requires internet access. The portable version does not install it automatically.

Accounts, worlds and settings are stored in `data` beside Graphite.exe. Keep that folder when updating manually. The downloads contain no Minecraft game files or personal accounts.

Release assets include a source archive and `SHA256SUMS.txt` for checking the downloads.

## Features

- Minecraft instances with loader, Java and memory settings.
- Microsoft sign-in and local profiles.
- Modrinth modpacks, mods, resource packs and shaders. Import `.mrpack` files and Prism instance ZIPs.
- Enable, disable and remove installed content. Back up worlds and settings. Modpack updates create a separate copy and carry over worlds and settings.
- Favorite servers, status checks and joining with a selected instance.
- Launch logs, crash diagnostics, session history, download queue and disk usage.
- Graphite and Smoky themes, with reduced-motion support.

## Beta limitations

The EXE and installer are not digitally signed. Windows may warn about them or block them, depending on its settings.

Microsoft sign-in currently uses Prism Launcher's upstream application ID. Graphite has no separate Microsoft application registration, so sign-in depends on that external registration. The bundled CurseForge key is disabled; there is no CurseForge interface.

Local automated tests and UI checks passed. Installation on another clean PC without WebView2 and every combination of account, Minecraft version and mods have not been verified. Launcher changes do not guarantee higher FPS in Minecraft.

## Report a problem

[Open an issue](https://github.com/minidream0/graphite-launcher/issues/new/choose) in English or Russian. Include the Graphite, Windows, Minecraft and loader versions, steps to reproduce and the error message. Remove tokens, passwords, account files and private addresses from logs.

For vulnerabilities, use [private reporting](https://github.com/minidream0/graphite-launcher/security/advisories/new). See [SECURITY.md](SECURITY.md).

## Build from source

The interface uses React and Tailwind. The Rust/Tauri host uses the system WebView2 runtime. A modified Prism Launcher engine handles Minecraft through C++. Graphite does not use Electron.

From `graphite-web`:

```powershell
npm ci
npm test
npm run build
```

The complete application also needs the Rust host and C++ engine. See [BUILD.md](BUILD.md) for commands and dependencies, and [THIRD-PARTY.md](THIRD-PARTY.md) for component notices.

## License

Graphite is licensed under [GPL-3.0-only](LICENSE). Individual components retain their own licenses.

The included engine is based on [Prism Launcher](https://github.com/PrismLauncher/PrismLauncher), which derives from MultiMC. Its modified source is included in this repository.

Graphite is an independent project, unaffiliated with and not endorsed by Prism Launcher, Mojang or Microsoft. A local profile does not grant a Minecraft license.
