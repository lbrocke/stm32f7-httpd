#![no_std]
#![no_main]
#![feature(alloc)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
};
use alloc_cortex_m::CortexMHeap;
use core::{
    alloc::Layout,
    panic::PanicInfo,
};
use cortex_m_rt::{entry, exception};
use stm32f7::stm32f7x6::{CorePeripherals, Peripherals};
use stm32f7_discovery::{
    print,
    println,
    gpio::{GpioPort, OutputPin},
    init,
    lcd::{self, Color},
    system_clock::{self, Hz}
};
use stm32f7_httpd::httpd::{
    self,
    Request,
    Response,
    Routes
};
use smoltcp::wire::{
    EthernetAddress,
    IpAddress,
    Ipv4Address
};

const SYSTICK_FREQ: Hz = Hz(20);
const HEAP_SIZE: usize = 50 * 1024;

const ETHER_ADDRESS: EthernetAddress = EthernetAddress([0x00, 0x08, 0xDC, 0xAB, 0xCD, 0xEF]);
const IP_ADDRESS: IpAddress = IpAddress::Ipv4(Ipv4Address([192, 168, 1, 42]));
const PORT: u16 = 80;

const PAGE_INDEX: &str = include_str!("wiki/index.html");
const PAGE_VIEW: &str = include_str!("wiki/view.html");
const PAGE_NOTFOUND: &str = include_str!("wiki/notfound.html");

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

    // Wake up the processor from low power state
    init::init_system_clock_216mhz(&mut rcc, &mut pwr, &mut flash);

    // Initialize timer interrupts that increase the systick
    init::init_systick(SYSTICK_FREQ, &mut systick, &rcc);
    systick.enable_interrupt();

    // Initializes GPIO pins
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

    // Initialize LCD
    init::init_sdram(&mut rcc, &mut fmc);
    let mut lcd = init::init_lcd(&mut ltdc, &mut rcc);
    pins.display_enable.set(true);
    pins.backlight.set(true);

    // Set text color to white
    lcd.set_color_lookup_table(255, Color::rgb(255, 255, 255));

    let mut layer_1 = lcd.layer_1().unwrap();
    let mut layer_2 = lcd.layer_2().unwrap();
    layer_1.clear();
    layer_2.clear();

    lcd::init_stdout(layer_2);

    println!("Initializing heap...");

    // Initialize heap (httpd needs heap, sorry ._.)
    unsafe { ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE) }

    println!("Initializing server...");

    let mut pages = BTreeMap::new();

    pages.insert("stuff", "<p>Hello, <em>World</em>!</p>");

    // Initialize server (doesn't really do anything yet)
    let mut server = httpd::HTTPD::new(
        &mut rcc,
        &mut syscfg,
        &mut ethernet_mac,
        ethernet_dma,
        ETHER_ADDRESS,
        IP_ADDRESS,
        PORT,
        |request: &Request, _body| {
            println!("Got {} request on {}", request.method(), request.path());

            Routes::init(request)
            .route("GET", "/", |_request, _args| {
                let mut links = String::new();
                for (key, _content) in pages.iter() {
                    links.push_str(&format!("<li><a href=\"/view/{}\">{}</a></li>", key, key));
                }
                let source = PAGE_INDEX.replace("{{links}}", &links);

                let mut headers = BTreeMap::new();
                headers.insert("Content-Type".to_string(), "text/html".to_string());
                headers.insert("Content-Length".to_string(), source.len().to_string());

                Response::new(
                    httpd::Status::OK,
                    headers,
                    source.as_bytes().to_vec()
                )
            })
            .route("GET", "/view/:page_name", |_request, args| {
                let source = match pages.get(args.get("page_name").unwrap().as_str()) {
                    None => PAGE_NOTFOUND.to_string(),
                    Some(content) => {
                        PAGE_VIEW
                            .replace("{{title}}", args.get("page_name").unwrap())
                            .replace("{{content}}", content)
                    }
                };

                let mut headers = BTreeMap::new();
                headers.insert("Content-Type".to_string(), "text/html".to_string());
                headers.insert("Content-Length".to_string(), source.len().to_string());

                Response::new(
                    httpd::Status::OK,
                    headers,
                    source.as_bytes().to_vec()
                )
            })
            .catch_all(|_request, _args| {
                let mut headers = BTreeMap::new();
                headers.insert("Content-Type".to_string(), "text/html".to_string());
                headers.insert("Content-Length".to_string(), PAGE_NOTFOUND.len().to_string());

                Response::new(
                    httpd::Status::NotFound,
                    headers,
                    PAGE_NOTFOUND.as_bytes().to_vec()
                )
            })
        }
    ).expect("Could not initialize HTTPD");

    println!("Server initialized on {}:{}", IP_ADDRESS, PORT);

    loop {
        // Poll for packets and handle 'em
        server.poll();
    }
}

// Timer interrupt
#[exception]
fn SysTick() {
    system_clock::tick();
}

// The heap allocator
#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

// Out of memory handler
#[alloc_error_handler]
fn out_of_memory(_: Layout) -> ! {
    println!("Out of memory!");
    loop {}
}

// What to do on a panic
#[panic_handler]
fn panic(info: &PanicInfo) -> !{
    println!("Panic: {:?}", info);
    loop {}
}
