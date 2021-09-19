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

use iq::{Receiver, Writer};
use queue::Queue;
use rusb::{Device, GlobalContext, UsbContext};
use simple_error::bail;
use std::{error::Error, io::Write, thread::sleep, time::Duration};

pub mod usb;
pub mod firmware;
pub mod iq;
pub mod queue;

/** Return the AR2300 IQ device. */
pub fn iq_device() -> Option<Device<GlobalContext>> {
    usb::find_iq_device()
}

/** Program the AR2300 firmware. */
pub fn program(device: &Device<GlobalContext>) -> Result<usize, Box<dyn Error>> {
    firmware::program(device)
}

pub fn init_device(load_firmware: bool) -> Result<(), Box<dyn Error>> {
    match iq_device() {
        Some(iq_device) => {
            let device_info = crate::usb::device_info(&iq_device);
            if load_firmware && !device_info.contains("AOR, LTD") {
                println!("Writing firmware");
                let bytes_written = program(&iq_device)?;
                println!("Bytes written: {}", bytes_written);
                sleep(Duration::from_secs(1));
                init_device(false)?;
            } else {
                println!("IQ Device: {}", device_info);
            }
            Ok(())
        },
        None => bail!("IQ Device Not Found")
    }
}

pub fn new_queue() -> Queue<(f32,f32)> {
    iq::new_queue()
}

pub fn receive(queue: Queue<(f32,f32)>) -> Result<(), Box<dyn Error>> {
    if let Some(iq_device) = iq_device() {
        let mut receiver = Receiver::new(iq_device, queue)?;
        receiver.start()?;
        let is_running= receiver.is_running();
        ctrlc::set_handler(move || {
            receiver.stop();
        })?;
        println!("IQ receiver started");
        while is_running() {
            GlobalContext::default().handle_events(Some(Duration::from_millis(50)))?;
        }
        println!("IQ receiver stopped");
        Ok(())
    } else {
        bail!("IQ Device Not Found")
    }
}

pub fn write(queue: Queue<(f32,f32)>, out: Box<dyn Write>) -> Result<(), Box<dyn Error>> {
    let q = queue.clone();
    let mut writer = Writer::new(queue, out);
    println!("Writer started");
    while !q.is_closed() {
        writer.write(Duration::from_millis(100))?;
    }
    writer.flush()?;
    println!("Writer stopped");
    Ok(())
}