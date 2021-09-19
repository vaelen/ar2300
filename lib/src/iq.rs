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

use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use rusb::{GlobalContext, DeviceHandle, Device};
use std::error::Error;
use std::io::Write;
use std::time::Duration;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use simple_error::{bail};
use crate::queue::Queue;
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
const PACKET_COUNT: usize = 2;

const BUFFER_LEN: usize = ( PACKET_LENGTH * PACKET_COUNT ) + PACKET_LENGTH;

pub struct Receiver {
    running: Arc<AtomicBool>,
    handle: Arc<DeviceHandle<GlobalContext>>,
    buf: Box<Vec<u8>>,
    skip_packet: Arc<AtomicBool>,
    queue: Queue<(f32,f32)>,
}

fn valid_packet(buffer: &[u8]) -> bool {
    (buffer[1] & 0x01) == 0x01
}

fn find_packet(buffer: &[u8]) -> Result<&[u8], Box<dyn Error>> {
    let mut buf = buffer;
    while buf.len() > 8 && !valid_packet(buf) {
        buf = &buf[1..];
    }
    if valid_packet(buf) {
        Ok(buf)
    } else {
        bail!("Packet not found")
    }
}

const BASE: f32 = 2f32 * 2147483648.0f32;

fn read_packet(packet: &[u8]) -> (f32, f32) {
    let i = LittleEndian::read_u32(&packet[0..4]);
    let q = LittleEndian::read_u32(&packet[4..8]);

    let f = |n: u32| -> f32 {
        let mut n16 = [
            (n >> 16) as u16,
            n as u16,
        ];
        // received data processing.
        if (n16[0] & 0x8000) == 0x8000 {
            n16[1] = n16[1] | 0x0001;
        } else {
            n16[1] = n16[1] & 0xfffe;
        }
        n16[0] = n16[0] << 1;
        ((((n16[0] as u32) << 16) | (n16[1] as u32)) as f32) / BASE
    };

    (f(i), f(q))
}

impl TransferCallback for Receiver {
    fn buffer(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }

    fn callback(&self, result: rusb::Result<()>) -> bool {
        let success = match result {
            Ok(_) => true,
            Err(rusb::Error::Other) => true,
            Err(e) => {
                eprintln!("Error reading IQ data: {}", e);
                self.running.swap(false, Ordering::Relaxed);
                false
            }
        };
        if success && !self.skip_packet.swap(false, Ordering::Relaxed) {
            let buffer = *self.buf.clone();
            match find_packet(buffer.as_slice()) {
                Ok(buf) => {
                    for packet in buf.chunks(8) {
                        if packet.len() == 8 && valid_packet(packet) {
                            self.queue.enqueue(read_packet(packet));
                        }
                        // TODO: Handle buffering the last partial packet
                    }
                },
                Err(_) => eprintln!("Couldn't find packet"),
            }

        }
        self.running.load(Ordering::Relaxed)
    }
}

impl Receiver {
    pub fn new(device: Device<GlobalContext>, queue: Queue<(f32,f32)>) -> Result<Receiver, Box<dyn Error>> {
        let mut handle = device.open()?;
        claim_interface(&mut handle, IQ_INTERFACE)?;
        Ok(Receiver {
            running: Arc::new(AtomicBool::new(false)),
            handle: Arc::new(handle),
            buf: Box::new(vec![0; BUFFER_LEN]),
            skip_packet: Arc::new(AtomicBool::new(true)),
            queue: queue,
        })
    }

    pub fn is_running(&self) -> Box<dyn Fn()->bool> {
        let r = self.running.clone();
        Box::new(move || r.load(Ordering::Relaxed))
    }

    pub fn queue(&self) -> Queue<(f32,f32)> {
        self.queue.clone()
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let running = self.running.clone();
        if let Ok(_) = running.compare_exchange(false,
                                          true,
                                          Ordering::Acquire,
                                          Ordering::Relaxed) {
            // Start IQ capture
            println!("IQ receiver starting");
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
                        Ok(_) => {
                            println!("Transfer request submitted");
                            Ok(())
                        }
                        Err(e) => {
                            bail!("Error submitting transfer request: {}", e);
                        }
                    }
                },
                Err(e) => {
                    bail!("Error starting IQ receiver: {}", e);
                }
            }
        } else {
            bail!("IQ receiver is already running")
        }
    }

    pub fn stop(&mut self) {
        let running = self.running.clone();
        if let Ok(_) = running.compare_exchange(true,
                                                false,
                                                Ordering::Acquire,
                                                Ordering::Relaxed) {
            print!("Stopping IQ receiver");
           
            self.queue.close();

            // End IQ capture
            match self.handle.write_bulk(CONTROL_ENDPOINT,
                                    &END_CAPTURE,
                                    Duration::from_secs(1)) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error stopping IQ capture: {}", e);
                }
            }
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct Writer {
    queue: Queue<(f32,f32)>,
    out: Box<dyn Write>,
}

impl Writer {
    pub fn new(queue: Queue<(f32,f32)>, out: Box<dyn Write>) -> Writer {
        Writer {
            queue: queue,
            out: out,
        }
    }

    pub fn queue(&self) -> Queue<(f32,f32)> {
        self.queue.clone()
    }

    pub fn write(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>> {
        if let Some((i,q)) = self.queue.dequeue(timeout) {
            self.out.write_f32::<BigEndian>(i)?;
            self.out.write_f32::<BigEndian>(q)?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        while !self.queue.is_empty() {
            self.write(Duration::from_millis(50))?;
        }
        Ok(())
    }
}

pub fn new_queue() -> Queue<(f32,f32)> {
    Queue::new(BUFFER_LEN/8)
}