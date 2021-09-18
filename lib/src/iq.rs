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

use rusb::{GlobalContext, DeviceHandle, Device};
use std::error::Error;
use std::time::Duration;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use simple_error::{bail};
use crate::usb::TransferCallback;
use crate::usb::IsochronousTransfer;
use crate::usb::claim_interface;

const IQ_INTERFACE: u8 = 0;
const CONTROL_ENDPOINT: u8 = 0x02;
const DATA_ENDPOINT: u8 = 0x86;
const START_CAPTURE: [u8; 6] = [0x5a, 0xa5, 0x00, 0x02, 0x41, 0x53];
const END_CAPTURE: [u8; 6] =  [0x5a, 0xa5, 0x00, 0x02, 0x41, 0x45];
const PACKET_ATOM: usize = 512;
const PACKET_LENGTH: usize = PACKET_ATOM*3;
const PACKET_COUNT: usize = 8192;

pub struct Receiver {
    running: Arc<AtomicBool>,
    handle: Arc<DeviceHandle<GlobalContext>>,
    buf: Option<Box<Vec<u8>>>
}

impl TransferCallback for Receiver {
    fn callback(&self, result: rusb::Result<&[u8]>) -> bool {
        match result {
            Ok(buffer) => {
                println!("Read {} bytes", buffer.len());
            },
            Err(e) => {
                eprintln!("Error reading IQ data: {}", e);
                self.running.swap(false, Ordering::Relaxed);
            }
        }
        self.running.load(Ordering::Relaxed)
    }
}

impl Receiver {
    pub fn new(device: Device<GlobalContext>) -> Result<Receiver, Box<dyn Error>> {
        let mut handle = device.open()?;
        claim_interface(&mut handle, IQ_INTERFACE)?;
        Ok(Receiver {
            running: Arc::new(AtomicBool::new(false)),
            handle: Arc::new(handle),
            buf: None,
        })
    }

    pub fn is_running(&self) -> Box<dyn Fn()->bool> {
        let r = self.running.clone();
        Box::new(move || r.load(Ordering::Relaxed))
    }

    /** Start data reception */
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let running = self.running.clone();
        if let Ok(_) = running.compare_exchange(false,
                                          true,
                                          Ordering::Acquire,
                                          Ordering::Relaxed) {
            // Start IQ capture
            println!("IQ capture starting");
            match self.handle.write_bulk(CONTROL_ENDPOINT,
                                         &START_CAPTURE,
                                         Duration::from_secs(1)) {
                Ok(_) => {
                    let handle = self.handle.clone();

                    println!("Submitting transfer request");
                    match handle.submit_iso(
                        DATA_ENDPOINT,
                        PACKET_COUNT,
                        PACKET_LENGTH,
                        self,
                        Duration::from_millis(0)) {
                        Ok(vec) => {
                            self.buf = Some(vec);
                            println!("Transfer request submitted");
                            Ok(())
                        }
                        Err(e) => {
                            bail!("Error submitting transfer request: {}", e);
                        }
                    }
                },
                Err(e) => {
                    bail!("Error starting IQ capture: {}", e);
                }
            }
        } else {
            bail!("Capture is already running")
        }
    }

    pub fn stop(&mut self) {
        let running = self.running.clone();
        if let Ok(_) = running.compare_exchange(true,
                                                false,
                                                Ordering::Acquire,
                                                Ordering::Relaxed) {
            print!("Stopping IQ capture");

            // End IQ capture
            match self.handle.write_bulk(CONTROL_ENDPOINT,
                                    &END_CAPTURE,
                                    Duration::from_secs(1)) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error stopping IQ capture: {}", e);
                }
            }
            println!("IQ capture stopped");
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop();
    }
}