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
    along with Foobar.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

fn check_device(load_firmware: bool) -> Result<(),Box<dyn Error>> {
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
        },
        None => println!("IQ Device Not Found")
    }
    Ok(())
}

fn main() -> Result<(),Box<dyn Error>> {
    //ar2300::usb::list_devices();
    check_device(true)?;
    Ok(())
}