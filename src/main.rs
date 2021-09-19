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

use std::{error::Error, fs::File, thread::spawn};
use ar2300::{init_device, new_queue, receive, write};

fn main() -> Result<(),Box<dyn Error>> {
    let filename = "iq.bin";
    //ar2300::usb::list_devices();
    init_device(true)?;
    let f = Box::new(File::create(filename)?);
    let q = new_queue();
    let read_q = q.clone();
    let write_q = q.clone();

    let r = spawn(move || {
        if let Err(e) = receive(read_q) {
            eprint!("Error reading from radio: {}", e);
        }
    });
        
    let w = spawn(|| {
        if let Err(e) = write(write_q, f) {
            eprint!("Error writing to file: {}", e);
        }
    });

    r.join().unwrap();
    w.join().unwrap();

    Ok(())
}