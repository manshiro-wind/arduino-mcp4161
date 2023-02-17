#![no_std]
#![no_main]

use panic_halt as _;

use arduino_hal::spi;

use arduino_mcp4161::{MCP4161};

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    /*
     * For examples (and inspiration), head to
     *
     *     https://github.com/Rahix/avr-hal/tree/main/examples
     *
     * NOTE: Not all examples were ported to all boards!  There is a good chance though, that code
     * for a different board can be adapted for yours.  The Arduino Uno currently has the most
     * examples available.
     */

    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());

    let adc_channel = pins.a0.into_analog_input(&mut adc).into_channel();

    // Create SPI interface.
    let (mut spi, _) = arduino_hal::Spi::new(
        dp.SPI,
        pins.d52.into_output(),
        pins.d51.into_output(),
        pins.d50.into_pull_up_input(),
        pins.d53.into_output(),
        spi::Settings::default(),
    );

    let chip_select_pin = pins.d48.into_output().downgrade();

    let mut mcp4161 = MCP4161::new(chip_select_pin, 5000, 512);

    loop {
        for i in 0..100 {
            let resistance: u16 = 50 * i as u16;

            mcp4161.set_resistance(&mut spi, resistance);
            arduino_hal::delay_ms(10);

            let v = adc.read_blocking(&adc_channel);

            ufmt::uwrite!(&mut serial, "Step {}, Voltage: {}\n", i, v).unwrap();

            arduino_hal::delay_ms(10);
        }
    }
}
