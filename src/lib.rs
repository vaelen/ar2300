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
use std::error::Error;

pub mod usb;
pub mod firmware;
pub mod iq;

pub fn iq_device() -> Option<Device<GlobalContext>> {
    usb::find_iq_device()
}
pub fn program(device: &Device<GlobalContext>) -> Result<usize, Box<dyn Error>> {
    firmware::program(device)
}