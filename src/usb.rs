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

use rusb::{Device, GlobalContext};

const IQ_VENDOR_ID: u16 = 0x08d0;
const IQ_PRODUCT_ID: u16 = 0xa001;

/** List all USB devices. */
pub fn list_devices() {
    match rusb::devices() {
        Ok(devices) => {
            println!("USB Devices:");
            for device in devices.iter() {
                println!("  {}", device_info(&device));
            }
            println!();
        },
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

pub fn device_info(device: &Device<GlobalContext>) -> String {
    let (manufacturer, product, serial) = match device.open() {
        Ok(handle) =>
            match device.device_descriptor() {
                Ok(device_desc) => (
                    match handle.read_manufacturer_string_ascii(&device_desc) {
                        Ok(s) => s,
                        Err(_) => String::new()
                    },
                    match handle.read_product_string_ascii(&device_desc) {
                        Ok(s) => s,
                        Err(_) => String::new()
                    },
                    match handle.read_serial_number_string_ascii(&device_desc) {
                        Ok(s) => s,
                        Err(_) => String::new()
                    }
                ),
                Err(_) => (String::new(),String::new(),String::new())
            },
        Err(_) => (String::new(),String::new(),String::new())
    };

    let id = match device.device_descriptor() {
        Ok(device_desc) =>
            format!("{:04x}:{:04x}",
                device_desc.vendor_id(), device_desc.product_id()),
        Err(_) => String::new()
    };

    format!("Bus: {:03} Device: {:03} ID: '{}' Manufacturer: '{}' Product: '{}' Serial: '{}'",
            device.bus_number(),
            device.address(),
            id,
            manufacturer,
            product,
            serial)
}

/** Returns true of the given USB device is an AR2300 IQ board */
fn is_iq_device(device: &Device<GlobalContext>) -> bool {
    match device.device_descriptor() {
        Ok(desc) =>
            desc.vendor_id() == IQ_VENDOR_ID &&
                desc.product_id() == IQ_PRODUCT_ID,
        Err(_) => false
    }
}

/** Find the AR2300 IQ device. */
pub fn find_iq_device() -> Option<Device<GlobalContext>> {
    match rusb::devices() {
        Ok(devices) =>
            devices.iter().find(|d| is_iq_device(d)),
        Err(_) => None
    }
}