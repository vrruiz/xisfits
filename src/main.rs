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

use std::{env, io, process};

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

    // Show the first 20 bytes of the converted image
    if fits_data.len() > 20 {
        for byte in fits_data.iter().take(20) {
            print!("{:x} ", byte);
        }
        println!();
    }
}

fn main() -> io::Result<()> {
    // Define variables
    let mut xisf_header = xisfreader::XISFHeader {
        signature: String::from(""),
        length: 0,
        reserved: 0,
        header: String::from(""),
        geometry: String::from(""),
        geometry_channels: 0,
        geometry_sizes: vec![],
        geometry_channel_size: 0,
        sample_format: String::from(""),
        sample_format_bytes: 0,
        color_space: String::from(""),
        location: String::from(""),
        location_method: String::from(""),
        location_start: 0,
        location_length: 0,
        compression: String::from(""),
        compression_codec: String::from(""),
        compression_size: 0,
    };

    let mut xisf_data = xisfreader::XISFData {
        // format:  String::from(""),
        // int8:    vec![],
        uint8: vec![],
        // int16:   vec![],
        uint16: vec![],
        // int32:   vec![],
        uint32: vec![],
        // int64:   vec![],
        // uint64:  vec![],
        // int128:  vec![],
        // uint128: vec![],
        float32: vec![],
        float64: vec![],
    };

    let mut xisf_fits_keywords = Vec::new();

    // Read command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("xisfits [input file name].xifs [output file name].fits");
        process::exit(1);
    }
    let xisf_filename = &args[1];
    let fits_filename = &args[2];

    println!("Args: {:?}", args);

    // Open XISF image file
    xisfreader::xisf_read_file(
        xisf_filename,
        &mut xisf_header,
        &mut xisf_data,
        &mut xisf_fits_keywords,
    )?;

    // -- Convert XISF to FITS
    println!("Convert to FITS > Image data to bytes");
    let mut fits_data = vec![];
    let mut bitpix = 0_i64;
    xisf_data_to_fits(&xisf_header, &mut xisf_data, &mut fits_data, &mut bitpix);

    // Write FITS image to disk
    if bitpix != 0 {
        println!("Convert to FITS > Write image data");
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
            fitswriter::fits_write_data(fits_filename, &fits_hd)?;
        } else {
            fitswriter::fits_write_data_keywords(fits_filename, &fits_hd, &xisf_fits_keywords)?;
        }
    }
    // -- End of convert XISF to FITS

    Ok(())
}
