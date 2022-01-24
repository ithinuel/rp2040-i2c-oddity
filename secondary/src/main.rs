#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::timer::CountDown;
use embedded_time::duration::Extensions;
use panic_probe as _;

use rp_pico as bsp;
//use solderparty_rp2040_stamp as bsp;

use bsp::hal::{
    clocks::init_clocks_and_plls, i2c::peripheral::I2CEvent, pac, sio::Sio, watchdog::Watchdog,
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
    let _clocks = init_clocks_and_plls(
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

    let mut periph = bsp::hal::i2c::I2C::new_peripheral_event_iterator(
        pac.I2C0,
        sda,
        scl,
        &mut pac.RESETS,
        0x55,
    );
    info!("ready");
    let mut timestamp = timer.get_counter_low();
    loop {
        let evt = periph.next();
        if let Some(evt) = &evt {
            info!("{}", defmt::Debug2Format(evt));
        }
        match evt {
            Some(I2CEvent::Start | I2CEvent::Restart) => {}
            Some(I2CEvent::TransferRead) => {
                let queued = periph.write(&[0; 16]);
                info!("{}: {} bytes queued", timer.get_counter_low(), queued);
            }
            Some(I2CEvent::TransferWrite) => {
                let mut b = 0;
                periph.read(core::slice::from_mut(&mut b));
            }
            Some(I2CEvent::Stop) => {
                timestamp = timer.get_counter_low();
            }
            None => {
                let now = timer.get_counter_low();
                if now.wrapping_sub(timestamp) > 1_000_000 {
                    let (i2c, (sda_pin, scl_pin)) = periph.free(&mut pac.RESETS);

                    info!("{}: Timeout resetting peripheral", now);
                    let mut cd = timer.count_down();
                    cd.start(1_000_000.microseconds());
                    let _ = nb::block!(cd.wait());

                    timestamp = timer.get_counter_low();
                    periph = bsp::hal::i2c::I2C::new_peripheral_event_iterator(
                        i2c,
                        sda_pin,
                        scl_pin,
                        &mut pac.RESETS,
                        0x55,
                    );
                }
            }
        }
    }
}
