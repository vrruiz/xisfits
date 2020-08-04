use log::info;
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
};

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
#[derive(Debug, Default)]
pub struct FITSKeyword {
    pub name: String,
    pub value: String,
    pub comment: String,
}

// Private functions to write the FITS headers to disk
fn fits_write_header<W>(fits: &mut W, string: &str, bytes: &mut u64) -> io::Result<()>
where
    W: Write,
{
    let mut header = string.to_string();
    header.truncate(80);
    info!("FITS header: \"{}\"", header);
    let header_bytes = header.as_bytes();
    fits.write_all(header_bytes)?;
    *bytes += header_bytes.len() as u64;
    Ok(())
}

fn fits_write_header_u64<W>(
    fits: &mut W,
    header: &str,
    value: u64,
    comment: &str,
    bytes: &mut u64,
) -> io::Result<()>
where
    W: Write,
{
    let string = format!("{:8} = {:<19} / {:47}", header, value, comment);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_header_i64<W>(
    fits: &mut W,
    header: &str,
    value: i64,
    comment: &str,
    bytes: &mut u64,
) -> io::Result<()>
where
    W: Write,
{
    let string = format!("{:8} = {:<19} / {:47}", header, value, comment);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_header_string<W>(
    fits: &mut W,
    header: &str,
    value: &str,
    comment: &str,
    bytes: &mut u64,
) -> io::Result<()>
where
    W: Write,
{
    let string = format!("{:8} = {:<19} / {:48}", header, value, comment);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_header_comment<W>(
    fits: &mut W,
    header: &str,
    comment: &str,
    bytes: &mut u64,
) -> io::Result<()>
where
    W: Write,
{
    let string = format!("{:8}{:72}", header, comment);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_header_no_comment<W>(fits: &mut W, header: &str, bytes: &mut u64) -> io::Result<()>
where
    W: Write,
{
    let string = format!("{:80}", header);
    fits_write_header(fits, &string, bytes)
}

fn fits_write_image_data<W>(fits: &mut W, fits_hd: &FitsHeaderData, _bytes: u64) -> io::Result<()>
where
    W: Write,
{
    // Write Data Unit
    info!("FITS write > Write image data");
    fits.write_all(&fits_hd.data_bytes)?;
    let total = fits_hd.data_bytes.len() as u64;
    let data_unit_rest = total % 2880;
    info!("FITS write > Write image data > Bytes total: {}", total);
    // Write Data Unit (fill the rest of the 2880 byte-block)
    if data_unit_rest > 0 {
        let rest = 2880 - data_unit_rest;
        for _i in 0..rest {
            fits.write_all(&[0])?;
        }
    }

    Ok(())
}

pub fn fits_write_data(filename: &Path, fits_hd: &FitsHeaderData) -> io::Result<()> {
    info!("FITS write > File name > {}", filename.display());
    let mut fits = BufWriter::new(File::create(filename)?);
    let mut bytes = 0;

    // Write HDU
    info!("FITS write > Write headers");
    fits_write_header_string(&mut fits, "SIMPLE", "T", "", &mut bytes)?;
    fits_write_header_i64(&mut fits, "BITPIX", fits_hd.bitpix, "", &mut bytes)?;
    fits_write_header_u64(&mut fits, "NAXIS", fits_hd.naxis, "", &mut bytes)?;
    for i in 0..fits_hd.naxis_vec.len() {
        let header_name = format!("NAXIS{}", i + 1);
        fits_write_header_u64(
            &mut fits,
            &header_name,
            fits_hd.naxis_vec[i],
            "",
            &mut bytes,
        )?;
    }
    fits_write_header_string(&mut fits, "EXTEND", "T", "", &mut bytes)?;
    fits_write_header_string(&mut fits, "BZERO", "0", "", &mut bytes)?;
    fits_write_header_string(&mut fits, "BSCALE", "1", "", &mut bytes)?;
    // fits_write_header_u64(&mut fits, "BSCALE", fits_hd.bscale, ""), &mut bytes)?;
    // fits_write_header_u64(&mut fits, "DATAMIN", fits_hd.datamin, ""), &mut bytes)?;
    // fits_write_header_u64(&mut fits, "DATAMAX", fits_hd.datamax, ""), &mut bytes)?;
    fits_write_header_no_comment(&mut fits, "END", &mut bytes)?;

    // Write HDU (fill the rest of the 2880 byte-block)
    let rest = bytes % 2880;
    if rest > 0 {
        let rest = 2880 - rest;
        for _i in 0..rest {
            fits.write_all(b" ")?;
        }
    }

    // Write Data Unit
    fits_write_image_data(&mut fits, &fits_hd, bytes)?;
    Ok(())
}

// Write FITS data, but use FITS keywords for the header
pub fn fits_write_data_keywords(
    filename: &Path,
    fits_hd: &FitsHeaderData,
    fits_keywords: &[FITSKeyword],
) -> io::Result<()> {
    info!("FITS write > File name > {}", filename.display());
    let mut fits = File::create(filename)?;
    let mut bytes = 0;

    // Write HDU
    info!("FITS write > Write headers");
    for keyword in fits_keywords.iter() {
        if keyword.name == "HISTORY" || keyword.name == "COMMENT" {
            fits_write_header_comment(&mut fits, &keyword.name, &keyword.comment, &mut bytes)?;
        } else {
            fits_write_header_string(
                &mut fits,
                &keyword.name,
                &keyword.value,
                &keyword.comment,
                &mut bytes,
            )?;
        }
    }
    fits_write_header_no_comment(&mut fits, "END", &mut bytes)?;

    // Write Data Unit
    fits_write_image_data(&mut fits, &fits_hd, bytes)?;

    Ok(())
}
