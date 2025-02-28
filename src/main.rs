use hidapi::{HidApi, HidDevice};
use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};
use notify_rust::Notification;
use std::env;

/// Константы для идентификации USB-устройства
const VID: u16 = 0x046D;        // Vendor ID (Logitech)
const PID: u16 = 0xC548;        // Product ID (Logitech Bolt)
const USAGE: u16 = 0x0001;
const USAGE_PAGE: u16 = 0xFF00;

/// Структура для формирования команды устройству
struct DeviceCommand {
    index: u8,    // Индекс устройства
    id: u8,       // ID команды
    channel: u8,  // Номер канала
}

impl DeviceCommand {
    fn new(index: u8, id: u8, channel: u8) -> Self {
        Self { index, id, channel }
    }

    fn to_bytes(&self) -> Vec<u8> {
        // Формируем команду фиксированной длины 7 байт, как в hidapitester
        vec![0x10, self.index, self.id, 0x1c, self.channel, 0x00, 0x00]
    }
}

/// Основная структура для управления переключением каналов
struct ChannelSwitcher {
    hid_device: HidDevice,      // HID-устройство (Logitech receiver)
    keyboard_cmd: DeviceCommand, // Команда для клавиатуры
    mouse_cmd: DeviceCommand,    // Команда для мыши
    current_channel: u8,         // Текущий активный канал
}

// В начале файла добавим глобальную переменную для debug режима
static mut DEBUG: bool = false;

// Макрос для debug вывода
macro_rules! debug {
    ($($arg:tt)*) => ({
        unsafe {
            if DEBUG {
                println!($($arg)*);
            }
        }
    })
}

#[cfg(target_os = "macos")]
fn notify(message: &str) {
    use std::process::Command;
    
    Command::new("osascript")
        .args(&[
            "-e",
            &format!(
                "display notification \"{}\" with title \"Logitech Switch\"",
                message
            ),
        ])
        .output()
        .ok();
}

impl ChannelSwitcher {
    fn new() -> Result<Self, Box<dyn Error>> {
        let api = HidApi::new()?;
        
        // Ищем устройство с нужными usage и usagePage
        let mut target_device = None;
        
        for device in api.device_list() {
            if device.vendor_id() == VID && 
               device.product_id() == PID && 
               device.usage() == USAGE && 
               device.usage_page() == USAGE_PAGE {
                debug!("Найдено подходящее устройство:");
                debug!("  Производитель: {}", device.manufacturer_string().unwrap_or("Неизвестно"));
                debug!("  Продукт: {}", device.product_string().unwrap_or("Неизвестно"));
                debug!("  VID/PID: {:04X}:{:04X}", device.vendor_id(), device.product_id());
                debug!("  Путь: {}", device.path().to_string_lossy());
                debug!("  Usage Page: 0x{:04X}", device.usage_page());
                debug!("  Usage: 0x{:04X}", device.usage());
                
                target_device = Some(device.path().to_string_lossy().into_owned());
                break;
            }
        }

        // Открываем устройство по найденному пути
        let device = if let Some(path) = target_device {
            api.open_path(std::ffi::CString::new(path)?.as_ref())
                .map_err(|e| format!("Failed to open device by path: {}", e))?
        } else {
            return Err("Не найдено подходящее устройство с нужными usage и usagePage".into());
        };

        Ok(Self {
            hid_device: device,
            keyboard_cmd: DeviceCommand::new(0x01, 0x09, 0),
            mouse_cmd: DeviceCommand::new(0x02, 0x0A, 0),
            current_channel: 0,
        })
    }

    /// Переключает устройства на указанный канал
    /// 
    /// # Аргументы
    /// * `channel` - Номер канала (0, 1 или 2)
    fn switch_to_channel(&mut self, channel: u8) -> Result<(), Box<dyn Error>> {
        if channel > 2 {
            return Err("Invalid channel number".into());
        }

        self.current_channel = channel;
        self.keyboard_cmd.channel = self.current_channel;
        self.mouse_cmd.channel = self.current_channel;

        const MAX_RETRIES: u8 = 3;
        let mut retry_count = 0;

        while retry_count < MAX_RETRIES {
            match self.send_commands() {
                Ok(_) => {
                    println!("Переключено на канал {}", self.current_channel);
                    
                    #[cfg(target_os = "macos")]
                    {
                        notify(&format!("Переключено на канал {}", self.current_channel + 1));
                    }
                    
                    #[cfg(not(target_os = "macos"))]
                    {
                        if env::var("DISPLAY").is_ok() {
                            if let Err(e) = Notification::new()
                                .summary("Канал переключен")
                                .body(&format!("Logitech переключен на {}", self.current_channel + 1))
                                .timeout(3000)
                                .show() {
                                debug!("Ошибка отправки уведомления: {}", e);
                            }
                        }
                    }
                    
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    eprintln!("Попытка {}/{}: Ошибка отправки команды: {}", retry_count, MAX_RETRIES, e);
                    if retry_count < MAX_RETRIES {
                        thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        }

        Err("Не удалось переключить канал после нескольких попыток".into())
    }

    fn reopen_device(&mut self) -> Result<(), Box<dyn Error>> {
        let api = HidApi::new()?;
        
        // Ищем устройство с нужными параметрами
        let mut target_device = None;
        for device in api.device_list() {
            if device.vendor_id() == VID && 
               device.product_id() == PID && 
               device.usage() == USAGE && 
               device.usage_page() == USAGE_PAGE {
                target_device = Some(device.path().to_string_lossy().into_owned());
                break;
            }
        }

        // Открываем устройство по найденному пути
        if let Some(path) = target_device {
            self.hid_device = api.open_path(std::ffi::CString::new(path)?.as_ref())?;
            Ok(())
        } else {
            Err("Устройство не найдено при переоткрытии".into())
        }
    }

    fn send_commands(&mut self) -> Result<(), Box<dyn Error>> {
        const MAX_RETRIES: u8 = 3;
        let mut retry_count = 0;

        while retry_count < MAX_RETRIES {
            if retry_count > 0 {
                if let Err(e) = self.reopen_device() {
                    debug!("Ошибка переоткрытия устройства: {}", e);
                    thread::sleep(Duration::from_millis(1000));
                    continue;
                }
                thread::sleep(Duration::from_millis(500));
            }

            // Отправляем команды клавиатуре с обоими индексами
            for keyboard_index in [0x01, 0x02] {
                self.keyboard_cmd.index = keyboard_index;
                let keyboard_bytes = self.keyboard_cmd.to_bytes();
                debug!("Отправка команды клавиатуре (индекс {}, длина {}): {}", 
                    keyboard_index,
                    keyboard_bytes.len(),
                    keyboard_bytes.iter()
                        .map(|b| format!("0x{:02X}", b))
                        .collect::<Vec<String>>()
                        .join(","));
                
                if let Err(e) = self.hid_device.write(&keyboard_bytes) {
                    retry_count += 1;
                    debug!("Попытка {}/{}: Ошибка отправки команды клавиатуре: {}", retry_count, MAX_RETRIES, e);
                    continue;
                }
                thread::sleep(Duration::from_millis(500));
            }

            // Отправляем команды мыши с обоими индексами
            for mouse_index in [0x01, 0x02] {
                self.mouse_cmd.index = mouse_index;
                let mouse_bytes = self.mouse_cmd.to_bytes();
                debug!("Отправка команды мыши (индекс {}, длина {}): {}", 
                    mouse_index,
                    mouse_bytes.len(),
                    mouse_bytes.iter()
                        .map(|b| format!("0x{:02X}", b))
                        .collect::<Vec<String>>()
                        .join(","));

                if let Err(e) = self.hid_device.write(&mouse_bytes) {
                    retry_count += 1;
                    debug!("Попытка {}/{}: Ошибка отправки команды мыши: {}", retry_count, MAX_RETRIES, e);
                    continue;
                }
                thread::sleep(Duration::from_millis(500));
            }

            return Ok(());
        }

        Err("Не удалось отправить команды после нескольких попыток".into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        DEBUG = env::var("DEBUG").is_ok();
    }

    let mut switcher = ChannelSwitcher::new()?;
    
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::os::unix::io::AsRawFd;
        use std::path::Path;
        
        debug!("Поиск клавиатуры...");
        
        let mut last_press: Option<(u16, Instant)> = None;
        let double_press_threshold = Duration::from_millis(500);

        for path in Path::new("/dev/input/by-id").read_dir()? {
            if let Ok(path) = path {
                let path_str = path.path().to_string_lossy().to_string();
                debug!("Проверка устройства: {}", path_str);
                
                if path_str.contains("kbd") {
                    debug!("Найдена клавиатура: {}", path_str);
                    if let Ok(file) = File::open(&path.path()) {
                        let fd = file.as_raw_fd();
                        println!("Переключатель каналов запущен. Используйте двойное нажатие клавиш 1, 2 или 3.");
                        
                        let mut event = input_linux::InputEvent { 
                            time: input_linux::EventTime::new(0, 0),
                            kind: input_linux::EventKind::Key,
                            code: 0,
                            value: 0,
                        };
                        loop {
                            if unsafe { libc::read(fd, &mut event as *mut _ as *mut libc::c_void, std::mem::size_of::<input_linux::InputEvent>()) } > 0 {
                                if event.kind == input_linux::EventKind::Key {
                                    if event.value == 0 { // 0 = Released
                                        let current_press = match event.code {
                                            2 => Some((2, 0)), // KEY_1
                                            3 => Some((3, 1)), // KEY_2
                                            4 => Some((4, 2)), // KEY_3
                                            _ => {
                                                last_press = None; // Сброс при нажатии любой другой клавиши
                                                None
                                            }
                                        };

                                        if let Some((key_code, channel)) = current_press {
                                            if let Some((last_key, last_time)) = last_press {
                                                if key_code == last_key && Instant::now() - last_time <= double_press_threshold {
                                                    switcher.switch_to_channel(channel)?;
                                                    last_press = None;
                                                    thread::sleep(Duration::from_millis(300));
                                                    continue;
                                                }
                                            }
                                            last_press = Some((key_code, Instant::now()));
                                        }
                                    }
                                }
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            }
        }
        return Err("Клавиатура не найдена".into());
    }

    #[cfg(not(target_os = "linux"))]
    {
        use device_query::{DeviceQuery, DeviceState, Keycode};

        let device_state = DeviceState::new();
        let mut last_press: Option<(Keycode, Instant)> = None;
        let mut key_released = true;
        let double_press_threshold = Duration::from_millis(500);

        println!("Переключатель каналов запущен. Используйте двойное нажатие клавиш 1, 2 или 3.");

        loop {
            let keys: Vec<Keycode> = device_state.get_keys();
            
            if keys.is_empty() && !key_released {
                key_released = true;
            } else if !keys.is_empty() && key_released {
                key_released = false;
                let current_key = if keys.contains(&Keycode::Key1) {
                    Some((Keycode::Key1, 0))
                } else if keys.contains(&Keycode::Key2) {
                    Some((Keycode::Key2, 1))
                } else if keys.contains(&Keycode::Key3) {
                    Some((Keycode::Key3, 2))
                } else {
                    last_press = None;
                    None
                };

                if let Some((key, channel)) = current_key {
                    if let Some((last_key, last_time)) = last_press {
                        if key == last_key && Instant::now() - last_time <= double_press_threshold {
                            switcher.switch_to_channel(channel)?;
                            last_press = None;
                            thread::sleep(Duration::from_millis(300));
                            continue;
                        }
                    }
                    last_press = Some((key, Instant::now()));
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}