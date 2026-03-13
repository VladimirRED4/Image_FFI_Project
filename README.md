# Image FFI Project with Plugins

![CI](https://github.com/VladimirRED4/Image_FFI_Project/actions/workflows/ci.yml/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

Rust-приложение для обработки изображений с поддержкой динамической загрузки плагинов.

## 📋 Описание

Приложение загружает PNG-изображение, применяет к нему указанный плагин обработки и сохраняет результат. Плагины представляют собой динамические библиотеки (.dll/.so/.dylib), которые загружаются во время выполнения.

## 🏗️ Архитектура

Проект состоит из:

- **image_processor** - основное приложение
- **mirror_plugin** - плагин для зеркального отражения
- **blur_plugin** - плагин для размытия изображения

## 🚀 Установка и сборка

### Требования

- Rust (версия 1.70 или выше)
- Cargo

### Сборка проекта

```bash
# Клонировать репозиторий
git clone https://github.com/VladimirRED4/Image_FFI_Project.git
cd image_ffi_project

# Собрать все компоненты
cargo build --workspace

# Собрать в release режиме (оптимизированная версия)
cargo build --workspace --release
```

## 📖 Использование

### Базовый синтаксис

```text
./target/release/image_processor.exe <INPUT> <OUTPUT> <PLUGIN> <PARAMS_FILE> [--plugin-path <PATH>]
```

#### Аргументы

- **INPUT** - Путь к исходному PNG-изображению
- **OUTPUT** - Путь для сохранения обработанного изображения
- **PLUGIN** - Имя плагина (без расширения)
- **PARAMS_FILE** - Путь к JSON-файлу с параметрами
- **--plugin-path** - Директория с плагинами (по умолчанию: target/debug или target/release)

### Примеры

- **Зеркальное отражение**

```bash
# Создать файл параметров
echo '{"horizontal":true,"vertical":false}' > mirror_params.json

# Запустить обработку
./target/release/image_processor.exe input.png output.png mirror_plugin mirror_params.json
```

- **Размытие изображения**

```bash
# Создать файл параметров
echo '{"radius":5,"iterations":2}' > blur_params.json

# Запустить обработку
./target/release/image_processor.exe input.png output.png blur_plugin blur_params.json
```

## 🎯 Плагины

- **Mirror Plugin (Зеркальное отражение)**

Параметры в формате JSON:

```json
{
    "radius": 5,          // радиус размытия (1-20)
    "iterations": 2       // количество итераций (1-5)
}
```

- **Blur Plugin (Размытие)**
Параметры в формате JSON:

```json
{
    "radius": 5,          // радиус размытия (1-20)
    "iterations": 2       // количество итераций (1-5)
}
```

## 🧪 Тестирование

```bash

# Тесты плагинов
cargo test -p mirror_plugin
cargo test -p blur_plugin
```
