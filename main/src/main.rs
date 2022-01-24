#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use eh1_0_alpha::i2c::Error;
use embedded_hal::blocking::i2c::Read;
use embedded_hal::timer::CountDown;
use embedded_time::duration::Extensions as _;
use embedded_time::rate::Extensions as _;
use panic_probe as _;

//use rp_pico as bsp;
use solderparty_rp2040_stamp as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = bsp::XOSC_CRYSTAL_FREQ;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let timer = bsp::hal::timer::Timer::new(pac.TIMER, &mut pac.RESETS);

    let mut sda = pins.gpio4.into_mode();
    let mut scl = pins.gpio5.into_mode();
    scl.set_drive_strength(bsp::hal::gpio::OutputDriveStrength::TwoMilliAmps);
    sda.set_drive_strength(bsp::hal::gpio::OutputDriveStrength::TwoMilliAmps);

    let mut ctrl = bsp::hal::i2c::I2C::new_controller(
        pac.I2C0,
        sda,
        scl,
        50_000.Hz(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );

    info!("ready");
    let mut buffer = [0; 16];
    loop {
        let res = ctrl.read(0x55, &mut buffer[..8]);
        match res {
            Err(err @ bsp::hal::i2c::Error::Abort(_)) => {
                info!("read: Err(Abort({}))", defmt::Debug2Format(&err.kind()))
            }
            _ => info!("read: {}", defmt::Debug2Format(&res)),
        }

        let mut cd = timer.count_down();
        cd.start(250_000.microseconds());
        let _ = nb::block!(cd.wait());
    }
}
