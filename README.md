# stm32f7-httpd

`A simple HTTP server for the STM32F7-Discovery board`

## Requirements

- cargo
- rustup
- openocd
- gdb-multiarch

## Prerequisites

`$ rustup target add thumbv7em-none-eabihf`

## Building

Open a connection to the STM32F7-Discovery board using `sudo openocd -f board/stm32f7discovery.cfg`.

Then, run `cargo run --release` to build and flash the program onto the board.
