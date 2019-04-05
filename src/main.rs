//! A simple HTTP server for the STM32F7-Discovery board

#![warn(clippy::all)]
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(alloc)]

#[macro_use]
extern crate alloc;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout as AllocLayout;
use core::panic::PanicInfo;
use cortex_m_rt::{entry, exception};
use smoltcp::wire::{EthernetAddress, Ipv4Address};
use stm32f7::stm32f7x6::{CorePeripherals, Peripherals};
use stm32f7_discovery::gpio::{GpioPort, OutputPin};
use stm32f7_discovery::init;
use stm32f7_discovery::lcd;
use stm32f7_discovery::print;
use stm32f7_discovery::println;
use stm32f7_discovery::system_clock::{self, Hz};

mod httpd;

const SYSTICK: Hz = Hz(20);

const HEAP_SIZE: usize = 50 * 1024;

const ETH_ADDR: EthernetAddress = EthernetAddress([0x00, 0x08, 0xdc, 0xab, 0xcd, 0xef]);
const IP_ADDR: Ipv4Address = Ipv4Address([192, 168, 0, 100]);
const PORT: u16 = 8000;

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
    let mut ethernet_dma = peripherals.ETHERNET_DMA;

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

    // set up layers
    let mut layer_1 = lcd.layer_1().unwrap();
    let mut layer_2 = lcd.layer_2().unwrap();
    layer_1.clear();
    layer_2.clear();

    // set stdout to layer 2
    lcd::init_stdout(layer_2);

    unsafe { ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE) }

    let mut server = httpd::init(
        &mut rcc,
        &mut syscfg,
        &mut ethernet_mac,
        &mut ethernet_dma,
        ETH_ADDR,
        IP_ADDR,
        PORT,
    ).unwrap();

    // set up routes
/*  server.routes(|request, mut response| {
        info!("{} {}", request.method(), request.uri());

        match (request.method(), request.uri().path) {
            (&Method::GET, "/") => {
                Ok(reponse.body("<p>Index</p>".as_bytes().to_vec())?)
            },
            (_, _) => {
                response.status(StatusCode::NOT_FOUND);
            }
        }
    });*/

    loop {
        // poll packets and answer them
        server.poll();
    }
}

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[alloc_error_handler]
fn oom_handler(_: AllocLayout) -> ! {
    loop {}
}

#[exception]
fn SysTick() {
    system_clock::tick();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{:?}", _info);
    loop {}
}
