# Переключатель каналов для Logitech Bolt

Это утилита для переключения каналов на устройствах Logitech Bolt (клавиатура + мышь). Позволяет быстро переключаться между разными компьютерами с помощью горячих клавиш.

## Как это работает
- Канал 0 - первый компьютер
- Канал 1 - второй компьютер 
- Канал 2 - третий компьютер

## Горячие клавиши
- **Linux**: Alt + 1/2/3
- **macOS**: Option + 1/2/3  
- **Windows**: просто 1/2/3

## Установка и запуск

1. Установите Rust если еще не установлен:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Скопируйте проект в свой репозиторий:

```bash
git clone https://github.com/IGoRFonin/hidapi-logitech-bolt-switcher
cd channel-switcher
cargo build --release
```

3. Запустите программу:

```bash
./target/release/channel_switcher
```

Режим отладки:
```bash
DEBUG=1 cargo run
# or
DEBUG=1 ./target/release/channel_switcher
```
