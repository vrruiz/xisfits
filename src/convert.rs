use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[allow(dead_code)]
pub fn u8_to_i8(vector: &[u8]) -> Vec<i8> {
    let mut rdr = Cursor::new(vector);
    let mut values = Vec::new();

    loop {
        let option = rdr.read_i8();
        match option {
            Ok(n) => values.push(n),
            Err(_err) => break,
        }
    }
    values
}

macro_rules! u8_to_t {
    ($func_name:ident, $read_func:ident, $type:ty) => {
        #[allow(dead_code)]
        pub fn $func_name(vector: &[u8]) -> Vec<$type> {
            let mut rdr = Cursor::new(vector);
            let mut values = Vec::new();

            loop {
                let option = rdr.$read_func::<LittleEndian>();
                match option {
                    Ok(n) => values.push(n),
                    Err(_err) => break,
                }
            }
            values
        }
    };
}

u8_to_t!(u8_to_v_u16, read_u16, u16);
u8_to_t!(u8_to_v_i16, read_i16, i16);
u8_to_t!(u8_to_v_u32, read_u32, u32);
u8_to_t!(u8_to_v_i32, read_i32, i32);
u8_to_t!(u8_to_v_u64, read_u64, u64);
u8_to_t!(u8_to_v_i64, read_i64, i64);
u8_to_t!(u8_to_v_u128, read_u128, u128);
u8_to_t!(u8_to_v_i128, read_i128, i128);
u8_to_t!(u8_to_v_f32, read_f32, f32);
u8_to_t!(u8_to_v_f64, read_f64, f64);

macro_rules! t_to_u8_be {
    ($func_name:ident, $type:ty) => {
        #[allow(dead_code)]
        pub fn $func_name(vector: &[$type]) -> Vec<u8> {
            let mut values = Vec::new();
            for value in vector {
                let bytes = value.to_be_bytes();
                for n in 0..bytes.len() {
                    values.push(bytes[n]);
                }
            }
            values
        }
    };
}

t_to_u8_be!(i8_to_v_u8_be, i8);
t_to_u8_be!(u16_to_v_u8_be, u16);
t_to_u8_be!(i16_to_v_u8_be, i16);
t_to_u8_be!(u32_to_v_u8_be, u32);
t_to_u8_be!(i32_to_v_u8_be, i32);
t_to_u8_be!(u64_to_v_u8_be, u64);
t_to_u8_be!(i64_to_v_u8_be, i64);
t_to_u8_be!(i128_to_v_u8_be, i128);
t_to_u8_be!(u128_to_v_u8_be, u128);

/// From u16 to i16 to Vec<u8> (Big Endian)
#[allow(clippy::cast_possible_wrap)]
pub fn u16_to_i16_to_v_u8_be(v: &[u16]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    for integer in v {
        let mut v_u = *integer;
        if v_u > i16::max_value() as u16 {
            v_u = i16::max_value() as u16;
        }
        let v_i = v_u as i16;
        result.append(&mut v_i.to_be_bytes().to_vec());
    }
    result
}

/// From u32 to i32 to Vec<u8> (Big Endian)
#[allow(clippy::cast_possible_wrap)]
pub fn u32_to_i32_to_v_u8_be(v: &[u32]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    for integer in v {
        let mut v_u = *integer;
        if v_u > i32::max_value() as u32 {
            v_u = i32::max_value() as u32;
        }
        let v_i = v_u as i32;
        result.append(&mut v_i.to_be_bytes().to_vec());
    }
    result
}

/// From f32 to Vec<u8> (Big Endian)
pub fn f32_to_v_u8_be(v: &[f32]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    for float in v {
        let mut value = float.to_bits().to_be_bytes().to_vec();
        result.append(&mut value);
    }
    result
}

/// From f64 to Vec<u8> (Big Endian)
pub fn f64_to_v_u8_be(v: &[f64]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    for float in v {
        let mut value = float.to_bits().to_be_bytes().to_vec();
        result.append(&mut value);
    }
    result
}

/// Unshuffle byte array
pub fn unshuffle(array: &[u8], byte_size: usize) -> Vec<u8> {
    // Based on http://pixinsight.com/doc/docs/XISF-1.0-spec/XISF-1.0-spec.html#byte_shuffling
    let array_size = array.len();
    let mut unshuffled = Vec::with_capacity(array_size);
    unshuffled.resize(unshuffled.capacity(), 0_u8);
    let n_items = array_size / byte_size;
    for j in 0..(byte_size-1) {
        let array_start = j * n_items * byte_size;
        for i in 0..(n_items-1) {
            unshuffled[j + byte_size] = array[array_start + i];
        }
    }
    unshuffled
}
