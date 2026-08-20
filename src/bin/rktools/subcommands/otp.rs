use std::time::Duration;

use rkusb::RkDevice;

use crate::{
    common,
    util::{hexdump, parse_u32},
};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long, help = "Bus number of the device")]
    bus: Option<u8>,
    #[arg(long, help = "Address of the device")]
    addr: Option<u8>,
    #[arg(long, value_parser = humantime::parse_duration, help = "Wait for device with timeout (e.g., 30s, 1m)")]
    wait: Option<Duration>,
    #[arg(
        help = "Number of OTP bytes to read (decimal or 0x-prefixed hex)",
        default_value = "64",
        value_parser = parse_u32
    )]
    length: u32,
}

pub fn exec(usb_ctx: rusb::Context, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    if args.length == 0 {
        return Err("OTP length must be greater than 0".into());
    }
    let selected_device = common::find_device(&usb_ctx, args.bus, args.addr, args.wait)?;
    let mut rkdev = RkDevice::open(&selected_device)?;
    let mut buf = vec![0u8; args.length as usize];
    rkdev.read_otp(&mut buf)?;
    println!("OTP ({} bytes):", buf.len());
    hexdump(&buf);
    Ok(())
}
