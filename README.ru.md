<img src="graphite-web/public/graphite.svg" width="52" alt="Логотип Graphite">

# Graphite Launcher

[English](README.md) | Русский

Лаунчер Minecraft: Java Edition для Windows. Управление сборками, установка контента из Modrinth и отдельные настройки для каждой сборки.

**0.11.2 бета · Windows x64**

[Скачать установщик](https://github.com/minidream0/graphite-launcher/releases/download/v0.11.2/Graphite-Setup-0.11.2.exe) · [Скачать portable](https://github.com/minidream0/graphite-launcher/releases/download/v0.11.2/Graphite-0.11.2-portable.zip) · [Последний релиз](https://github.com/minidream0/graphite-launcher/releases/latest)

![Библиотека Graphite](docs/images/graphite.png)

<details>
<summary>Тема «Дымчатый»</summary>

![Тема «Дымчатый»](docs/images/smoky.png)

</details>

## Установка

Запустите установщик или распакуйте portable ZIP в доступную для записи папку и откройте `portable/Graphite.exe`. Папка `engine` должна оставаться рядом.

Для работы нужен Microsoft Edge WebView2 Runtime. Если его нет, установщик загрузит его через интернет. Portable-версия не устанавливает WebView2 автоматически.

Аккаунты, миры и настройки хранятся в `data` рядом с Graphite.exe. Сохраняйте эту папку при ручном обновлении. В загрузки не входят файлы Minecraft и личные аккаунты.

В релизе есть архив исходников и `SHA256SUMS.txt` для проверки скачанных файлов.

## Возможности

- Сборки Minecraft с настройками загрузчика, Java и памяти.
- Вход через Microsoft и локальные профили.
- Сборки, моды, ресурс-паки и шейдеры из Modrinth. Импорт файлов `.mrpack` и ZIP со сборками Prism.
- Включение, отключение и удаление установленного контента. Резервные копии миров и настроек. Обновление сборки создаёт отдельную копию с переносом миров и настроек.
- Избранные серверы, проверка статуса и подключение с выбранной сборкой.
- Логи запуска, поиск причин вылета, история сессий, очередь загрузок и управление местом на диске.
- Темы Graphite и «Дымчатый», настройка уменьшения анимации.

## Ограничения беты

EXE и установщик не подписаны цифровой подписью. Windows может показать предупреждение или заблокировать запуск в зависимости от настроек.

Вход через Microsoft пока использует ID приложения Prism Launcher. У Graphite нет отдельной регистрации приложения Microsoft, поэтому вход зависит от чужой регистрации. Встроенный ключ CurseForge отключён; интерфейса CurseForge нет.

Локальные автоматические тесты и проверки интерфейса прошли. Установка на другом чистом ПК без WebView2 и все сочетания аккаунтов, версий Minecraft и модов не проверены. Изменения лаунчера не гарантируют прирост FPS в игре.

## Сообщить о проблеме

[Создайте issue](https://github.com/minidream0/graphite-launcher/issues/new/choose) на русском или английском. Укажите версии Graphite, Windows, Minecraft и загрузчика, действия для повторения и текст ошибки. Удалите из логов токены, пароли, файлы аккаунтов и приватные адреса.

Об уязвимостях сообщайте через [закрытую форму](https://github.com/minidream0/graphite-launcher/security/advisories/new). Подробнее в [SECURITY.md](SECURITY.md), на английском.

## Сборка из исходников

Интерфейс написан на React и Tailwind. Оболочка Rust/Tauri использует системный WebView2. Изменённый движок Prism Launcher на C++ отвечает за работу с Minecraft. Electron не используется.

Из папки `graphite-web`:

```powershell
npm ci
npm test
npm run build
```

Для полного приложения также нужны оболочка Rust и движок C++. Команды и зависимости описаны в [BUILD.md](BUILD.md), лицензии компонентов в [THIRD-PARTY.md](THIRD-PARTY.md). Эти технические документы написаны на английском.

## Лицензия

Graphite распространяется под [GPL-3.0-only](LICENSE). У отдельных компонентов свои лицензии.

В комплект входит изменённый движок [Prism Launcher](https://github.com/PrismLauncher/PrismLauncher), основанный на MultiMC. Его исходники включены в репозиторий.

Graphite не связан с Prism Launcher, Mojang или Microsoft и не одобрен ими. Локальный профиль не даёт лицензию Minecraft.

