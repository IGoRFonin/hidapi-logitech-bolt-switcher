use hidapi::{HidApi, HidDevice};
use device_query::{DeviceQuery, DeviceState, Keycode};
use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};
use std::env;
use notify_rust::Notification;

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

        // Обновляем канал в командах
        self.keyboard_cmd.channel = self.current_channel;
        self.mouse_cmd.channel = self.current_channel;
        // println!("{:?}", self.keyboard_cmd);
        // Добавляем несколько попыток отправки команд
        const MAX_RETRIES: u8 = 3;
        let mut retry_count = 0;

        while retry_count < MAX_RETRIES {
            match self.send_commands() {
                Ok(_) => {
                    println!("Переключено на канал {}", self.current_channel);
                    if let Err(e) = Notification::new()
                        .summary("Канал переключен")
                        .body(&format!("Logitech переключен на ПК {}", self.current_channel + 1))
                        .timeout(3000) // 3 секунды
                        .show() {
                        debug!("Ошибка отправки уведомления: {}", e);
                    }
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    eprintln!("Попытка {}/{}: Ошибка отправки команды: {}", retry_count, MAX_RETRIES, e);
                    if retry_count < MAX_RETRIES {
                        thread::sleep(Duration::from_millis(500)); // Увеличенная задержка между попытками
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

            let keyboard_bytes = self.keyboard_cmd.to_bytes();
            debug!("Отправка команды клавиатуре (длина {}): {}", 
                keyboard_bytes.len(),
                keyboard_bytes.iter()
                    .map(|b| format!("0x{:02X}", b))
                    .collect::<Vec<String>>()
                    .join(","));
            
            match self.hid_device.write(&keyboard_bytes) {
                Ok(_) => {
                    thread::sleep(Duration::from_millis(500));
                    
                    let mouse_bytes = self.mouse_cmd.to_bytes();
                    debug!("Отправка команды мыши (длина {}): {}", 
                        mouse_bytes.len(),
                        mouse_bytes.iter()
                            .map(|b| format!("0x{:02X}", b))
                            .collect::<Vec<String>>()
                            .join(","));

                    match self.hid_device.write(&mouse_bytes) {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            retry_count += 1;
                            debug!("Попытка {}/{}: Ошибка отправки команды мыши: {}", retry_count, MAX_RETRIES, e);
                        }
                    }
                }
                Err(e) => {
                    retry_count += 1;
                    debug!("Попытка {}/{}: Ошибка отправки команды клавиатуре: {}", retry_count, MAX_RETRIES, e);
                }
            }
        }

        Err("Не удалось отправить команды после нескольких попыток".into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Проверяем переменную окружения для debug режима
    unsafe {
        DEBUG = env::var("DEBUG").is_ok();
    }

    let mut switcher = ChannelSwitcher::new()?;
    let device_state = DeviceState::new();
    
    let mut last_press = None;
    let mut key_released = true;
    let double_press_threshold = Duration::from_millis(500);

    println!("Переключатель каналов запущен.");
    println!("Быстро нажмите 1, 2 или 3 два раза для переключения каналов (0, 1, 2 соответственно)");
    println!("Нажмите Ctrl+C для выхода.");

    loop {
        let keys: Vec<Keycode> = device_state.get_keys();
        
        if !keys.is_empty() {
            debug!("Нажаты клавиши: {:?}", keys);
        }

        let current_press = if keys.is_empty() {
            key_released = true;
            None
        } else if key_released {
            key_released = false;
            if keys.contains(&Keycode::Key1) {
                Some((0, Instant::now()))
            } else if keys.contains(&Keycode::Key2) {
                Some((1, Instant::now()))
            } else if keys.contains(&Keycode::Key3) {
                Some((2, Instant::now()))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((channel, time)) = current_press {
            debug!("Обнаружено нажатие канала {}", channel);
            if let Some((last_channel, last_time)) = last_press {
                debug!("Предыдущее нажатие: канал {}, время {:?}", last_channel, time - last_time);
                if channel == last_channel && time - last_time <= double_press_threshold {
                    debug!("Обнаружено двойное нажатие для канала {}", channel);
                    if let Err(e) = switcher.switch_to_channel(channel) {
                        eprintln!("Ошибка переключения канала: {}", e);
                    }
                    last_press = None;
                    thread::sleep(Duration::from_millis(300));
                    continue;
                }
            }
            last_press = Some((channel, time));
        }

        thread::sleep(Duration::from_millis(50));
    }
}