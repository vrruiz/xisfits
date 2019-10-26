//! XISFITS is a command line tool to convert XISF images to FITS.

#![forbid(anonymous_parameters)]
#![warn(clippy::pedantic)]
#![deny(
    clippy::all,
    variant_size_differences,
    unused_results,
    unused_qualifications,
    unused_import_braces,
    unsafe_code,
    trivial_numeric_casts,
    trivial_casts,
    missing_docs,
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations
)]

mod convert;
mod fitswriter;
mod xisfreader;

use lazy_static::lazy_static;
use std::{
    io,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about)]
struct Cli {
    // Wether to include extra information while doing the conversion in
    #[structopt(short, long)]
    verbose: bool,
    /// Path to the XISF input file.
    #[structopt(name = "input-file", parse(from_os_str))]
    input: PathBuf,
    /// Path to the FITS output file.
    #[structopt(name = "output-file", parse(from_os_str))]
    output: PathBuf,
}

impl Cli {
    /// Gets wether the program was executed in verbose mode or not.
    pub fn verbose(&self) -> bool {
        self.verbose
    }

    /// Gets the path to the input XISF file.
    pub fn input(&self) -> &Path {
        self.input.as_path()
    }

    /// Gets the path to the output FITS file.
    pub fn output(&self) -> &Path {
        self.output.as_path()
    }
}

lazy_static! {
    /// CLI interface information.
    static ref CLI: Cli = Cli::from_args();
}

/// Convert XISF binary data to FITS format (Big Endian)
pub fn xisf_data_to_fits(
    xisf_header: &xisfreader::XISFHeader,
    xisf_data: &mut xisfreader::XISFData,
    fits_data: &mut Vec<u8>,
    bitpix: &mut i64,
) {
    // +---------+-------+------+
    // | XISF    > Rust  > FITS |
    // +---------+-------+------+
    // | UInt8   | u8    | 8    |
    // | UInt16  | i16   | 16   |
    // | UInt32  | i32   | 32   |
    // | Float32 | f32   | -32  |
    // | Float64 | f64   | -64  |
    // +---------+-------+------+
    for i in 0..xisf_header.geometry_channels as usize {
        match xisf_header.sample_format.as_str() {
            "UInt8" => {
                *bitpix = 8;
                fits_data.append(&mut xisf_data.uint8[i]);
            }
            "UInt16" => {
                *bitpix = 16;
                fits_data.append(&mut convert::u16_to_i16_to_v_u8_be(&xisf_data.uint16[i]));
            }
            "UInt32" => {
                *bitpix = 32;
                fits_data.append(&mut convert::u32_to_i32_to_v_u8_be(&xisf_data.uint32[i]));
            }
            "Float32" => {
                *bitpix = -32;
                fits_data.append(&mut convert::f32_to_v_u8_be(&xisf_data.float32[i]));
            }
            "Float64" => {
                *bitpix = -64;
                fits_data.append(&mut convert::f64_to_v_u8_be(&xisf_data.float64[i]));
            }
            _ => println!(
                "Convert to FITS > Unsupported XISF type > {}",
                xisf_header.sample_format.as_str()
            ),
        }
    }

    if CLI.verbose() {
        // Show the first 20 bytes of the converted image
        if fits_data.len() > 20 {
            for byte in fits_data.iter().take(20) {
                print!("{:x} ", byte);
            }
            println!();
        }
    }
}

fn main() -> io::Result<()> {
    // Define variables
    let mut xisf_header = xisfreader::XISFHeader::default();
    let mut xisf_data = xisfreader::XISFData::default();
    let mut xisf_fits_keywords = Vec::new();

    // Open XISF image file
    xisfreader::xisf_read_file(
        CLI.input(),
        &mut xisf_header,
        &mut xisf_data,
        &mut xisf_fits_keywords,
    )?;

    // -- Convert XISF to FITS
    if CLI.verbose() {
        println!("Convert to FITS > Image data to bytes");
    }
    let mut fits_data = vec![];
    let mut bitpix = 0_i64;
    xisf_data_to_fits(&xisf_header, &mut xisf_data, &mut fits_data, &mut bitpix);

    // Write FITS image to disk
    if bitpix != 0 {
        if CLI.verbose() {
            println!("Convert to FITS > Write image data");
        }
        let fits_hd = fitswriter::FitsHeaderData {
            bitpix,
            naxis: xisf_header.geometry_sizes.len() as u64,
            naxis_vec: xisf_header.geometry_sizes,
            bzero: 0,
            bscale: 1,
            datamin: 0,
            datamax: 0,
            history: vec![String::new()],
            comment: vec![String::new()],
            data_bytes: fits_data,
        };
        if xisf_fits_keywords.is_empty() {
            fitswriter::fits_write_data(CLI.output(), &fits_hd)?;
        } else {
            fitswriter::fits_write_data_keywords(CLI.output(), &fits_hd, &xisf_fits_keywords)?;
        }
    }
    // -- End of convert XISF to FITS

    Ok(())
}
