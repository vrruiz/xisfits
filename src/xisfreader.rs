use crate::{convert, fitswriter::FITSKeyword};

use compress::{lz4, zlib};
use getset::{CopyGetters, Getters};
use log::{debug, info};
use quick_xml::{events::Event, Reader};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
    process, str,
};

/// XISF file information structure.
#[derive(Debug)]
pub struct XISFile {
    header: XISFHeader,
    keywords: Box<[FITSKeyword]>,
    data: XISFData,
}

impl XISFile {
    pub fn header(&self) -> &XISFHeader {
        &self.header
    }

    pub fn keywords(&self) -> &[FITSKeyword] {
        &self.keywords
    }

    pub fn data(&self) -> &XISFData {
        &self.data
    }

    /// Read XISF file and decode headers and image
    pub fn read_file(xisf_filename: &Path) -> io::Result<Self> {
        let mut xisf_header = XISFHeaderReader::default();
        let mut xisf_data = XISFData::default();
        let mut xisf_fits_keywords = Vec::new();

        // Declare buffers
        let mut buffer_header_signature = String::new();
        let mut buffer_header_length = [0; 4];
        let mut buffer_header_reserved = [0; 4];

        // Open XISF image file
        let f = File::open(xisf_filename)?;
        let file_size = f.metadata().unwrap().len();
        let mut f = BufReader::new(f);
        info!("File size: {}", file_size);

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

        // Assign header values to XISF header struct
        xisf_header.signature = buffer_header_signature;
        xisf_header.length = convert::u8_to_v_u32(&buffer_header_length)[0];
        xisf_header.reserved = convert::u8_to_v_u32(&buffer_header_reserved)[0];
        // -- End of read header fields

        // Header: XML section
        let handle = f
            .by_ref()
            .take(u64::from(convert::u8_to_v_u32(&buffer_header_length)[0]));

        // Parse XML Header section
        xisf_header.fill_from_reader(handle, &mut xisf_fits_keywords)?;
        let xisf_header = xisf_header.build();

        // Check signature
        if xisf_header.signature() == "XISF0100" {
            info!("XISF signature: Ok");
        } else {
            eprintln!("Incorrect XISF signature: {}", xisf_header.signature());
            process::exit(1);
            // TODO: proper error handling
        }

        // Output parsed data
        xisf_header.print_info();

        // Stop if data is compressed
        if xisf_header.compression().is_empty() {
            info!("Read XISF > Data uncompressed.");
        } else {
            info!("Read XISF > Data compressed.");
        }

        // Interpret it as numbers and store as vector/s
        if xisf_header.location_method() == "attachment" && 
        // Goto to file position where the image begins
        xisf_header.location_start() + xisf_header.location_length() <= file_size
        {
            match f.seek(SeekFrom::Start(xisf_header.location_start())) {
                Ok(v) => {
                    info!("Read XISF > File correctly seek: {:?}", v);
                }
                Err(r) => {
                    eprintln!("Read XISF > Error seeking file: {:?}", r);
                    process::exit(1);
                    // TODO: better error handling
                }
            }

            let mut image_data = Vec::new();
            // Read image size bytes
            match f
                .by_ref()
                .take(xisf_header.location_length())
                .read_to_end(&mut image_data)
            {
                Ok(v) => {
                    info!("Read XISF > Data correctly read: {:?}", v);
                }
                Err(r) => {
                    eprintln!("Read XISF > Error reading image: {:?}", r);
                }
            };

            // Uncompress data
            let image_data = if !xisf_header.compression_codec().is_empty() {
                xisf_uncompress_data(&xisf_header, &image_data[..])
            } else {
                image_data.into_boxed_slice()
            };

            // Read each channel
            for n in 0..xisf_header.geometry_channels() {
                let image_channel = &image_data[(xisf_header.geometry_channel_size() as u32 * n)
                    as usize
                    ..(xisf_header.geometry_channel_size() as u32 * (n + 1)) as usize];

                // Convert bytes to actual numbers and store the channel in a vector
                match xisf_header.sample_format() {
                    XISFType::UInt8 => xisf_data.uint8.push(image_channel.to_vec()),
                    XISFType::UInt16 => xisf_data.uint16.push(convert::u8_to_v_u16(&image_channel)),
                    XISFType::UInt32 => xisf_data.uint32.push(convert::u8_to_v_u32(&image_channel)),
                    XISFType::Float32 => {
                        xisf_data.float32.push(convert::u8_to_v_f32(&image_channel))
                    }
                    XISFType::Float64 => {
                        xisf_data.float64.push(convert::u8_to_v_f64(&image_channel))
                    }
                    _ => eprintln!(
                        "Read XISF > Unsupported type > {}",
                        xisf_header.sample_format().as_str()
                    ),
                }
            }
        }

        Ok(XISFile {
            header: xisf_header,
            keywords: xisf_fits_keywords.into_boxed_slice(),
            data: xisf_data,
        })
        // -- End of read image data from file
    }
}

// Struct to read XISF header data
#[derive(Debug, Getters, CopyGetters)]
pub struct XISFHeader {
    signature: Box<str>,
    #[getset(get_copy = "pub")]
    length: u32,
    #[getset(get_copy = "pub")]
    reserved: u32,
    geometry: Box<str>,
    #[getset(get_copy = "pub")]
    geometry_channels: u32,
    geometry_sizes: Box<[u64]>,
    #[getset(get_copy = "pub")]
    geometry_channel_size: u64,
    #[getset(get_copy = "pub")]
    sample_format: XISFType,
    color_space: Box<str>,
    location: Box<str>,
    location_method: Box<str>,
    #[getset(get_copy = "pub")]
    location_start: u64,
    #[getset(get_copy = "pub")]
    location_length: u64,
    compression: Box<str>,
    compression_codec: Box<str>,
    #[getset(get_copy = "pub")]
    compression_size: usize,
}

impl XISFHeader {
    pub fn signature(&self) -> &str {
        &self.signature
    }

    pub fn geometry(&self) -> &str {
        &self.geometry
    }

    pub fn geometry_sizes(&self) -> &[u64] {
        &self.geometry_sizes
    }

    pub fn color_space(&self) -> &str {
        &self.color_space
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    pub fn location_method(&self) -> &str {
        &self.location_method
    }

    pub fn compression(&self) -> &str {
        &self.compression
    }

    pub fn compression_codec(&self) -> &str {
        &self.compression_codec
    }

    /// Print header data
    fn print_info(&self) {
        // Print header values
        info!("Header signature: {}", self.signature());

        info!("Length: {}", self.length());
        info!("Reserved: {}", self.reserved());

        info!("Geometry: {}", self.geometry());
        info!("Geometry sizes: {:?}", self.geometry_sizes());
        info!("Geometry channels: {}", self.geometry_channels());
        info!("Geometry channel size: {}", self.geometry_channel_size());
        info!("Sample format: {}", self.sample_format().as_str());
        info!("Sample format bytes: {}", self.sample_format().size());
        info!("Color space: {}", self.color_space());
        info!("Location: {}", self.location());
        info!("Location method: {}", self.location_method());
        info!("Location start: {}", self.location_start());
        info!("Location length: {}", self.location_length());
        info!(
            "Location length ({}) == channel size * channels ({})",
            self.location_length(),
            self.geometry_channel_size() * u64::from(self.geometry_channels())
        );
        info!(
            "Compression: {} {} {}",
            self.compression(),
            self.compression_codec(),
            self.compression_size()
        );
    }
}

// Struct to read XISF header data
#[derive(Debug, Default)]
struct XISFHeaderReader {
    signature: String,
    length: u32,
    reserved: u32,
    geometry: String,
    geometry_channels: u32,
    geometry_sizes: Vec<u64>,
    geometry_channel_size: u64,
    sample_format: Option<XISFType>,
    color_space: String,
    location: String,
    location_method: String,
    location_start: u64,
    location_length: u64,
    compression: String,
    compression_codec: String,
    compression_size: usize,
}

impl XISFHeaderReader {
    /// Parse XISF's XML header and add it to this header information.
    fn fill_from_reader<R>(
        &mut self,
        reader: R,
        xisf_fits_keywords: &mut Vec<FITSKeyword>,
    ) -> io::Result<()>
    where
        R: BufRead,
    {
        // -- Parse XML Header
        // e.g. <Image geometry="256:256:1" sampleFormat="UInt8"
        //       colorSpace="Gray" location="attachment:4096:65536">
        let mut reader = Reader::from_reader(reader);
        let mut buf = Vec::new();

        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    info!("<{}>", String::from_utf8_lossy(e.name()));
                    match e.name() {
                        b"Image" => {
                            // Parse and store <Image> tag attributes
                            for attr in e.attributes() {
                                let attr = attr.unwrap();
                                info!(
                                    "<{} {}=\"{}\">",
                                    String::from_utf8_lossy(e.name()),
                                    String::from_utf8_lossy(&attr.key),
                                    String::from_utf8_lossy(&attr.value),
                                );
                                match attr.key {
                                    b"geometry" => {
                                        // Parse geometry string (size_x:size_y:n)
                                        self.geometry =
                                            str::from_utf8(&attr.value).unwrap().to_owned();

                                        // TODO: allow for more geometries
                                        if self.geometry.contains(':') {
                                            let mut iter = self.geometry.split(':');

                                            let size_x =
                                                iter.next().unwrap().parse::<u64>().unwrap();
                                            let size_y =
                                                iter.next().unwrap().parse::<u64>().unwrap();
                                            let num_channels =
                                                iter.next().unwrap().parse::<u32>().unwrap();

                                            self.geometry_channel_size = size_x * size_y;
                                            self.geometry_channels = num_channels;
                                        }
                                    }
                                    b"sampleFormat" => {
                                        // Parse image format
                                        self.sample_format = Some(
                                            str::from_utf8(&attr.value).unwrap().parse().unwrap(),
                                        );
                                    }
                                    b"colorSpace" => {
                                        // Parse space color
                                        self.color_space =
                                            str::from_utf8(&attr.value).unwrap().to_owned();
                                    }
                                    b"location" => {
                                        // Parse location. Format: "chan_size1:..:chan_size_n:n_channels" format
                                        self.location =
                                            str::from_utf8(&attr.value).unwrap().to_owned();
                                        let split = self.location.split(':');
                                        for (n, s) in split.enumerate() {
                                            info!("Location part: {}", s);
                                            if n == 0 {
                                                self.location_method = s.to_owned();
                                            } else if n == 1 {
                                                self.location_start = s.parse().unwrap();
                                            } else if n == 2 {
                                                // location_length = image data size (compressed)
                                                self.location_length = s.parse().unwrap();
                                            }
                                        }
                                    }
                                    b"compression" => {
                                        // Parse compression. Format: "compression_algorithm:uncompressed-size"
                                        self.compression =
                                            str::from_utf8(&attr.value).unwrap().to_owned();
                                        let mut iter = self.compression.split(':');

                                        self.compression_codec = iter.next().unwrap().to_owned();
                                        self.compression_size =
                                            iter.next().unwrap().parse().unwrap();
                                    }
                                    _ => {} //name => eprintln!("unknown attribute name {}", name),
                                }
                            }
                        }
                        b"FITSKeyword" => {
                            // Parse and store the values of the FITS keyword
                            let mut xisf_fits_keyword = FITSKeyword::default();

                            for attr in e.attributes() {
                                let attr = attr.unwrap();

                                let value = str::from_utf8(&attr.value).unwrap().to_owned();
                                match attr.key {
                                    b"name" => {
                                        xisf_fits_keyword.name = value;
                                    }
                                    b"value" => {
                                        xisf_fits_keyword.value = value;
                                    }
                                    b"comment" => xisf_fits_keyword.comment = value,
                                    _ => {}
                                }
                            }

                            info!(
                                "FITS Keyword: {} = {} / {}",
                                xisf_fits_keyword.name,
                                xisf_fits_keyword.value,
                                xisf_fits_keyword.comment
                            );
                            xisf_fits_keywords.push(xisf_fits_keyword);
                        }
                        tag => debug!("unknown tag {}", String::from_utf8_lossy(tag)),
                    }
                }
                Ok(Event::Eof) => break, // exits the loop when reaching end of file
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                Ok(_) => (), // There are several other `Event`s we do not consider here
            }

            // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
            buf.clear();
        }

        // Calculate the size in bytes of the image
        let sample_format_bytes = self.sample_format.unwrap().size();
        if sample_format_bytes > 0 {
            self.geometry_channel_size *= u64::from(sample_format_bytes);
        }

        Ok(())
    }

    /// Builds the final header.
    fn build(self) -> XISFHeader {
        XISFHeader {
            signature: self.signature.into_boxed_str(),
            length: self.length,
            reserved: self.reserved,
            geometry: self.geometry.into_boxed_str(),
            geometry_channels: self.geometry_channels,
            geometry_sizes: self.geometry_sizes.into_boxed_slice(),
            geometry_channel_size: self.geometry_channel_size,
            sample_format: self.sample_format.unwrap(), // TODO: proper error handling
            color_space: self.color_space.into_boxed_str(),
            location: self.location.into_boxed_str(),
            location_method: self.location_method.into_boxed_str(),
            location_start: self.location_start,
            location_length: self.location_length,
            compression: self.compression.into_boxed_str(),
            compression_codec: self.compression_codec.into_boxed_str(),
            compression_size: self.compression_size,
        }
    }
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

/// Enumeration with the different XISF types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XISFType {
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Float32,
    Int64,
    UInt64,
    Float64,
    Int128,
    UInt128,
    Float128,
}

impl XISFType {
    /// Gets the size of the XISF type, in bytes.
    fn size(self) -> u8 {
        match self {
            Self::Int8 | Self::UInt8 => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int32 | Self::UInt32 | Self::Float32 => 4,
            Self::Int64 | Self::UInt64 | Self::Float64 => 8,
            Self::Int128 | Self::UInt128 | Self::Float128 => 16,
        }
    }

    /// Gets the XISF type as a string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Int8 => "Int8",
            Self::UInt8 => "UInt8",
            Self::Int16 => "Int16",
            Self::UInt16 => "UInt16",
            Self::Int32 => "Int32",
            Self::UInt32 => "UInt32",
            Self::Float32 => "Float32",
            Self::Int64 => "Int64",
            Self::UInt64 => "UInt64",
            Self::Float64 => "Float64",
            Self::Int128 => "Int128",
            Self::UInt128 => "UInt128",
            Self::Float128 => "Float128",
        }
    }
}

impl str::FromStr for XISFType {
    type Err = String; // TODO: propper error handling.

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Int8" => Ok(Self::Int8),
            "UInt8" => Ok(Self::UInt8),
            "Int16" => Ok(Self::Int16),
            "UInt16" => Ok(Self::UInt16),
            "Int32" => Ok(Self::Int32),
            "UInt32" => Ok(Self::UInt32),
            "Float32" => Ok(Self::Float32),
            "Int64" => Ok(Self::Int64),
            "UInt64" => Ok(Self::UInt64),
            "Float64" => Ok(Self::Float64),
            "Int128" => Ok(Self::Int128),
            "UInt128" => Ok(Self::UInt128),
            "Float128" => Ok(Self::Float128),
            _ => Err(format!("unsupported XISF type found: {}", s)),
        }
    }
}

/// Uncompress image data
fn xisf_uncompress_data(xisf_header: &XISFHeader, image_data: &[u8]) -> Box<[u8]> {
    info!("Read XISF > Uncompressing");
    let mut decompressed = Vec::new();
    let result;
    // Match compression codec and call decoder
    match xisf_header.compression_codec() {
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
                xisf_header.compression_codec()
            );
            process::exit(1);
        }
    }
    info!("Read XISF > Uncompressed size: {}", decompressed.len());
    match result {
        Ok(_v) => {
            // Data uncompressed
            // If expected size doesn't match, abort
            if decompressed.len() != xisf_header.compression_size {
                eprintln!(
                    "Read XISF > Uncompressing > Sizes don't match. Uncompressed: {} Expected: {}",
                    image_data.len(),
                    xisf_header.compression_size()
                );
                process::exit(1);
            }
        }
        Err(r) => {
            // Error uncompressing data
            eprintln!("Read XISF > Uncompressing > Cannot uncompress: {}", r);
            process::exit(1);
        }
    }
    // Unshuffle
    if xisf_header.sample_format().size() > 1 {
        info!(
            "Read XISF > Uncompressing > Unshuffling {}",
            xisf_header.compression_codec()
        );
        if xisf_header.compression_codec() == "zlib+sh" {
            decompressed =
                convert::unshuffle(&decompressed, xisf_header.sample_format().size() as usize);
            info!(
                "Read XISF > Uncompressing > Unshuffling > Decompressed len: {}",
                decompressed.len()
            );
        }
    }
    decompressed.into_boxed_slice()
}
