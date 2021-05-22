/*
    Copyright 2021, Andrew C. Young <andrew@vaelen.org>

    This file is part of the AR2300 library.

    The AR2300 library is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Foobar is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with the AR2300 library.  If not, see <https://www.gnu.org/licenses/>.
 */

use rusb::{Device, GlobalContext, DeviceHandle, LogLevel};
use std::error::Error;
use std::time::Duration;
use std::str;

const FIRMWARE_HEX: &str = include_str!("fx2fw.hex");
const RESET_ADDRESS: u16 = 0xe600;
const RESET_COMMAND: [u8;1] = [1];
const RUN_COMMAND: [u8;1] = [0];

/** Program the device */
pub fn program(device: &Device<GlobalContext>) -> Result<usize, Box<dyn Error>> {
    rusb::set_log_level(LogLevel::Info);
    let handle = device.open()?;
    reset(&handle)?;
    let bytes_written= write_firmware(&handle, FIRMWARE_HEX)?;
    run(&handle)?;
    Ok(bytes_written)
}

/** Reset the device */
pub fn reset(handle: &DeviceHandle<GlobalContext>) -> rusb::Result<usize> {
    write_ram(handle, RESET_ADDRESS, &RESET_COMMAND)
}

/** Start the device */
pub fn run(handle: &DeviceHandle<GlobalContext>) -> rusb::Result<usize> {
    write_ram(handle, RESET_ADDRESS, &RUN_COMMAND)
}

/** Write firmware to the given device */
pub fn write_firmware(handle: &DeviceHandle<GlobalContext>, firmware: &str) -> Result<usize, Box<dyn Error>> {
    let mut bytes_written: usize = 0;
    for line in firmware.lines() {
        // Parse Intel hex file format
        if !line.starts_with(&":") || line.len() < 11 {
            continue;
        }
        let num_bytes = usize::from_str_radix(&line[1..3], 16)?;
        let address = u16::from_str_radix(&line[3..7], 16)?;
        let typ = u8::from_str_radix(&line[7..9], 16)?;
        match typ {
            0 => {
                // Data
                let hex = &line[9..line.len()-2];
                let data= parse_hex(hex);
                if data.len() != num_bytes {
                    // Bad Data Length
                    eprintln!("Bad data length. Expected: {}, Received: {}", num_bytes, data.len());
                    continue;
                }
                bytes_written += write_ram(handle, address, &data)?;
            },
            1 => {
                // EOF
                break;
            } ,
            _ => {}
        }
    }
    Ok(bytes_written)
}

/** Parse a hex string into a byte vector */
fn parse_hex(data: &str) -> Vec<u8> {
    data
        .as_bytes()
        .chunks(2)
        .map(str::from_utf8)
        .map(|x|
            match x {
                Ok(s) => match u8::from_str_radix(s, 16) {
                    Ok(b) => b,
                    Err(_) => 0
                }
                Err(_) => 0
            })
        .collect::<Vec<u8>>()
}

/** Write data to RAM */
pub fn write_ram(handle: &DeviceHandle<GlobalContext>, address: u16, data: &[u8]) -> rusb::Result<usize> {
    let mut bytes_written = 0;
    bytes_written += handle.write_control(0x40, 0xa0, address, 0, data, Duration::from_secs(5))?;
    Ok(bytes_written)
}