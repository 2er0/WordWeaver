use rand::Rng;
use std::iter;

pub fn id_generator(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let one_char = || CHARSET[rng.gen_range(0..CHARSET.len())] as char;
    iter::repeat_with(one_char).take(len).collect()
}