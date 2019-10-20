use std::fs::File;
use std::io;
use std::io::Write;

pub struct FitsHeaderData {
    pub bitpix: i64,
    pub naxis: u64,
    pub naxis_vec: Vec<u64>,
    pub bzero: u64,
    pub bscale: u64,
    pub datamin: u64,
    pub datamax: u64,
    pub history: Vec<String>,
    pub comment: Vec<String>,
    pub data_bytes: Vec<u8>,
}

fn fits_write_header(fits: &mut File, string: &String, bytes: &mut u64) -> io::Result<()> {
    println!("FITS header: \"{}\"", string);
    let header_bytes = string.as_bytes();
    fits.write(header_bytes)?;
    *bytes = *bytes + header_bytes.len() as u64;
    Ok(())
}

fn fits_write_header_u64(fits: &mut File, header: &String, value: u64, bytes: &mut u64) {
    let string = format!("{:8} = {:>19} /{:48}", header, value, "");
    fits_write_header(fits, &string, bytes);
}

fn fits_write_header_i64(fits: &mut File, header: &String, value: i64, bytes: &mut u64) {
    let string = format!("{:8} = {:>19} /{:48}", header, value, "");
    fits_write_header(fits, &string, bytes);
}

fn fits_write_header_string(fits: &mut File, header: &String, value: &String, bytes: &mut u64) {
    let string = format!("{:8} = {:>19} /{:48}", header, value, "");
    fits_write_header(fits, &string, bytes);
}

fn fits_write_header_no_comment(fits: &mut File, header: &String, bytes: &mut u64) {
    let string = format!("{:<80}", header);
    fits_write_header(fits, &string, bytes);
}

pub fn fits_write(filename: &String, fits_hd: &FitsHeaderData) -> io::Result<()> {
    println!("FITS write > File name > {}", filename);
    let mut fits = File::create(filename)?;
    let mut bytes = 0;

    // Write HDU
    println!("FITS write > Write headers");
    fits_write_header_string(&mut fits, &String::from("SIMPLE"), &String::from("T"), &mut bytes);
    fits_write_header_i64(&mut fits, &String::from("BITPIX"), fits_hd.bitpix, &mut bytes);
    fits_write_header_u64(&mut fits, &String::from("NAXIS"), fits_hd.naxis, &mut bytes);
    for i in 0..fits_hd.naxis_vec.len() {
        let header_name = format!("NAXIS{}", i+1);
        fits_write_header_u64(&mut fits, &header_name, fits_hd.naxis_vec[i], &mut bytes);
    }
    fits_write_header_string(&mut fits, &String::from("EXTEND"), &String::from("T"), &mut bytes);
    fits_write_header_string(&mut fits, &String::from("BZERO"), &String::from("0"), &mut bytes);
    fits_write_header_string(&mut fits, &String::from("BSCALE"), &String::from("1"), &mut bytes);
    // fits_write_header_u64(&mut fits, &String::from("BSCALE"), fits_hd.bscale, &mut bytes);
    // fits_write_header_u64(&mut fits, &String::from("DATAMIN"), fits_hd.datamin, &mut bytes);
    // fits_write_header_u64(&mut fits, &String::from("DATAMAX"), fits_hd.datamax, &mut bytes);
    fits_write_header_no_comment(&mut fits, &String::from("END"), &mut bytes);

    // Write HDU (fill the rest of the 2880 byte-block)
    let rest = bytes % 2880;
    if rest > 0 {
        let rest = 2880 - rest;
        for _i in 0..rest {
            fits.write(b" ")?;
        }
    }

    // Write Data Unit
    println!("FITS write > Write image data");
    fits.write(&fits_hd.data_bytes)?;
    let total = fits_hd.data_bytes.len() as u64;
    let rest = total % 2880;
    println!("FITS write > Write image data > Bytes total: {}", total);
    // Write Data Unit (fill the rest of the 2880 byte-block)
    if rest > 0 {
        let rest = 2880 - rest;
        for _i in 0..rest {
            fits.write(&[0])?;
        }
    }

    Ok(())
}
