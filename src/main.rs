extern crate roxmltree;

mod convert;
mod fitswriter;

use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs::File;
use std::env;
use std::process;
use std::collections::HashMap;

// Struct to store XISF header data
struct XISFHeader {
    signature: String,
    length: u32,
    reserved: u32,
    header: String,
    geometry: String,
    geometry_channels: u64,
    geometry_sizes: Vec<u64>,
    geometry_channel_size: u64,
    sample_format: String,
    sample_format_bytes: u8,
    color_space: String,
    location: String,
    location_method: String,
    location_start: u64,
    location_length: u64,
}

// Struct to store image data as vector
struct XISFData {
    format:  String,
    int8:    Vec<Vec<i8>>,
    uint8:   Vec<Vec<u8>>,
    int16:   Vec<Vec<i16>>,
    uint16:  Vec<Vec<u16>>,
    int32:   Vec<Vec<i32>>,
    uint32:  Vec<Vec<u32>>,
    int64:   Vec<Vec<i64>>,
    uint64:  Vec<Vec<u64>>,
    int128:  Vec<Vec<i128>>,
    uint128: Vec<Vec<u128>>,
    float32: Vec<Vec<f32>>,
    float64: Vec<Vec<f64>>,
}

fn main() -> io::Result<()> {
    // Define variables
    let mut xisf_header = XISFHeader {
        signature: "".to_string(),
        length: 0,
        reserved: 0,
        header: "".to_string(),
        geometry: "".to_string(),
        geometry_channels: 0,
        geometry_sizes: vec![],
        geometry_channel_size: 0,
        sample_format: "".to_string(),
        sample_format_bytes: 0,
        color_space: "".to_string(),
        location: "".to_string(),
        location_method: "".to_string(),
        location_start: 0,
        location_length: 0,
    };

    let mut xisf_data = XISFData {
        format:  "".to_string(),
        int8:    vec![],
        uint8:   vec![],
        int16:   vec![],
        uint16:  vec![],
        int32:   vec![],
        uint32:  vec![],
        int64:   vec![],
        uint64:  vec![],
        int128:  vec![],
        uint128: vec![],
        float32: vec![],
        float64: vec![],
    };

    // Fundamental Scalar Types
    let xisf_type_size: HashMap<&str, u8> =
        [("Int8", 1),
         ("UInt8", 1),
         ("Int16", 2),
         ("UInt16", 2),
         ("Int32", 4),
         ("UInt32", 4),
         ("Int32", 4),
         ("Int64", 8),
         ("UInt64", 8),
         ("Int128", 16),
         ("UInt128", 16),
         ("Float32", 4),
         ("Float64", 8),
         ("Float128", 16),
        ].iter().cloned().collect();

    let mut buffer_header_signature = String::new();
    let mut buffer_header_length = [0; 4];
    let mut buffer_header_reserved = [0; 4];
    let mut buffer_header_header = String::new();

    // Read command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("xisfits [input file name].xifs [output file name].fits");
        process::exit(1);
    }
    println!("Args: {:?}", args);
 
    // Open XISF image file
    let xisf_filename = &args[1];
    let fits_filename = &args[2];
    let mut f = File::open(xisf_filename)?;
    let file_size = f.metadata().unwrap().len();
    println!("File size: {}", file_size);

    // -- Read header fields
    // Header: Signature
    f.by_ref().take(8).read_to_string(&mut buffer_header_signature)?;
    // Header: Length of XML section
    f.read_exact(&mut buffer_header_length)?;
    // Header: Reserved for future use
    f.read_exact(&mut buffer_header_reserved)?;

    // Header: XML section
    let mut handle = f.by_ref().take(convert::u8_to_u32(&buffer_header_length)[0] as u64);
    handle.read_to_string(&mut buffer_header_header)?;

    // Assign header values to XISF header struct
    xisf_header.signature = buffer_header_signature.clone();
    xisf_header.length = convert::u8_to_u32(&buffer_header_length)[0];
    xisf_header.reserved = convert::u8_to_u32(&buffer_header_reserved)[0];
    xisf_header.header = buffer_header_header.clone();
    // -- End of read header fields

    // Print header values
    println!("{}", xisf_header.signature);
    if xisf_header.signature == "XISF0100" {
        println!("XISF signature: Ok");
    }
    println!("Length: {}", xisf_header.length);
    println!("Reserved: {}", xisf_header.reserved);
    println!("Header: {}", xisf_header.header);

    // -- Parse XML Header
    // e.g. <Image geometry="256:256:1" sampleFormat="UInt8"
    //       colorSpace="Gray" location="attachment:4096:65536">
    let doc = match roxmltree::Document::parse(&xisf_header.header) {
        Ok(doc) => doc,
        Err(e) => {
            println!("Error: {}.", e);
            process::exit(1);
        },
    };

    for node in doc.descendants() {
        if node.is_element() {
            println!("<{}>", node.tag_name().name());
            if node.tag_name().name() == "Image" {
                // Parse and store <Image> tag attributes
                for attr in node.attributes() {
                    println!("<{} {}=\"{}\">", node.tag_name().name(), attr.name(), attr.value());
                    if attr.name() == "geometry" {
                        xisf_header.geometry = attr.value().to_string();
                        // Parse geometry string (size_x:size_y:n)
                        let geometry_data: Vec<&str> = xisf_header.geometry.split(":").collect();
                        if geometry_data.len() > 1 {
                            let mut channel_size = 0;
                            for i in 0..geometry_data.len() - 1 {
                                let size = geometry_data[i].parse::<u64>().unwrap();
                                if channel_size == 0 {
                                    channel_size = size;
                                } else {
                                    channel_size = channel_size * size;
                                }
                                xisf_header.geometry_sizes.push(size);
                            }
                            xisf_header.geometry_channel_size = channel_size.into();
                            xisf_header.geometry_channels = geometry_data[geometry_data.len() - 1].parse::<u64>().unwrap();
                        }
                    } else if attr.name() == "sampleFormat" {
                        // Parse image format
                        xisf_header.sample_format = attr.value().to_string();
                        xisf_header.sample_format_bytes = xisf_type_size[xisf_header.sample_format.as_str()];
                    } else if attr.name() == "colorSpace" {
                        // Parse space color
                        xisf_header.color_space = attr.value().to_string();
                    } else if attr.name() == "location" {
                        // Parse location. Format: "chan_size1:..:chan_size_n:n_channels" format
                        xisf_header.location = attr.value().to_string();
                        let split = xisf_header.location.split(":");
                        let mut n = 0;
                        for s in split {
                            println!("Location part: {}", s);
                            if n == 0 {
                                xisf_header.location_method = s.to_string();
                            } else if n == 1 {
                                xisf_header.location_start = s.parse().unwrap();
                            } else if n == 2 {
                                xisf_header.location_length = s.parse().unwrap();
                            }
                            n += 1;
                        }
                    }
                }
                // NOTE: location_length == geometry x * geometry y * ... * geometry n.
            }
        }
    }
    // Calculate the size in bytes of the image 
    if xisf_header.sample_format_bytes > 0 {
        xisf_header.geometry_channel_size = xisf_header.geometry_channel_size * xisf_header.sample_format_bytes as u64;
    }
    // -- End of parse XML Header

    // Output parsed data
    println!("Geometry: {}", xisf_header.geometry);
    println!("Geometry sizes: {:?}", xisf_header.geometry_sizes);
    println!("Geometry channels: {}", xisf_header.geometry_channels);
    println!("Geometry channel size: {}", xisf_header.geometry_channel_size);
    println!("Sample format: {}", xisf_header.sample_format);
    println!("Sample format: {}", xisf_header.sample_format_bytes);
    println!("Color space: {}", xisf_header.color_space);
    println!("Location: {}", xisf_header.location);
    println!("Location method: {}", xisf_header.location_method);
    println!("Location start: {}", xisf_header.location_start);
    println!("Location length: {}", xisf_header.location_length);
    println!("Location length ({}) == channel size * channels ({})", xisf_header.location_length, xisf_header.geometry_channel_size * xisf_header.geometry_channels);

    // -- Read image data from file
    // Interpret it as numbers and store as vector/s
    if xisf_header.location_method == "attachment" && 
        // Goto to file position where the image begins
        xisf_header.location_start + xisf_header.location_length <= file_size {
        match f.seek(SeekFrom::Start(xisf_header.location_start)) {
            Ok(v) => println!("Read XISF > File correctly seek: {:?}", v),
            Err(r) => println!("Read XISF > Error seeking file: {:?}", r)
        }

        // Read each channel
        for n in 0..xisf_header.geometry_channels {
            let mut image_channel = Vec::new();
            // Read channel size bytes
            match f.by_ref().take(xisf_header.geometry_channel_size.into()).read_to_end(&mut image_channel) {
                Ok(v) => println!("Read XISF > Data correctly read (channel {}): {:?}", n, v),
                Err(r) => println!("Read XISF > Error reading image (channel {}): {:?}", n, r)
            };

            // Convert bytes to actual numbers and store the channel in a vector
            match xisf_header.sample_format.as_str() {                
                "UInt8" => xisf_data.uint8.push(image_channel.clone()),
                "UInt16" => xisf_data.uint16.push(convert::u8_to_u16(&image_channel)),
                "UInt32" => xisf_data.uint32.push(convert::u8_to_u32(&image_channel)),
                "Float32" => xisf_data.float32.push(convert::u8_to_f32(&image_channel)),
                "Float64" => xisf_data.float64.push(convert::u8_to_f64(&image_channel)),
                 _ => println!("Read XISF > Unsupported type > {}", xisf_header.sample_format.as_str()),
            }

            // Show the first 20 bytes of the original image data
            if image_channel.len() >= 20 {
                for i in 0..20 {
                    print!("{:x} ", image_channel[i]);
                }
                println!("");
            }
        }
    }
    // -- End of read image data from file

    // -- Convert XISF to FITS
    println!("Convert to FITS > Image data to bytes");
    let mut data_bytes: Vec<u8> = vec![];
    let mut bitpix: i64 = 0;
    // Convert binary formats
    //
    // +--------+-------+------+
    // | XISF   > Rust  > FITS |
    // +--------+-------+------+
    // | UInt8  | u8    | 8    |
    // | UInt16 | i16   | 16   |
    // | UInt32 | i32   | 32   |
    // +--------+-------+------+
    //
    for i in 0..xisf_header.geometry_channels as usize {
            match xisf_header.sample_format.as_str() {
                "UInt8" => {
                    bitpix = 8;
                    data_bytes.append(&mut xisf_data.uint8[i]);
                },
                "UInt16" => {
                    bitpix = 16;
                    data_bytes.append(&mut convert::u16_to_i16_to_u8_be(&xisf_data.uint16[i]));
                },
                "UInt32" => {
                    bitpix = 32;
                    data_bytes.append(&mut convert::u32_to_i32_to_u8_be(&xisf_data.uint32[i]));
                },
                // "Float32" =>  { },
                // "Float64" =>  { },
                 _ => println!("Convert to FITS > Unsupported XISF type > {}", xisf_header.sample_format.as_str()),
            }
    }

    // Show the first 20 bytes of the converted image
    if data_bytes.len() > 20 {
        for p in 0..20 {
            print!("{:x} ", data_bytes[p]);
        }
        println!();
    }

    // Write FITS image to disk
    if bitpix > 0 {
        println!("Convert to FITS > Write image data");
        let fits_hd = fitswriter::FitsHeaderData {
            bitpix: bitpix,
            naxis: xisf_header.geometry_sizes.len() as u64,
            naxis_vec: xisf_header.geometry_sizes,
            bzero: 0,
            bscale: 1,
            datamin: 0,
            datamax: 2u64.pow((xisf_header.sample_format_bytes * 8).into()) - 1,
            history: vec!["".to_string()],
            comment: vec!["".to_string()],
            data_bytes: data_bytes,
        };
        fitswriter::fits_write(fits_filename, &fits_hd)?;
    }
    // -- End of convert XISF to FITS

    Ok(())
}