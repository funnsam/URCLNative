use std::{fs::*, env::*};

fn main() {
    let args = args().collect::<Vec<String>>();
    let file = read(&args[1]).unwrap();
    let remapped = file.chunks(2).map(|a| (a[0] as u32) << 8 | a[1] as u32).collect();
    write(&args[2], convert(&remapped)).unwrap();
}

pub fn convert(data: &Vec<u32>) -> Vec<u8> {
    let mut res = vec![0; data.len()<<2];
    for i in 0..data.len() {
        res[4*i..][..4].copy_from_slice(&data[i].to_be_bytes());
    }
    res
}
