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

use rusb::ffi::{constants::*, *};
use rusb::{Device, GlobalContext, DeviceHandle, Error};
use simple_error::SimpleError;
use std::time::Duration;
use std::os::raw::{c_int, c_uint};
use std::ffi::c_void;

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
    let (manufacturer, product) = match device.open() {
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
                    }
                ),
                Err(_) => (String::new(),String::new())
            },
        Err(_) => (String::new(),String::new())
    };

    let id = match device.device_descriptor() {
        Ok(device_desc) =>
            format!("{:04x}:{:04x}",
                device_desc.vendor_id(), device_desc.product_id()),
        Err(_) => String::new()
    };

    format!("Bus: {:03} Device: {:03} ID: '{}' Manufacturer: '{}' Product: '{}'",
            device.bus_number(),
            device.address(),
            id,
            manufacturer,
            product)
}

pub trait IsIQDevice {
    fn is_iq_device(&self) -> bool;
}

impl IsIQDevice for Device<GlobalContext> {
    /** Returns true of the given USB device is an AR2300 IQ board */
    fn is_iq_device(&self) -> bool {
        match self.device_descriptor() {
            Ok(desc) =>
                desc.vendor_id() == IQ_VENDOR_ID &&
                    desc.product_id() == IQ_PRODUCT_ID,
            Err(_) => false
        }
    }
}


/** Find the AR2300 IQ device. */
pub fn find_iq_device() -> Option<Device<GlobalContext>> {
    match rusb::devices() {
        Ok(devices) =>
            devices.iter().find(|d| d.is_iq_device()),
        Err(_) => None
    }
}

// Check for a kernel driver and detach it if necessary
pub fn check_for_kernel_driver(handle: &mut DeviceHandle<GlobalContext>)
    -> Result<(),SimpleError> {
    match handle.set_auto_detach_kernel_driver(true) {
        Ok(_) => Ok(()),
        Err(e) => match e {
            // Kernel drivers are not supported on this platform
            rusb::Error::NotSupported => Ok(()),
            // All other errors should return an error
            _ => Err(SimpleError::new(format!("Couldn't check kernel driver status: {}", e)))
        }
    }
}

// Claim an interface
pub fn claim_interface(handle: &mut DeviceHandle<GlobalContext>, interface: u8)
    -> Result<(),SimpleError> {
    check_for_kernel_driver(handle)?;
    match handle.claim_interface(interface) {
        Ok(_) => {
            Ok(())
        },
        Err(e) => Err(SimpleError::new(format!("Couldn't claim interface: {}", e)))
    }
}

///// Isochronous Transfer Implementation /////

pub trait TransferCallback {
    fn callback(&self, r: rusb::Result<()>) -> bool;
    fn buffer(&mut self) -> &mut [u8];
}

pub trait IsochronousTransfer {
    /** Submits an Isochronous transfer. */
    fn submit_iso<T: TransferCallback> (
        &self,
        endpoint: u8,
        num_packets: usize,
        packet_len: usize,
        callback: &mut T,
        timeout: Duration,
    ) -> rusb::Result<()>;
}

impl IsochronousTransfer for DeviceHandle<GlobalContext> {

    /** Submits an Isochronous transfer. */
    fn submit_iso<T: TransferCallback> (
        &self,
        endpoint: u8,
        num_packets: usize,
        packet_len: usize,
        callback: &mut T,
        timeout: Duration,
    ) -> rusb::Result<()> {
        if endpoint & LIBUSB_ENDPOINT_DIR_MASK != LIBUSB_ENDPOINT_IN {
            return Err(Error::InvalidParam);
        }

        let buffer_len = ( packet_len * num_packets ) + packet_len;
        let buffer = callback.buffer();
        if buffer.len() < buffer_len {
            return Err(Error::InvalidParam);
        }

        unsafe {
            let transfer = libusb_alloc_transfer(num_packets as c_int);

            libusb_fill_iso_transfer(
                transfer,
                self.as_raw(),
                endpoint,
                buffer.as_mut_ptr(),
                buffer.len() as c_int,
                num_packets as c_int,
                callback_wrapper::<T>,
                callback as *mut _ as *mut c_void,
                timeout.as_millis() as c_uint
            );

            libusb_set_iso_packet_lengths(transfer, packet_len as c_uint);

            match libusb_submit_transfer(transfer) {
                0 => Ok(()),
                err => Err(from_libusb(err))
            }
        }
    }
}

extern "system" fn callback_wrapper<T: TransferCallback>(transfer: *mut libusb_transfer) {
    let callback = unsafe {
        &mut *((*transfer).user_data as *mut T)
    };

    let status = unsafe {
        (*transfer).status
    };

    let cont = match status {
        LIBUSB_TRANSFER_COMPLETED => callback.callback(Ok(())),
        LIBUSB_TRANSFER_ERROR => callback.callback(Err(Error::Other)),
        LIBUSB_TRANSFER_TIMED_OUT => callback.callback(Err(Error::Timeout)),
        LIBUSB_TRANSFER_CANCELLED => callback.callback(Err(Error::Interrupted)),
        LIBUSB_TRANSFER_STALL => callback.callback(Err(Error::Io)),
        LIBUSB_TRANSFER_NO_DEVICE => callback.callback(Err(Error::NoDevice)),
        LIBUSB_TRANSFER_OVERFLOW => callback.callback(Err(Error::Overflow)),
        err => callback.callback(Err(from_libusb(err))),
    };

    if cont {
        let s = unsafe {
            libusb_submit_transfer(transfer)
        };
        match s {
            0 => {},
            err => {
                callback.callback(Err(from_libusb(err)));
            }
        }
    }
}

/** This is copied from error.rs in rusb */
fn from_libusb(err: i32) -> Error {
    match err {
        LIBUSB_ERROR_IO => Error::Io,
        LIBUSB_ERROR_INVALID_PARAM => Error::InvalidParam,
        LIBUSB_ERROR_ACCESS => Error::Access,
        LIBUSB_ERROR_NO_DEVICE => Error::NoDevice,
        LIBUSB_ERROR_NOT_FOUND => Error::NotFound,
        LIBUSB_ERROR_BUSY => Error::Busy,
        LIBUSB_ERROR_TIMEOUT => Error::Timeout,
        LIBUSB_ERROR_OVERFLOW => Error::Overflow,
        LIBUSB_ERROR_PIPE => Error::Pipe,
        LIBUSB_ERROR_INTERRUPTED => Error::Interrupted,
        LIBUSB_ERROR_NO_MEM => Error::NoMem,
        LIBUSB_ERROR_NOT_SUPPORTED => Error::NotSupported,
        LIBUSB_ERROR_OTHER | _ => Error::Other,
    }
}