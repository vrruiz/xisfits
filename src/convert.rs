use std::io::Cursor;
use byteorder::LittleEndian;
use byteorder::ReadBytesExt;

/*
pub fn u8_4_to_u32(a: &[u8;4]) -> u32 {
    // Little endian conversion
    let mut rdr = Cursor::new(a);
    let option = rdr.read_u32::<LittleEndian>();
    let mut r: u32 = 0;
    match option {
        Ok(n) => r = n,
        Err(_err) => eprintln!("Error converting {:?} to u32", a),
    }
    return r;
}
*/

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

u8_to_t!(u8_to_u16, read_u16, u16);
u8_to_t!(u8_to_i16, read_i16, i16);
u8_to_t!(u8_to_u32, read_u32, u32);
u8_to_t!(u8_to_i32, read_i32, i32);
u8_to_t!(u8_to_u64, read_u64, u64);
u8_to_t!(u8_to_i64, read_i64, i64);
u8_to_t!(u8_to_u128, read_u128, u128);
u8_to_t!(u8_to_i128, read_i128, i128);
u8_to_t!(u8_to_f32, read_f32, f32);
u8_to_t!(u8_to_f64, read_f64, f64);

macro_rules! t_to_u8_be {
    ($func_name:ident, $type:ty) => {
        pub fn $func_name(vector: &Vec<$type>) -> Vec<u8> {
            let mut values = Vec::new();
            for i in 0..vector.len() {
                let bytes = vector[i].to_be_bytes();
                for n in 0..bytes.len() {
                    values.push(bytes[n]);
                }
            }
            values
        }
    };
}

t_to_u8_be!(i8_to_u8_be, i8);
t_to_u8_be!(u16_to_u8_be, u16);
t_to_u8_be!(i16_to_u8_be, i16);
t_to_u8_be!(u32_to_u8_be, u32);
t_to_u8_be!(i32_to_u8_be, i32);
t_to_u8_be!(u64_to_u8_be, u64);
t_to_u8_be!(i64_to_u8_be, i64);
t_to_u8_be!(i128_to_u8_be, i128);
t_to_u8_be!(u128_to_u8_be, u128);

// From u16 to i16 to Vec<u8> (Big Endian)
pub fn u16_to_i16_to_u8_be(v: &Vec<u16>) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    for i in 0..v.len() {
        let mut v_u = v[i];
        if v_u > i16::max_value() as u16 {
            v_u = i16::max_value() as u16;
        }
        let v_i = v_u as i16;
        result.append(&mut v_i.to_be_bytes().to_vec());
    }
    result
}

// From u32 to i32 to Vec<u8> (Big Endian)
pub fn u32_to_i32_to_u8_be(v: &Vec<u32>) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    for i in 0..v.len() {
        let mut v_u = v[i];
        if v_u > i32::max_value() as u32 {
            v_u = i32::max_value() as u32;
        }
        let v_i = v_u as i32;
        result.append(&mut v_i.to_be_bytes().to_vec());
    }
    result
}
