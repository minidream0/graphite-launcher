# Graphite native worker

Prism Launcher 11.1.1, upstream commit
7d2d3c1ec6fcf8255ebacd02d9f792688575dfb4, with Graphite changes.

The current desktop UI lives in the sibling graphite-web project. Start the
engine with --dir <data> --design-service graphite-<32 random hex digits>.
It runs a current-user local JSON pipe without a setup wizard or MainWindow.
Legacy Qt UI code and --design-command remain available for standalone builds;
Graphite no longer uses them. The service ignores legacy activation messages.

DesignService exposes public account state, device-code login, validated global
and instance settings, creation through native staging/download tasks, progress,
cancellation and direct LaunchTask execution with ownership checks. Tokens stay
in the worker. A heap maximum above free physical RAM is a non-blocking warning.

Modrinth catalog installs reuse InstanceImportTask and ResourceDownloadTask,
with SHA-512 verification and required dependencies resolved by the shell.
loadComponents reads the installed component list without starting a resolver.
Pack imports create independent instances, including repeat installs.

The Electron shell uses this service; it contains no replacement downloader or
replacement authentication flow. Official launcher binary self-updates are
disabled. Minecraft metadata and original service identifiers are retained.
The complete source archive contains both projects and upstream licence files.

See graphite-web/README.md and RELEASE.md for build, tests and known limits.

