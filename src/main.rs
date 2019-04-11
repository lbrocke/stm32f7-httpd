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

use alloc::{str, string::ToString, vec::Vec};
use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout as AllocLayout;
use core::cell::RefCell;
use core::panic::PanicInfo;
use cortex_m::asm;
use cortex_m_rt::{entry, exception};
use log::{debug, error, info};
use smoltcp::wire::{EthernetAddress, IpAddress, Ipv4Address};
use stm32f7::stm32f7x6::{CorePeripherals, Peripherals};
use stm32f7_discovery::gpio::{GpioPort, OutputPin};
use stm32f7_discovery::{init, touch};
use stm32f7_discovery::lcd::{self, Color, Framebuffer, Layer};
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

    // Initialize I2C and touch
    let mut i2c_3 = init::init_i2c_3(peripherals.I2C3, &mut rcc);
    touch::check_family_id(&mut i2c_3).expect("Could not initialize touch");

    // Wrap layer 1 so it can be used in request_handler and touch handler
    let layer_wrapper = RefCell::new(layer_1);
    // Pixels to send next time the client polls
    let new_pixels = RefCell::new(vec![]);

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
                let pins_body =
                    format!("{{ \"led\": {led}, \"backlight\": {backlight}, \"display_enable\": {display_enable} }}\n",
                        led = pins.led.get(),
                        backlight = pins.backlight.get(),
                        display_enable = pins.display_enable.get()
                    );

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
            .route("POST", "/pixels", |_req, _args| {
                let mut body_iter = body.iter();

                while let (Some(x), Some(y)) = (body_iter.next(), body_iter.next()) {
                    debug!("Setting ({}|{})", x, y);
                    draw_pixel(&mut layer_wrapper.borrow_mut(), *x as usize, *y as usize);
                }

                let mut pixel_data = vec![];
                
                for (x, y) in new_pixels.borrow().iter() {
                    pixel_data.push(*x);
                    pixel_data.push(*y);
                }

                let res = ResponseBuilder::new(Status::OK)
                    .header("Content-Type", "application/octet-stream")
                    .body(pixel_data)
                    .finalize();

                new_pixels.borrow_mut().clear();
                res
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
        // server "middleware"
        // sets CORS headers, Server header and prints a nice message
        |req: &Request, body: &Vec<u8>| {
            let mut res = request_handler(req, body);
            res.headers
                .insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
            res.headers.insert(
                "Server".to_string(),
                format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            );

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

        // poll for touch events and draw stuff
        for touch_event in &touch::touches(&mut i2c_3).expect("Could not read touch events") {
            let px_x = touch_event.x / 4;
            let px_y = touch_event.y / 4;
            draw_pixel(&mut layer_wrapper.borrow_mut(), px_x as usize, px_y as usize);

            let px = (px_x as u8, px_y as u8);

            if !new_pixels.borrow().contains(&px) {
                new_pixels.borrow_mut().push(px);
            }
        }
    }
}

fn draw_pixel<T: Framebuffer>(layer: &mut Layer<T>, x: usize, y: usize) {
    let w = lcd::WIDTH;
    let h = lcd::HEIGHT;

    let max_x = w / 4;
    let max_y = h / 4;

    if x < max_x && y < max_y {
        for j in 0..4 {
            for i in 0..4 {
                layer.print_point_color_at(x * 4 + i, y * 4 + j, Color::rgb(0, 0, 0));
            }
        }
    }
}

// some functions that are required because we're not using std

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
