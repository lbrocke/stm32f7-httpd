//! A simple HTTP server for the STM32F7-Discovery board

#![warn(clippy::all)]
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(alloc)]

#[macro_use]
extern crate alloc;

mod httpd;
mod logger;

use alloc::{str, vec::Vec};
use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout as AllocLayout;
use core::panic::PanicInfo;
use cortex_m::asm;
use cortex_m_rt::{entry, exception};
use log::{debug, error, info};
use smoltcp::wire::{EthernetAddress, IpAddress, Ipv4Address};
use stm32f7::stm32f7x6::{CorePeripherals, Peripherals};
use stm32f7_discovery::gpio::{GpioPort, OutputPin};
use stm32f7_discovery::init;
use stm32f7_discovery::lcd::{self, Color};
use stm32f7_discovery::system_clock::{self, Hz};

use httpd::{Request, ResponseBuilder, Routes, Status, HTTPD};

const SYSTICK: Hz = Hz(20);

const HEAP_SIZE: usize = 50 * 1024;

const ETH_ADDR: EthernetAddress = EthernetAddress([0x00, 0x08, 0xDC, 0xAB, 0xCD, 0xEF]);
const IP_ADDR: IpAddress = IpAddress::Ipv4(Ipv4Address([192, 168, 1, 42]));
const PORT: u16 = 80;

const INDEX_PAGE: &str = include_str!("httpd/index.html");
const NOTFOUND_PAGE: &str = include_str!("httpd/notfound.html");

#[entry]
fn main() -> ! {
    let core_peripherals = CorePeripherals::take().unwrap();
    let peripherals = Peripherals::take().unwrap();

    let mut systick = core_peripherals.SYST;
    let mut rcc = peripherals.RCC;
    let mut pwr = peripherals.PWR;
    let mut flash = peripherals.FLASH;
    let mut fmc = peripherals.FMC;
    let mut ltdc = peripherals.LTDC;
    let mut syscfg = peripherals.SYSCFG;
    let mut ethernet_mac = peripherals.ETHERNET_MAC;
    let ethernet_dma = peripherals.ETHERNET_DMA;

    // wake up Cortex M7 from low power state
    init::init_system_clock_216mhz(&mut rcc, &mut pwr, &mut flash);

    // init systick
    init::init_systick(SYSTICK, &mut systick, &rcc);
    systick.enable_interrupt();

    // initialize GPIO
    init::enable_gpio_ports(&mut rcc);
    let mut pins = init::pins(
        GpioPort::new(peripherals.GPIOA),
        GpioPort::new(peripherals.GPIOB),
        GpioPort::new(peripherals.GPIOC),
        GpioPort::new(peripherals.GPIOD),
        GpioPort::new(peripherals.GPIOE),
        GpioPort::new(peripherals.GPIOF),
        GpioPort::new(peripherals.GPIOG),
        GpioPort::new(peripherals.GPIOH),
        GpioPort::new(peripherals.GPIOI),
        GpioPort::new(peripherals.GPIOJ),
        GpioPort::new(peripherals.GPIOK),
    );

    // initialize LCD display
    init::init_sdram(&mut rcc, &mut fmc);
    let mut lcd = init::init_lcd(&mut ltdc, &mut rcc);

    // enable display and backlight
    pins.display_enable.set(true);
    pins.backlight.set(true);

    // Set text color (white)
    lcd.set_color_lookup_table(255, Color::rgb(255, 255, 255));

    // set up layers
    let mut layer_1 = lcd.layer_1().unwrap();
    let mut layer_2 = lcd.layer_2().unwrap();
    layer_1.clear();
    layer_2.clear();

    // set stdout to layer 2
    lcd::init_stdout(layer_2);

    logger::init().unwrap();

    unsafe { ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE) }

    // Gets called on each request

    let mut request_handler = |req: &Request, body: &Vec<u8>| {
        Routes::init(req)
            // Frontend route
            .route("GET", "/", |_req, _args| {
                ResponseBuilder::new(Status::OK)
                    .body_html(INDEX_PAGE)
                    .finalize()
            })
            // API routes
            .route("GET", "/pins", |_req, _args| {
                let pins_body = format!("{{\"led\": {}}}\n", pins.led.get());

                ResponseBuilder::new(Status::OK)
                    .header("Content-Type", "application/json")
                    .body(pins_body.as_bytes().to_vec())
                    .finalize()
            })
            .route("POST", "/pins/:name", |_req, args| {
                let pin_to_toggle: Option<&mut OutputPin> = match args.get("name").unwrap().as_str() {
                    "led" => Some(&mut pins.led),
                    "backlight" => Some(&mut pins.backlight),
                    "display_enable" => Some(&mut pins.display_enable),
                    _ => None
                };

                let state_to_set =
                    str::from_utf8(body)
                    .ok()
                    .and_then(|state_string| {
                        if state_string == "1" {
                            Some(true)
                        } else if state_string == "0" {
                            Some(false)
                        } else {
                            None
                        }
                    });

                match (pin_to_toggle, state_to_set) {
                    (Some(pin), Some(state)) => {
                        pin.set(state);
                        ResponseBuilder::new(Status::OK)
                            .finalize()
                    },
                    _ => ResponseBuilder::new(Status::BadRequest).finalize(),
                }
            })
            .catch_all(|_req, _args| {
                ResponseBuilder::new(Status::NotFound)
                    .body_html(NOTFOUND_PAGE)
                    .finalize()
            })
    };

    let mut server = HTTPD::new(
        &mut rcc,
        &mut syscfg,
        &mut ethernet_mac,
        ethernet_dma,
        ETH_ADDR,
        IP_ADDR,
        PORT,
        |req: &Request, body: &Vec<u8>| {
            let res = request_handler(req, body);
            let req_text = format!("{} {}", req.method(), req.path());
            let (status_num, status_txt) = res.status.numerical_and_text();
            let res_text = format!("[{} {}]", status_num, status_txt);
            let empty_space = 50 - req_text.len() - res_text.len();

            info!("{}{}{}", req_text, " ".repeat(empty_space), res_text);

            res
        },
    )
    .expect("HTTPD initialisation failed");

    info!("Server initialized on {}:{}", IP_ADDR, PORT);

    info!("Entering loop");

    loop {
        // poll packets and answer them
        server.poll();
    }
}

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[alloc_error_handler]
fn oom_handler(_: AllocLayout) -> ! {
    error!("I have no memory of this");
    asm::bkpt();
    loop {}
}

#[exception]
fn SysTick() {
    system_clock::tick();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("Don't panic: {:?}", info);
    asm::bkpt();
    loop {}
}
