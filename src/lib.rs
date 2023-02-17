#![no_std]

use arduino_hal::port::{mode, Pin};
use embedded_hal::spi::FullDuplex;

#[derive(Debug, Clone)]
struct InvalidCommandCombinationError;

#[derive(Clone, Copy)]
pub enum Command {
    Write,
    Increment,
    Decrement,
    Read,
    EnableWiperLock,
    DisableWiperLock,
}

#[allow(clippy::from_over_into)]
impl Into<u8> for Command {
    fn into(self) -> u8 {
        match self {
            Command::Write => 0b00 << 2,
            Command::Increment => 0b01 << 2,
            Command::Decrement => 0b10 << 2,
            Command::Read => 0b11 << 2,
            Command::EnableWiperLock => 0b01 << 2,
            Command::DisableWiperLock => 0x10 << 2,
        }
    }
}


#[derive(Clone, Copy)]
pub enum MemoryAddress {
    VolatileWiper0,
    VolatileWiper1,
    NonVolatileWiper0,
    NonVolatileWiper1,
    VolatileTCONRegister,
    StatusRegister,
    DataEEPROM(u8),
    WiperLock,
}

#[allow(clippy::from_over_into)]
impl Into<u8> for MemoryAddress {
    fn into(self) -> u8 {
        match self {
            MemoryAddress::VolatileWiper0 => 0x00 << 4,
            MemoryAddress::VolatileWiper1 => 0x01 << 4,
            MemoryAddress::NonVolatileWiper0 => 0x02 << 4,
            MemoryAddress::NonVolatileWiper1 => 0x03 << 4,
            MemoryAddress::VolatileTCONRegister => 0x04 << 4,
            MemoryAddress::StatusRegister => 0x05 << 4,
            MemoryAddress::DataEEPROM(eeprom_slot) => {
                if eeprom_slot >= 10 {
                    panic!();
                }
                (0x05 + eeprom_slot) << 4
            }
            MemoryAddress::WiperLock => 0x0F << 4,
        }
    }
}

fn compose_command(
    memory_address: &MemoryAddress,
    command: &Command,
    data_byte: Option<u16>,
) -> Result<(u8, Option<u8>), InvalidCommandCombinationError> {
    let mut command_byte = 0;

    command_byte |= Into::<u8>::into(*memory_address);
    command_byte |= Into::<u8>::into(*command);

    match command {
        Command::Write => {
            if data_byte.is_none() {
                return Err(InvalidCommandCombinationError);
            }
        }
        Command::Increment
        | Command::Decrement
        | Command::EnableWiperLock
        | Command::DisableWiperLock
        | Command::Read => {
            if data_byte.is_some() {
                return Err(InvalidCommandCombinationError);
            }
        }
    }

    if let Some(data) = data_byte {
        let top_bits = (data >> 8) as u8 & 0b0000_0011;
        command_byte |= top_bits;

        let bottom_byte = (data & 0b1111_1111) as u8;

        Ok((command_byte, Some(bottom_byte)))
    } else {
        Ok((command_byte, None))
    }
}

pub struct MCP4161 {
    chip_select_pin: Pin<mode::Output>,
    pub full_scale: u16,
    pub n_bits: u16,
}

impl MCP4161 {
    pub fn new(mut chip_select_pin: Pin<mode::Output>, full_scale: u16, n_bits: u16) -> Self {
        chip_select_pin.set_high();
        Self {
            chip_select_pin,
            full_scale,
            n_bits,
        }
    }

    pub fn set_resistance(&mut self, spi: &mut arduino_hal::Spi, resistance: u16) {
        let scale = ((resistance.min(self.full_scale) as u32 * self.n_bits as u32)
            / self.full_scale as u32) as u16;
        self.send_command(
            spi,
            MemoryAddress::VolatileWiper0,
            Command::Write,
            Some(scale),
        );
    }

    pub fn send_command(
        &mut self,
        spi: &mut arduino_hal::Spi,
        address: MemoryAddress,
        command: Command,
        optional_data: Option<u16>,
    ) -> (u8, Option<u8>) {
        let (command_byte, data_byte_option) =
            compose_command(&address, &command, optional_data).unwrap();

        self.chip_select_pin.set_low();
        arduino_hal::delay_ms(1);

        nb::block!(spi.send(command_byte)).unwrap();
        if let Some(data_byte) = data_byte_option {
            nb::block!(spi.send(data_byte)).unwrap();
        } else {
            nb::block!(spi.send(0b0000_0000)).unwrap();
        }

        arduino_hal::delay_ms(1);
        self.chip_select_pin.set_high();
        (command_byte, data_byte_option)
    }
}
