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

// Struct to store FITS keywords
pub struct FITSKeyword {
    pub name:    String,
    pub value:   String,
    pub comment: String,
}

// Private functions to write the FITS headers to disk
fn fits_write_header(fits: &mut File, string: &String, bytes: &mut u64) -> io::Result<()> {
    let mut header = string.clone();
    header.truncate(80);
    println!("FITS header: \"{}\"", header);
    let header_bytes = header.as_bytes();
    fits.write(header_bytes)?;
    *bytes = *bytes + header_bytes.len() as u64;
    Ok(())
}

fn fits_write_header_u64(fits: &mut File, header: &String, value: u64, comment: &String, bytes: &mut u64) -> io::Result<()> {
    let string = format!("{:8} = {:<19} / {:47}", header, value, comment);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_header_i64(fits: &mut File, header: &String, value: i64, comment: &String, bytes: &mut u64) -> io::Result<()> {
    let string = format!("{:8} = {:<19} / {:47}", header, value, comment);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_header_string(fits: &mut File, header: &String, value: &String, comment: &String, bytes: &mut u64) -> io::Result<()> {
    let string = format!("{:8} = {:<19} / {:48}", header, value, comment);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_header_comment(fits: &mut File, header: &String, comment: &String, bytes: &mut u64) -> io::Result<()> {
    let string = format!("{:8}{:72}", header, comment);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_header_no_comment(fits: &mut File, header: &String, bytes: &mut u64) -> io::Result<()> {
    let string = format!("{:80}", header);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_image_data(fits: &mut File, fits_hd: &FitsHeaderData, bytes: &u64) -> io::Result<()> {
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

pub fn fits_write_data(filename: &String, fits_hd: &FitsHeaderData) -> io::Result<()> {
    println!("FITS write > File name > {}", filename);
    let mut fits = File::create(filename)?;
    let mut bytes = 0;

    // Write HDU
    println!("FITS write > Write headers");
    fits_write_header_string(&mut fits, &String::from("SIMPLE"), &String::from("T"), &String::from(""), &mut bytes)?;
    fits_write_header_i64(&mut fits, &String::from("BITPIX"), fits_hd.bitpix, &String::from(""), &mut bytes)?;
    fits_write_header_u64(&mut fits, &String::from("NAXIS"), fits_hd.naxis, &String::from(""), &mut bytes)?;
    for i in 0..fits_hd.naxis_vec.len() {
        let header_name = format!("NAXIS{}", i+1);
        fits_write_header_u64(&mut fits, &header_name, fits_hd.naxis_vec[i], &String::from(""), &mut bytes)?;
    }
    fits_write_header_string(&mut fits, &String::from("EXTEND"), &String::from("T"), &String::from(""), &mut bytes)?;
    fits_write_header_string(&mut fits, &String::from("BZERO"), &String::from("0"), &String::from(""), &mut bytes)?;
    fits_write_header_string(&mut fits, &String::from("BSCALE"), &String::from("1"), &String::from(""), &mut bytes)?;
    // fits_write_header_u64(&mut fits, &String::from("BSCALE"), fits_hd.bscale, &String::from(""), &mut bytes)?;
    // fits_write_header_u64(&mut fits, &String::from("DATAMIN"), fits_hd.datamin, &String::from(""), &mut bytes)?;
    // fits_write_header_u64(&mut fits, &String::from("DATAMAX"), fits_hd.datamax, &String::from(""), &mut bytes)?;
    fits_write_header_no_comment(&mut fits, &String::from("END"), &mut bytes)?;

    // Write HDU (fill the rest of the 2880 byte-block)
    let rest = bytes % 2880;
    if rest > 0 {
        let rest = 2880 - rest;
        for _i in 0..rest {
            fits.write(b" ")?;
        }
    }

    // Write Data Unit
    fits_write_image_data(&mut fits, &fits_hd, &bytes)?;
    Ok(())
}

// Write FITS data, but use FITS keywords for the header
pub fn fits_write_data_keywords(filename: &String, fits_hd: &FitsHeaderData, fits_keywords: &Vec<FITSKeyword>) -> io::Result<()> {
    println!("FITS write > File name > {}", filename);
    let mut fits = File::create(filename)?;
    let mut bytes = 0;

    // Write HDU
    println!("FITS write > Write headers");
    for i in 0..fits_keywords.len() {
        if fits_keywords[i].name == "HISTORY" || fits_keywords[i].name == "COMMENT" {
            fits_write_header_comment(&mut fits, &fits_keywords[i].name, &fits_keywords[i].comment, &mut bytes)?;
        } else {
            fits_write_header_string(&mut fits, &fits_keywords[i].name, &fits_keywords[i].value, &fits_keywords[i].comment, &mut bytes)?;
        }
    }
    fits_write_header_no_comment(&mut fits, &String::from("END"), &mut bytes)?;

    // Write Data Unit
    fits_write_image_data(&mut fits, &fits_hd, &bytes)?;

    Ok(())
}
