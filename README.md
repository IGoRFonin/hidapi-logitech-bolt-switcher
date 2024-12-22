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

## Требования
- Rust
- Linux/macOS/Windows
- Приемник Logitech Bolt

## Установка Rust
### Linux/macOS
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Windows
Скачайте и запустите [rustup-init.exe](https://win.rustup.rs/)

## Запуск в режиме отладки
```bash
DEBUG=1 cargo run
```

## Установка и автозапуск

### Linux (Ubuntu, Debian, Fedora)
```bash
# 1. Сборка
cargo build --release

# 2. Копирование в bin
sudo cp target/release/channel_switcher /usr/local/bin/

# 3. Добавление прав доступа к USB
sudo usermod -a -G plugdev $USER
sudo usermod -a -G input $USER

cat << EOF | sudo tee /etc/udev/rules.d/50-logitech.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="046d", ATTRS{idProduct}=="c548", MODE="0666", GROUP="plugdev"
EOF

sudo udevadm control --reload-rules
sudo udevadm trigger

# 4. Добавление в автозапуск
cat << EOF | sudo tee /etc/systemd/system/logitech-switch.service
[Unit]
Description=Logitech Channel Switcher
After=network.target

[Service]
Type=simple
Environment="DISPLAY=:0"
Environment="DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus"
Environment="XDG_RUNTIME_DIR=/run/user/$(id -u)"
ExecStart=/usr/local/bin/channel_switcher
Restart=always
RestartSec=3
User=$USER
Group=input
SupplementaryGroups=plugdev input

[Install]
WantedBy=multi-user.target
EOF

# Создаем лог файл и даем права
sudo touch /var/log/logitech-switch.log
sudo chown $USER:$USER /var/log/logitech-switch.log

sudo systemctl daemon-reload
sudo systemctl restart logitech-switch
```

Отключение автозапуска:
```bash
sudo systemctl disable logitech-switch
sudo systemctl stop logitech-switch
sudo rm /etc/systemd/system/logitech-switch.service
sudo rm /etc/udev/rules.d/50-logitech.rules
sudo rm /var/log/logitech-switch.log
```

### macOS
```bash
# 1. Сборка
cargo build --release

# 2. Копирование в bin
sudo cp target/release/channel_switcher /usr/local/bin/

# 3. Добавление в автозапуск
mkdir -p ~/Library/LaunchAgents
cat << EOF > ~/Library/LaunchAgents/com.logitech.switch.plist
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.logitech.switch</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/channel_switcher</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
EOF

# Загружаем сервис
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.logitech.switch.plist
```

Для управления программой используйте:
```bash
# Остановить
launchctl bootout gui/$UID/com.logitech.switch

# Запустить
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.logitech.switch.plist

# Перезапустить
launchctl bootout gui/$UID/com.logitech.switch
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.logitech.switch.plist
```

Для полного удаления:
```bash
launchctl bootout gui/$UID/com.logitech.switch
rm ~/Library/LaunchAgents/com.logitech.switch.plist
```

### Windows
```powershell
# 1. Сборка
cargo build --release

# 2. Копирование в Program Files
mkdir "C:\Program Files\LogitechSwitch"
cp target/release/channel_switcher.exe "C:\Program Files\LogitechSwitch\"

# 3. Добавление в автозапуск
$WshShell = New-Object -comObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\LogitechSwitch.lnk")
$Shortcut.TargetPath = "C:\Program Files\LogitechSwitch\channel_switcher.exe"
$Shortcut.Save()
```

Отключение автозапуска:
```powershell
Remove-Item "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\LogitechSwitch.lnk"
```