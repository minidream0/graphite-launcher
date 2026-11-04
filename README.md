<img src="graphite-web/public/graphite.svg" width="52" alt="Логотип Graphite">

# Graphite Launcher

Лаунчер Minecraft: Java Edition для Windows с библиотекой сборок, Modrinth и двумя темами оформления.

Open-source Minecraft Java launcher for Windows. React/Tailwind interface, Rust/Tauri host with system WebView2, and a modified Prism Launcher C++ engine.

**Версия 0.11.2 · Бета · Windows x64**

[Скачать установщик](https://github.com/minidream0/graphite-launcher/releases/download/v0.11.2/Graphite-Setup-0.11.2.exe) · [Скачать portable](https://github.com/minidream0/graphite-launcher/releases/download/v0.11.2/Graphite-0.11.2-portable.zip) · [Релиз и исходники](https://github.com/minidream0/graphite-launcher/releases/tag/v0.11.2)

![Библиотека Graphite](docs/images/graphite.png)

<details>
<summary>Тема «Дымчатый»</summary>

![Тема Дымчатый](docs/images/smoky.png)

</details>

## Установка

Файлы доступны в [Releases](https://github.com/minidream0/graphite-launcher/releases):

- `Graphite-Setup-0.11.2.exe` — установщик.
- `Graphite-0.11.2-portable.zip` — распаковать в доступную для записи папку и запустить `portable/Graphite.exe`. Папку `engine` оставить рядом.
- `Graphite-0.11.2-source.zip` — исходники выпуска, включая изменённый Prism.
- `SHA256SUMS.txt` — контрольные суммы.

Для работы нужен Microsoft Edge WebView2 Runtime. Установщик содержит его загрузчик; если Runtime отсутствует, для установки потребуется интернет. Portable автоматически WebView2 не устанавливает.

Данные хранятся в `data` рядом с Graphite.exe. **Сохраняйте эту папку при ручном обновлении.** Готовые файлы Minecraft, личные аккаунты и миры в дистрибутив не включены.

## Возможности

- Установка Minecraft и загрузчиков, настройки Java и памяти.
- Microsoft-аккаунты и локальные профили.
- Modrinth: сборки, моды, ресурс-паки и шейдеры; импорт `.mrpack` и Prism ZIP.
- Управление содержимым сборок, резервные копии, обновление сборки в отдельную копию с переносом миров и настроек.
- Избранные серверы, проверка состояния и подключение с выбранной сборкой.
- Логи запуска, диагностика, история сессий, очередь загрузок и управление местом на диске.
- Темы Graphite и «Дымчатый», анимации с учётом настройки уменьшения движения.

## Статус беты

Приложение и установщик **не подписаны Authenticode**. Windows может показать предупреждение или заблокировать запуск согласно своим настройкам безопасности.

Microsoft-вход сейчас использует upstream Client ID Prism; отдельная регистрация Graphite не создана. Доступность входа зависит от этой внешней регистрации. Это не означает одобрение Graphite со стороны Prism или Microsoft. Встроенный ключ CurseForge отключён; интеграции CurseForge в интерфейсе нет.

Проверены локальные JS/Rust/C++ тесты и нативные проверки интерфейса. Чистая установка на другом ПК без WebView2 и все сочетания игр, модов и аккаунтов не подтверждены. Изменения интерфейса не гарантируют повышение FPS Minecraft.

## Ошибки и предложения

Создайте Issue: укажите версию Graphite, Windows, Minecraft и загрузчика, шаги воспроизведения и сообщение об ошибке. Перед прикреплением лога удалите пароли, токены, личные адреса и данные аккаунтов. См. [SECURITY.md](SECURITY.md).

## Разработка

[Инструкция сборки](BUILD.md) · [Лицензии компонентов](THIRD-PARTY.md)

| Каталог | Содержимое |
| --- | --- |
| `graphite-web/src` | React-интерфейс, темы, анимации |
| `graphite-web/public` | Арт и иконки |
| `graphite-web/src-tauri` | Rust-бэкенд и Tauri |
| `PrismLauncher` | Изменённый C++-движок и vendored-библиотеки |
| `third-party` | Лицензии и исходники authlib-injector 1.2.7 |

Для фронтенда: `npm ci`, `npm test`, `npm run build` из `graphite-web`. Для полной сборки нужен также C++-движок; команды и зависимости описаны в BUILD.md.

## Лицензия и авторство

Код проекта распространяется под [GPL-3.0-only](LICENSE), с сохранением отдельных лицензий сторонних компонентов и уведомлений в исходниках. Graphite основан на [Prism Launcher](https://github.com/PrismLauncher/PrismLauncher), производном от MultiMC; изменённые исходники движка включены в репозиторий и архив релиза.

Graphite — самостоятельный проект. Он не связан с Prism Launcher, Mojang или Microsoft и не одобрен ими. Локальный профиль не предоставляет лицензию на Minecraft.
