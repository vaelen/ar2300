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

use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use std::sync::{Arc, Barrier};
use simple_error::SimpleError;

fn check_device(load_firmware: bool) -> Result<(), Box<dyn Error>> {
    match ar2300::iq_device() {
        Some(iq_device) => {
            let device_info = ar2300::usb::device_info(&iq_device);
            if load_firmware && !device_info.contains("AOR, LTD") {
                println!("Writing firmware");
                let bytes_written = ar2300::program(&iq_device)?;
                println!("Bytes written: {}", bytes_written);
                sleep(Duration::from_secs(1));
                check_device(false)?;
            } else {
                println!("IQ Device: {}", device_info);
            }
            Ok(())
        },
        None => Err(Box::new(SimpleError::new("IQ Device Not Found")))
    }
}

fn receive() -> Result<Arc<Barrier>, Box<dyn Error>> {
    if let Some(iq_device) = ar2300::iq_device() {
        let barrier = Arc::new(Barrier::new(2));
        let mut receiver = ar2300::iq::Receiver::new(iq_device)?;
        let done = barrier.clone();
        ctrlc::set_handler(move || {
            receiver.stop();
            done.wait();
        })?;
        Ok(barrier)
    } else {
        Err(Box::new(SimpleError::new("IQ Device Not Found")))
    }
}

fn main() -> Result<(),Box<dyn Error>> {
    //ar2300::usb::list_devices();
    check_device(true)?;
    let done = receive()?;
    done.wait();
    Ok(())
}