# XISFITS

XISFITS is a tool to convert PixInsight images ([XISF](http://pixinsight.com/doc/docs/XISF-1.0-spec/XISF-1.0-spec.html)) to [FITS](https://fits.gsfc.nasa.gov/fits_standard.html).

## Installation

This is programmed in [Rust](http://rust-lang.org/) language. Rust development tools must be installed following its own [instructions](https://www.rust-lang.org/tools/install). After that, download XISFITS sources and, on the main directory, run:

```bash
$ cargo build xisfits
```

## Usage

```bash
$ xisfits <image.xisf> <image.fits>
```

## Known issues and limitations

- Although the XISF format supports signed integers, currently only UInt8, UInt16 and UInt32 types can be converted to FITS.
- UInt8 is converted to FITS BITPIX 8, which is also unsigned.
- UInt16 and UInt32 are converted to signed 16 and 32 bits. In order to do so, if there are unsigned values greater than what signed values can store, those values are clipped.
- Float32 and Float64 conversions are still unsupported.
- If XISF file stores FITS headers, those headers are not copied over to the final FITS image.

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## License
[MIT](https://choosealicense.com/licenses/mit/)