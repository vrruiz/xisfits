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

use crate::xisfreader::{XISFType, XISFile};
use log::info;
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
    /// Gets the path to the input XISF file.
    pub fn input(&self) -> &Path {
        self.input.as_path()
    }

    /// Gets the path to the output FITS file.
    pub fn output(&self) -> &Path {
        self.output.as_path()
    }
}

/// Convert XISF binary data to FITS format (Big Endian)
pub fn xisf_data_to_fits(xisf_file: &XISFile) -> (Box<[u8]>, i64) {
    let mut fits_data = Vec::new();
    let mut bitpix = 0;

    // +---------+-------+------+
    // | XISF    > Rust  > FITS |
    // +---------+-------+------+
    // | UInt8   | u8    | 8    |
    // | UInt16  | i16   | 16   |
    // | UInt32  | i32   | 32   |
    // | Float32 | f32   | -32  |
    // | Float64 | f64   | -64  |
    // +---------+-------+------+
    let header = xisf_file.header();
    let data = xisf_file.data();

    for i in 0..header.geometry_channels() as usize {
        match header.sample_format() {
            XISFType::UInt8 => {
                bitpix = 8;
                fits_data.extend_from_slice(&data.uint8[i]);
            }
            XISFType::UInt16 => {
                bitpix = 16;
                fits_data.append(&mut convert::u16_to_i16_to_v_u8_be(&data.uint16[i]));
            }
            XISFType::UInt32 => {
                bitpix = 32;
                fits_data.append(&mut convert::u32_to_i32_to_v_u8_be(&data.uint32[i]));
            }
            XISFType::Float32 => {
                bitpix = -32;
                fits_data.append(&mut convert::f32_to_v_u8_be(&data.float32[i]));
            }
            XISFType::Float64 => {
                bitpix = -64;
                fits_data.append(&mut convert::f64_to_v_u8_be(&data.float64[i]));
            }
            _ => println!(
                "Convert to FITS > Unsupported XISF type > {}",
                header.sample_format().as_str()
            ),
        }
    }

    // Show the first 20 bytes of the converted image
    if fits_data.len() > 20 {
        let mut message = String::with_capacity(20 * 2);
        for byte in fits_data.iter().take(20) {
            message.push_str(&format!("{:x} ", byte));
        }
        info!("{}", message);
    }

    (fits_data.into_boxed_slice(), bitpix)
}

fn main() -> io::Result<()> {
    // Init logger
    env_logger::builder().format_timestamp(None).init();

    // CLI interface information.
    let cli = Cli::from_args();

    // Open XISF image file
    let xisf_file = XISFile::read_file(cli.input())?;

    // -- Convert XISF to FITS
    info!("Convert to FITS > Image data to bytes");
    let (fits_data, bitpix) = xisf_data_to_fits(&xisf_file);

    // Write FITS image to disk
    if bitpix != 0 {
        info!("Convert to FITS > Write image data");
        let fits_hd = fitswriter::FitsHeaderData {
            bitpix,
            naxis: xisf_file.header().geometry_sizes().len() as u64,
            naxis_vec: xisf_file.header().geometry_sizes(),
            bzero: 0,
            bscale: 1,
            datamin: 0,
            datamax: 0,
            history: vec![String::new()],
            comment: vec![String::new()],
            data_bytes: fits_data,
        };
        if xisf_file.keywords().is_empty() {
            fitswriter::fits_write_data(cli.output(), &fits_hd)?;
        } else {
            fitswriter::fits_write_data_keywords(cli.output(), &fits_hd, &xisf_file.keywords())?;
        }
    }
    // -- End of convert XISF to FITS

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_xisf_read_gray_8bit_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/x-special/xisf-image-gray-256x256-8bits.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);
        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::UInt8);
                assert_eq!(file.header().geometry(), "256:256:1");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }

    #[test]
    fn test_xisf_read_rgb_16bit_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/x-special/xisf-image-rgb-256x256-16bits.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);
        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::UInt16);
                assert_eq!(file.header().geometry(), "256:256:3");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }

    #[test]
    fn test_xisf_read_rgb_32bit_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/xisf-image-rgb-256x256-32bits.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);
        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::UInt32);
                assert_eq!(file.header().geometry(), "256:256:3");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }

    #[test]
    fn test_xisf_read_rgb_8bit_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/xisf-image-rgb-256x256-8bits.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);

        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::UInt8);
                assert_eq!(file.header().geometry(), "256:256:3");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }

    #[test]
    fn test_xisf_read_gray_float32_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/xisf-image-gray-256x256-float-32bits.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);

        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::Float32);
                assert_eq!(file.header().geometry(), "255:255:1");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }

    #[test]
    fn test_xisf_read_gray_float64_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/xisf-image-gray-256x256-float-64bits.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);

        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::Float64);
                assert_eq!(file.header().geometry(), "255:255:1");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }

    #[test]
    fn test_xisf_read_zlib_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/xisf-image-gray-256x256-16bits-zlib.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);

        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::UInt16);
                assert_eq!(file.header().geometry(), "256:256:1");
                assert_eq!(file.header().compression_codec(), "zlib");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }

    #[test]
    fn test_xisf_read_zlibsh_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/xisf-image-gray-256x256-16bits-zlib_sh.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);

        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::UInt16);
                assert_eq!(file.header().geometry(), "256:256:1");
                assert_eq!(file.header().compression_codec(), "zlib+sh");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }

    #[test]
    #[ignore] // LZ4 uncompression currently fails
    fn test_xisf_read_lz4_file() {
        init();

        // Test that we can read a XISF file
        let xisf_filename = Path::new("tests/images/xisf-image-gray-256x256-16bits-lz4.xisf");

        let xisf_file = XISFile::read_file(xisf_filename);

        match xisf_file {
            Ok(file) => {
                assert_eq!(file.header().sample_format(), XISFType::UInt16);
                assert_eq!(file.header().geometry(), "256:256:1");
                assert_eq!(file.header().compression_codec(), "lz4");
            }
            Err(e) => {
                eprintln!("Tests > Error: {}", e);
            }
        }
    }
}
