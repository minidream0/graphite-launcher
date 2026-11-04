<img src="graphite-web/public/graphite.svg" width="52" alt="Graphite">

# Graphite Launcher

[English](README.md) | Русский

Лаунчер Minecraft: Java Edition для Windows. Несколько сборок, моды из Modrinth и две темы оформления: Graphite и «Дымчатый».

[Скачать](https://github.com/minidream0/graphite-launcher/releases/latest) · [Ошибки и предложения](https://github.com/minidream0/graphite-launcher/issues)

![Библиотека сборок](docs/images/graphite.png)

<details>
<summary>Тема «Дымчатый»</summary>

![Тема «Дымчатый»](docs/images/smoky.png)

</details>

## Возможности

- Отдельные сборки с выбором версии Minecraft, загрузчика модов, Java и памяти.
- Вход через Microsoft и локальные профили.
- Установка сборок, модов, ресурс-паков и шейдеров из Modrinth.
- Импорт `.mrpack` и сборок Prism, установка файлов перетаскиванием в окно.
- Управление модами и резервные копии миров и настроек.
- Обновление сборки в отдельной копии с переносом миров и настроек.
- Избранные серверы и подключение с выбранной сборкой.
- Логи, диагностика вылетов, история сессий, очередь загрузок и очистка диска.

## Установка

Скачайте установщик со [страницы релиза](https://github.com/minidream0/graphite-launcher/releases/latest) и запустите его.

Для версии без установки распакуйте portable-архив и откройте `portable/Graphite.exe`. Папку `engine` оставьте рядом. При ручном обновлении сохраняйте папку `data` с аккаунтами, мирами и настройками.

Для работы нужен WebView2. Установщик загрузит его при необходимости; для portable-версии его нужно установить отдельно.

## Разработка

Интерфейс: React и Tailwind. Оболочка: Rust/Tauri с системным WebView2. Движок: изменённый Prism Launcher на C++.

[Сборка из исходников](BUILD.md) · [Сторонние компоненты](THIRD-PARTY.md) · [Безопасность](SECURITY.md)

## Лицензия

[GPL-3.0-only](LICENSE). Лицензии сторонних компонентов указаны отдельно.

Graphite основан на [Prism Launcher](https://github.com/PrismLauncher/PrismLauncher). Исходники изменённого движка включены в репозиторий. Graphite не является официальным продуктом Prism Launcher, Mojang или Microsoft.
