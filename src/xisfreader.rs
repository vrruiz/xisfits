use crate::{convert, fitswriter::FITSKeyword, CLI};

use compress::{lz4, zlib};

use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::Path,
    process,
};

// Struct to store XISF header data
#[derive(Debug, Default)]
pub struct XISFHeader {
    pub signature: String,
    pub length: u32,
    pub reserved: u32,
    pub header: String,
    pub geometry: String,
    pub geometry_channels: u32,
    pub geometry_sizes: Vec<u64>,
    pub geometry_channel_size: u64,
    pub sample_format: String,
    pub sample_format_bytes: u8,
    pub color_space: String,
    pub location: String,
    pub location_method: String,
    pub location_start: u64,
    pub location_length: u64,
    pub compression: String,
    pub compression_codec: String,
    pub compression_size: usize,
}

// Struct to store image data as vector
#[derive(Debug, Default)]
pub struct XISFData {
    // pub int8:    Vec<Vec<i8>>,
    pub uint8: Vec<Vec<u8>>,
    // pub int16:   Vec<Vec<i16>>,
    pub uint16: Vec<Vec<u16>>,
    // pub int32:   Vec<Vec<i32>>,
    pub uint32: Vec<Vec<u32>>,
    // pub int64:   Vec<Vec<i64>>,
    // pub uint64:  Vec<Vec<u64>>,
    // pub int128:  Vec<Vec<i128>>,
    // pub uint128: Vec<Vec<u128>>,
    pub float32: Vec<Vec<f32>>,
    pub float64: Vec<Vec<f64>>,
}

/// Gets the size of the XISF type, in bytes.
pub fn xisf_type_size(xisf_type: &str) -> u8 {
    match xisf_type {
        "Int8" | "UInt8" => 1,
        "Int16" | "UInt16" => 2,
        "Int32" | "UInt32" | "Float32" => 4,
        "Int64" | "UInt64" | "Float64" => 8,
        "Int128" | "UInt128" | "Float128" => 16,
        _ => unreachable!(),
    }
}

fn xisf_parse_xml(
    xisf_header: &mut XISFHeader,
    xisf_fits_keywords: &mut Vec<FITSKeyword>,
) -> io::Result<()> {
    // -- Parse XML Header
    // e.g. <Image geometry="256:256:1" sampleFormat="UInt8"
    //       colorSpace="Gray" location="attachment:4096:65536">
    let doc = match roxmltree::Document::parse(&xisf_header.header) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Error: {}.", e);
            process::exit(1);
        }
    };

    for node in doc.descendants() {
        if node.is_element() {
            if CLI.verbose() {
                println!("<{}>", node.tag_name().name());
            }
            match node.tag_name().name() {
                // Parse and store <Image> tag attributes
                "Image" => {
                    for attr in node.attributes() {
                        if CLI.verbose() {
                            println!(
                                "<{} {}=\"{}\">",
                                node.tag_name().name(),
                                attr.name(),
                                attr.value()
                            );
                        }
                        match attr.name() {
                            "geometry" => {
                                // Parse geometry string (size_x:size_y:n)
                                xisf_header.geometry = String::from(attr.value());
                                let geometry_data: Vec<&str> =
                                    xisf_header.geometry.split(':').collect();
                                if geometry_data.len() > 1 {
                                    let mut channel_size = 0;
                                    for g_data in &geometry_data {
                                        let size = g_data.parse::<u64>().unwrap();
                                        if channel_size == 0 {
                                            channel_size = size;
                                        } else {
                                            channel_size *= size;
                                        }
                                        xisf_header.geometry_sizes.push(size);
                                    }
                                    xisf_header.geometry_channel_size = channel_size;
                                    xisf_header.geometry_channels = geometry_data
                                        [geometry_data.len() - 1]
                                        .parse::<u32>()
                                        .unwrap();
                                }
                            }
                            "sampleFormat" => {
                                // Parse image format
                                xisf_header.sample_format = attr.value().to_string();
                                xisf_header.sample_format_bytes =
                                    xisf_type_size(&xisf_header.sample_format);
                            }
                            "colorSpace" => {
                                // Parse space color
                                xisf_header.color_space = attr.value().to_string();
                            }
                            "location" => {
                                // Parse location. Format: "chan_size1:..:chan_size_n:n_channels" format
                                xisf_header.location = attr.value().to_string();
                                let split = xisf_header.location.split(':');
                                for (n, s) in split.enumerate() {
                                    if CLI.verbose() {
                                        println!("Location part: {}", s);
                                    }
                                    if n == 0 {
                                        xisf_header.location_method = s.to_string();
                                    } else if n == 1 {
                                        xisf_header.location_start = s.parse().unwrap();
                                    } else if n == 2 {
                                        // location_length = image data size (compressed)
                                        xisf_header.location_length = s.parse().unwrap();
                                    }
                                }
                            }
                            "compression" => {
                                // Parse compression. Format: "compression_algorithm:uncompressed-size"
                                xisf_header.compression = attr.value().to_string();
                                let split = xisf_header.compression.split(':');
                                for (n, s) in split.enumerate() {
                                    match n {
                                        0 => xisf_header.compression_codec = s.to_string(),
                                        1 => xisf_header.compression_size = s.parse().unwrap(),
                                        _ => (),
                                    }
                                }
                            }
                            _ => {} //name => eprintln!("unknown attribute name {}", name),
                        }
                    }
                }
                "FITSKeyword" => {
                    // Parse and store the values of the FITS keyword
                    let mut xisf_fits_keyword = FITSKeyword {
                        name: String::from(""),
                        value: String::from(""),
                        comment: String::from(""),
                    };
                    for attr in node.attributes() {
                        if attr.name() == "name" {
                            xisf_fits_keyword.name = attr.value().to_string();
                        } else if attr.name() == "value" {
                            xisf_fits_keyword.value = attr.value().to_string();
                        } else if attr.name() == "comment" {
                            xisf_fits_keyword.comment = attr.value().to_string();
                        }
                    }
                    if CLI.verbose() {
                        println!(
                            "FITS Keyword: {} = {} / {}",
                            xisf_fits_keyword.name,
                            xisf_fits_keyword.value,
                            xisf_fits_keyword.comment
                        );
                    }
                    xisf_fits_keywords.push(xisf_fits_keyword);
                }
                _ => {} // tag => eprintln!("unknown tag {}", tag);
            }
        }
    }
    // Calculate the size in bytes of the image
    if xisf_header.sample_format_bytes > 0 {
        xisf_header.geometry_channel_size *= u64::from(xisf_header.sample_format_bytes);
    }

    Ok(())
}

/// Uncompress image data
fn xisf_uncompress_data(xisf_header: &XISFHeader, image_data: &[u8]) -> Vec<u8> {
    if CLI.verbose() {
        println!("Read XISF > Uncompressing");
    }
    let mut decompressed = Vec::new();
    let result;
    // Match compression codec and call decoder
    match xisf_header.compression_codec.as_ref() {
        "zlib" | "zlib+sh" => {
            // Uncompress using zlib decoder
            result =
                zlib::Decoder::new(BufReader::new(&image_data[..])).read_to_end(&mut decompressed);
        }
        "lz4" => {
            // Uncompress using lz4 decoder
            result =
                lz4::Decoder::new(BufReader::new(&image_data[..])).read_to_end(&mut decompressed);
        }
        // "lz4+sh" => {} // Gives error with lz4 decoder
        // "lz4hc" => {} // Not supported by lz4 decoder
        _ => {
            // Unsupported codec. Abort.
            eprintln!(
                "Read XISF > Uncompressing > Unsupported codec: {}",
                xisf_header.compression_codec
            );
            process::exit(1);
        }
    }
    match result {
        Ok(_v) => {
            // Data uncompressed
            if CLI.verbose() {
                println!("Read XISF > Uncompressed size: {}", decompressed.len());
            }
            // If expected size doesn't match, abort
            if decompressed.len() != xisf_header.compression_size {
                eprintln!(
                    "Read XISF > Uncompressing > Sizes don't match. Uncompressed: {} Expected: {}",
                    image_data.len(),
                    xisf_header.compression_size
                );
                process::exit(1);
            }
        }
        Err(r) => {
            eprintln!("Read XISF > Uncompressing > Cannot uncompress: {}", r);
            process::exit(1);
        }
    }
    decompressed
}

// Read XISF file and decode headers and image
pub fn xisf_read_file(
    xisf_filename: &Path,
    xisf_header: &mut XISFHeader,
    xisf_data: &mut XISFData,
    xisf_fits_keywords: &mut Vec<FITSKeyword>,
) -> io::Result<()> {
    // Declare buffers
    let mut buffer_header_signature = String::new();
    let mut buffer_header_length = [0; 4];
    let mut buffer_header_reserved = [0; 4];
    let mut buffer_header_header = String::new();

    // Open XISF image file
    let f = File::open(xisf_filename)?;
    let file_size = f.metadata().unwrap().len();
    if CLI.verbose() {
        println!("File size: {}", file_size);
    }
    let mut f = BufReader::new(f);

    // -- Read header fields
    // Header: Signature
    let _ = f
        .by_ref()
        .take(8)
        .read_to_string(&mut buffer_header_signature)?;
    // Header: Length of XML section
    f.read_exact(&mut buffer_header_length)?;
    // Header: Reserved for future use
    f.read_exact(&mut buffer_header_reserved)?;

    // Header: XML section
    let mut handle = f
        .by_ref()
        .take(u64::from(convert::u8_to_v_u32(&buffer_header_length)[0]));
    let _ = handle.read_to_string(&mut buffer_header_header)?;

    // Assign header values to XISF header struct
    xisf_header.signature = buffer_header_signature.clone();
    xisf_header.length = convert::u8_to_v_u32(&buffer_header_length)[0];
    xisf_header.reserved = convert::u8_to_v_u32(&buffer_header_reserved)[0];
    xisf_header.header = buffer_header_header.clone();
    // -- End of read header fields

    if CLI.verbose() {
        // Print header values
        println!("{}", xisf_header.signature);
        if xisf_header.signature == "XISF0100" {
            println!("XISF signature: Ok");
        }
        println!("Length: {}", xisf_header.length);
        println!("Reserved: {}", xisf_header.reserved);
        println!("Header: {}", xisf_header.header);
    }

    // Parse XML Header section
    xisf_parse_xml(xisf_header, xisf_fits_keywords)?;

    if CLI.verbose() {
        // Output parsed data
        println!("Geometry: {}", xisf_header.geometry);
        println!("Geometry sizes: {:?}", xisf_header.geometry_sizes);
        println!("Geometry channels: {}", xisf_header.geometry_channels);
        println!(
            "Geometry channel size: {}",
            xisf_header.geometry_channel_size
        );
        println!("Sample format: {}", xisf_header.sample_format);
        println!("Sample format: {}", xisf_header.sample_format_bytes);
        println!("Color space: {}", xisf_header.color_space);
        println!("Location: {}", xisf_header.location);
        println!("Location method: {}", xisf_header.location_method);
        println!("Location start: {}", xisf_header.location_start);
        println!("Location length: {}", xisf_header.location_length);
        println!(
            "Location length ({}) == channel size * channels ({})",
            xisf_header.location_length,
            xisf_header.geometry_channel_size * u64::from(xisf_header.geometry_channels)
        );
        println!(
            "Compression: {} {} {}",
            xisf_header.compression, xisf_header.compression_codec, xisf_header.compression_size
        );
    }

    // Stop if data is compressed
    if CLI.verbose() {
        if xisf_header.compression.is_empty() {
            println!("Read XISF > Data uncompressed.");
        } else {
            println!("Read XISF > Data compressed.");
        }
    }

    // Interpret it as numbers and store as vector/s
    if xisf_header.location_method == "attachment" && 
        // Goto to file position where the image begins
        xisf_header.location_start + xisf_header.location_length <= file_size
    {
        match f.seek(SeekFrom::Start(xisf_header.location_start)) {
            Ok(v) => {
                if CLI.verbose() {
                    println!("Read XISF > File correctly seek: {:?}", v)
                }
            }
            Err(r) => eprintln!("Read XISF > Error seeking file: {:?}", r),
        }

        let mut image_data = Vec::new();
        // Read image size bytes
        match f
            .by_ref()
            .take(xisf_header.location_length)
            .read_to_end(&mut image_data)
        {
            Ok(v) => {
                if CLI.verbose() {
                    println!("Read XISF > Data correctly read: {:?}", v)
                }
            }
            Err(r) => eprintln!("Read XISF > Error reading image: {:?}", r),
        };

        // Uncompress data
        if !xisf_header.compression_codec.is_empty() {
            image_data = xisf_uncompress_data(&xisf_header, &image_data[..]);
        }

        // Read each channel
        for n in 0..xisf_header.geometry_channels {
            let image_channel = &image_data[(xisf_header.geometry_channels * n) as usize
                ..(xisf_header.geometry_channels * (n + 1) - 1) as usize];

            // Convert bytes to actual numbers and store the channel in a vector
            match xisf_header.sample_format.as_str() {
                "UInt8" => xisf_data.uint8.push(image_channel.to_vec()),
                "UInt16" => xisf_data.uint16.push(convert::u8_to_v_u16(&image_channel)),
                "UInt32" => xisf_data.uint32.push(convert::u8_to_v_u32(&image_channel)),
                "Float32" => xisf_data.float32.push(convert::u8_to_v_f32(&image_channel)),
                "Float64" => xisf_data.float64.push(convert::u8_to_v_f64(&image_channel)),
                _ => eprintln!(
                    "Read XISF > Unsupported type > {}",
                    xisf_header.sample_format.as_str()
                ),
            }

            if CLI.verbose() {
                // Show the first 20 bytes of the original image data
                if image_channel.len() >= 20 {
                    for byte in image_channel.iter().take(20) {
                        print!("{:x} ", byte);
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
    // -- End of read image data from file
}
