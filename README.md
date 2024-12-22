# Переключатель каналов для Logitech Bolt

Это утилита для переключения каналов на устройствах Logitech Bolt (клавиатура + мышь). Позволяет быстро переключаться между разными компьютерами с помощью горячих клавиш.

## Как это работает
- Канал 0 - первый компьютер
- Канал 1 - второй компьютер 
- Канал 2 - третий компьютер

## Управление
Для переключения между каналами быстро нажмите два раза на соответствующую цифру:
- Двойное нажатие **1** - переключение на канал 0
- Двойное нажатие **2** - переключение на канал 1
- Двойное нажатие **3** - переключение на канал 2

Интервал между нажатиями не должен превышать 500мс.

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

## Примечания
- На Linux может потребоваться запуск с sudo для доступа к USB-устройству
- Программа автоматически находит Logitech Bolt приемник по VID/PID
- При возникновении ошибок попробуйте запустить в режиме отладки

## Требования
- Rust
- Linux/macOS/Windows
- Приемник Logitech Bolt