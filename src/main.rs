#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::time::Duration as CoreDuration;
use core::cell::RefCell;
use critical_section::Mutex;

// display and graphics imports
use embedded_graphics::{
    pixelcolor::Rgb565, prelude::*,
};
use display_interface_spi::SPIInterfaceNoCS;
mod embassy_task_st7789;
use embassy_task_st7789::EmbassyTaskDisplay;
use mipidsi::{ColorOrder, Orientation, ColorInversion};

// esp-box UI elements imports
use esp_box_ui::{
    sensor_data::{SensorData, SensorType, update_sensor_data},
    build_sensor_ui,
    food_item::{ FoodItem, update_field, draw_buy_button },
    build_inventory,
};

// peripherals imports
use hal::{
    clock::ClockControl,
    adc::{self, AdcConfig, Attenuation, ADC, ADC1},
    i2c::I2C,
    spi::{
        master::Spi,
        SpiMode,
    },
    peripherals::Peripherals,
    prelude::{_fugit_RateExtU32, *},
    timer::TimerGroup,
    IO, Delay,
    embassy,
};

// embassy imports
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use bme680::*;

use heapless::String;
use core::fmt::Write;

use esp_backtrace as _;
use esp_println::println;

static TEMPERATURE_DATA: Mutex<RefCell<SensorData>> = Mutex::new(RefCell::new(SensorData { sensor_type: SensorType::Temperature, pos_x: 35, value: 0.0 }));
static HUMIDITY_DATA: Mutex<RefCell<SensorData>> = Mutex::new(RefCell::new(SensorData { sensor_type: SensorType::Humidity, pos_x: 120, value: 0.0 }));
static PRESSURE_DATA: Mutex<RefCell<SensorData>> = Mutex::new(RefCell::new(SensorData {sensor_type: SensorType::Pressure, pos_x: 205, value: 0.0 }));

static HOTDOG: Mutex<RefCell<FoodItem>> = Mutex::new(RefCell::new(FoodItem { name: "Hotdog", pos_y: 17, amount: 10, price: 2.50, highlighted: true, purchased: false }));
static SANDWICH: Mutex<RefCell<FoodItem>> = Mutex::new(RefCell::new(FoodItem { name: "Sandwich", pos_y: 87, amount: 9, price: 3.50, highlighted: false, purchased: false }));
static ENERGY_DRINK: Mutex<RefCell<FoodItem>> = Mutex::new(RefCell::new(FoodItem { name: "Energy Drink", pos_y: 157, amount: 11, price: 2.00, highlighted: false, purchased: false }));

enum Selection {
    Hotdog,
    Sandwich,
    EnergyDrink,
}

static CURRENT_SELECTION: Mutex<RefCell<Selection>> = Mutex::new(RefCell::new(Selection::Hotdog));


#[main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
    );

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    embassy::init(
        &clocks,
        timer_group0,
    );
    
    let sclk = io.pins.gpio7;
    let mosi = io.pins.gpio6;
    let miso = io.pins.gpio2;
    let cs = io.pins.gpio5;

    let dc = io.pins.gpio4.into_push_pull_output();
    let mut backlight = io.pins.gpio45.into_push_pull_output();
    let reset = io.pins.gpio48.into_push_pull_output();

    let spi = Spi::new(
        peripherals.SPI2,
        40u32.MHz(),
        SpiMode::Mode0,
        &clocks,
    ).with_pins(
        Some(sclk),
        Some(mosi),
        Some(miso),
        Some(cs),
    );

    let di = SPIInterfaceNoCS::new(spi, dc);
    delay.delay_ms(500u32);

    let mut display_struct = EmbassyTaskDisplay {
        display: match mipidsi::Builder::st7789(di)
            .with_display_size(240, 320)
            .with_orientation(Orientation::LandscapeInverted(true))
            .with_color_order(ColorOrder::Rgb)
            .with_invert_colors(ColorInversion::Inverted)
            .init(&mut delay, Some(reset)) {
            Ok(display) => display,
            Err(e) => {
                println!("Display initialization failed: {:?}", e);
                panic!("Display initialization failed");
            }
        },
    };

    backlight.set_low().unwrap();

    display_struct.display.clear(Rgb565::WHITE).unwrap();

    let hotdog = critical_section::with(|cs| HOTDOG.borrow(cs).borrow().clone());
    let sandwich = critical_section::with(|cs| SANDWICH.borrow(cs).borrow().clone());
    let energy_drink = critical_section::with(|cs| ENERGY_DRINK.borrow(cs).borrow().clone());

    build_inventory(
        &mut display_struct.display,
        &hotdog,
        &sandwich,
        &energy_drink,
    );

    update_field(&mut display_struct.display, &hotdog);
    update_field(&mut display_struct.display, &sandwich);
    update_field(&mut display_struct.display, &energy_drink);

    // Create ADC instances
    let analog = peripherals.SENS.split();

    let mut adc1_config = AdcConfig::new();

    let atten = Attenuation::Attenuation11dB;

    type AdcCal = adc::AdcCalCurve<ADC1>;

    let pin = adc1_config.enable_pin_with_cal::<_, AdcCal>(io.pins.gpio1.into_analog(), atten);

    let adc1 = ADC::<ADC1>::adc(analog.adc1, adc1_config).unwrap();
    
    spawner.spawn(button_handling_task(adc1, pin, display_struct)).ok();

    let i2c = I2C::new(
        peripherals.I2C0,
        io.pins.gpio41,
        io.pins.gpio40,
        100u32.kHz(),
        &clocks,
    );

    //initialize BME680
    let mut bme = Bme680::init(i2c, &mut delay, I2CAddress::Primary).expect("Failed to initialize Bme680");
    println!("I got here");
    let settings = SettingsBuilder::new()
        .with_humidity_oversampling(OversamplingSetting::OS2x)
        .with_pressure_oversampling(OversamplingSetting::OS4x)
        .with_temperature_oversampling(OversamplingSetting::OS8x)
        .with_temperature_filter(IIRFilterSize::Size3)
        .with_gas_measurement(CoreDuration::from_millis(1500), 320, 25)
        .with_run_gas(true)
        .build();
    bme.set_sensor_settings(&mut delay, settings).expect("Failed to set the settings");

    loop {
        bme.set_sensor_mode(&mut delay, PowerMode::ForcedMode).expect("Failed to set sensor mode");

        let profile_duration = bme.get_profile_dur(&settings.0).expect("Failed to get profile duration");
        let duration_ms = profile_duration.as_millis() as u32;
        delay.delay_ms(duration_ms);

        let (data, _state) = bme.get_sensor_data(&mut delay).expect("Failed to get sensor data");
        
        let temp = data.temperature_celsius();
        let hum = data.humidity_percent();
        let pres = data.pressure_hpa();
        let gas = data.gas_resistance_ohm();

        critical_section::with(|cs| {
            TEMPERATURE_DATA.borrow(cs).borrow_mut().value = temp;
            HUMIDITY_DATA.borrow(cs).borrow_mut().value = hum;
            PRESSURE_DATA.borrow(cs).borrow_mut().value = pres;
        });

        println!("|========================|");
        println!("| Temperature {:.2}°C    |", temp);
        println!("| Humidity {:.2}%        |", hum);
        println!("| Pressure {:.2}hPa     |", pres);
        println!("| Gas Resistance {:.2}Ω ", gas);
        println!("|========================|");

        sleep(60000).await;
    }
    loop {}
}

pub async fn sleep(millis: u32) {
    Timer::after(Duration::from_millis(millis as u64)).await;
}

const LEFT_BUTTON_RANGE: (u16, u16) = (2680, 2720); // Range for left-most button
const MIDDLE_BUTTON_RANGE: (u16, u16) = (2130, 2170); // Range for middle button
const RIGHT_BUTTON_RANGE: (u16, u16) = (705, 745); // Range for right-most button

#[embassy_executor::task]
async fn button_handling_task
(
    mut adc1: ADC<'static, ADC1>, 
    mut pin: adc::AdcPin<hal::gpio::GpioPin<hal::gpio::Analog, 1>, ADC1, adc::AdcCalCurve<ADC1>>,
    mut display_struct: EmbassyTaskDisplay<'static>,
) {
    
    let mut is_left_button_pressed = false;

    loop {
        let pin_mv = nb::block!(adc1.read(&mut pin)).unwrap();
        if (LEFT_BUTTON_RANGE.0..=LEFT_BUTTON_RANGE.1).contains(&pin_mv) {
            if !is_left_button_pressed {
                is_left_button_pressed = true;

                let temperature_data = critical_section::with(|cs| TEMPERATURE_DATA.borrow(cs).borrow().clone());
                let humidity_data = critical_section::with(|cs| HUMIDITY_DATA.borrow(cs).borrow().clone());
                let pressure_data = critical_section::with(|cs| PRESSURE_DATA.borrow(cs).borrow().clone());

                build_sensor_ui(&mut display_struct.display, &temperature_data, &humidity_data, &pressure_data);
                update_sensor_data(&mut display_struct.display, &temperature_data);
                update_sensor_data(&mut display_struct.display, &humidity_data);
                update_sensor_data(&mut display_struct.display, &pressure_data);
                
            }
        } else {
            if is_left_button_pressed {
                is_left_button_pressed = false;

                display_struct.display.clear(Rgb565::WHITE).unwrap();

                let hotdog = critical_section::with(|cs| HOTDOG.borrow(cs).borrow().clone());
                let sandwich = critical_section::with(|cs| SANDWICH.borrow(cs).borrow().clone());
                let energy_drink = critical_section::with(|cs| ENERGY_DRINK.borrow(cs).borrow().clone());

                build_inventory(&mut display_struct.display, &hotdog, &sandwich, &energy_drink);
                update_field(&mut display_struct.display, &hotdog);
                update_field(&mut display_struct.display, &sandwich);
                update_field(&mut display_struct.display, &energy_drink);
            }
        }

        if (MIDDLE_BUTTON_RANGE.0..=MIDDLE_BUTTON_RANGE.1).contains(&pin_mv) {
            critical_section::with(|cs| {
                let mut current_selection = CURRENT_SELECTION.borrow(cs).borrow_mut();
                let mut hotdog = HOTDOG.borrow(cs).borrow_mut();
                let mut sandwich = SANDWICH.borrow(cs).borrow_mut();
                let mut energy_drink = ENERGY_DRINK.borrow(cs).borrow_mut();

                match *current_selection {
                    Selection::Hotdog => {
                        hotdog.highlighted = false;
                        sandwich.highlighted = true;
                        energy_drink.highlighted = false;
                        *current_selection = Selection::Sandwich;
                        println!("Sandwich selected");
                    },
                    Selection::Sandwich => {
                        hotdog.highlighted = false;
                        sandwich.highlighted = false;
                        energy_drink.highlighted = true;
                        *current_selection = Selection::EnergyDrink;
                        println!("Energy Drink selected");
                    },
                    Selection::EnergyDrink => {
                        energy_drink.highlighted = false;
                        hotdog.highlighted = true;
                        sandwich.highlighted = false;
                        *current_selection = Selection::Hotdog;
                        println!("Hotdog selected");
                    },
                }
                build_inventory(&mut display_struct.display, &hotdog, &sandwich, &energy_drink);
                update_field(&mut display_struct.display, &hotdog);
                update_field(&mut display_struct.display, &sandwich);
                update_field(&mut display_struct.display, &energy_drink);
            });
        }

        if (RIGHT_BUTTON_RANGE.0..=RIGHT_BUTTON_RANGE.1).contains(&pin_mv) {
            critical_section::with(|cs| {
                let current_selection = CURRENT_SELECTION.borrow(cs).borrow();
                match *current_selection {
                    Selection::Hotdog => {
                        let mut hotdog = HOTDOG.borrow(cs).borrow_mut();
                        if hotdog.amount > 0 {
                            hotdog.amount -= 1;
                            hotdog.purchased = true;
                            let mut hotdog_amount: String<32> = String::new();
                            write!(hotdog_amount, "{}", hotdog.amount).expect("write! failed!");
                            println!("Bought one Hotdog!");
                            draw_buy_button(&mut display_struct.display, &hotdog);
                            sleep(1000);
                            hotdog.purchased = false;
                        } else {
                            println!("Hotdog is out of stock!");
                        }
                    },
                    Selection::Sandwich => {
                        let mut sandwich = SANDWICH.borrow(cs).borrow_mut();
                        if sandwich.amount > 0 {
                            sandwich.amount -= 1;
                            sandwich.purchased = true;
                            println!("Bought one Sandwich!");
                            draw_buy_button(&mut display_struct.display, &sandwich);
                            sleep(1000);
                            sandwich.purchased = false;
                        } else {
                            println!("Sandwich is out of stock!");
                        }
                    },
                    Selection::EnergyDrink => {
                        let mut energy_drink = ENERGY_DRINK.borrow(cs).borrow_mut();
                        if energy_drink.amount > 0 {
                            energy_drink.amount -= 1;
                            energy_drink.purchased = true;
                            println!("Bought one Energy Drink!");
                            draw_buy_button(&mut display_struct.display, &energy_drink);
                            sleep(1000);
                            energy_drink.purchased = false;
                        } else {
                            println!("Energy Drink is out of stock!");
                        }
                    },
                }
                let hotdog = HOTDOG.borrow(cs).borrow().clone();
                let sandwich = SANDWICH.borrow(cs).borrow().clone();
                let energy_drink = ENERGY_DRINK.borrow(cs).borrow().clone();
                build_inventory(&mut display_struct.display, &hotdog, &sandwich, &energy_drink);
                update_field(&mut display_struct.display, &hotdog);
                update_field(&mut display_struct.display, &sandwich);
                update_field(&mut display_struct.display, &energy_drink);
            }); 
        }

        sleep(100).await;
    }
}