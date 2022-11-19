use std::cmp::min;

const ENCODE_MAP: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const DECODE_MAP: &[u8; 256] = &construct_decode_map();

const SIX_BIT_MASK: u128 = 0x3f;
const BYTE_MASK: u128 = 0xff;
const INVALID_BYTE: u8 = 0x40;

pub fn encode(bytes: &[u8]) -> String {
    let mut buffer = vec![0; ((bytes.len() / 3) + 1) * 4];
    let mut in_index = 0;
    let mut out_index = 0;

    while in_index < bytes.len().saturating_sub(16) {
        let in_u128 = read_u128(bytes, in_index);
        let chunk = &mut buffer[out_index..out_index + 16];

        for (i, item) in chunk.iter_mut().enumerate() {
            *item = encode_byte(((in_u128 >> (122 - i * 6)) & SIX_BIT_MASK as u128) as u8);
        }
        out_index += 16;
        in_index += 12;
    }

    let acc = read_u128_partial(bytes, in_index);
    let mut acc_bits = 8 * (bytes.len() - in_index);

    while acc_bits >= 6 {
        acc_bits -= 6;
        buffer[out_index] = encode_byte(((acc >> acc_bits) & SIX_BIT_MASK) as u8);
        out_index += 1;
    }

    if acc_bits > 0 {
        buffer[out_index] = encode_byte(((acc << (6 - acc_bits)) & SIX_BIT_MASK) as u8);
        out_index += 1;
    }

    while out_index % 4 > 0 {
        buffer[out_index] = b'=';
        out_index += 1;
    }

    buffer.truncate(out_index);

    // Buffer is built from UTF8 chars only. Safe to use and improves performance.
    unsafe { String::from_utf8_unchecked(buffer) }
}

const STEP: usize = 2;

pub fn decode(encoded: &str) -> Vec<u8> {
    let input = encoded.as_bytes();
    let mut buffer = vec![0; ((encoded.len() + 3) / 4) * 3];
    let mut in_index = 0;
    let mut out_index = 0;

    while in_index < input.len().saturating_sub(STEP * 4) {
        let in_chunk = &input[in_index..in_index + (STEP * 4)];
        let out_chunk = &mut buffer[out_index..out_index + (STEP * 3)];
        let mut in_u64 = 0u64;

        for (i, in_byte) in in_chunk.iter().enumerate() {
            in_u64 |= (decode_byte(*in_byte) as u64) << (44 - i * 6) as u64;
        }
        for (i, out_byte) in out_chunk.iter_mut().enumerate() {
            *out_byte = ((in_u64 >> ((STEP * 4 * 6 - 6) - (i * 8))) & BYTE_MASK as u64) as u8;
        }
        out_index += STEP * 3;
        in_index += STEP * 4;
    }

    let mut acc = 0u64;
    let mut acc_bits = 0u8;

    while in_index < input.len() {
        if input[in_index] == b'=' {
            break;
        }
        acc = (acc << 6) + decode_byte(input[in_index]) as u64;
        acc_bits += 6;
        in_index += 1;
    }

    while acc_bits >= 8 {
        acc_bits -= 8;
        buffer[out_index] = ((acc >> acc_bits) & BYTE_MASK as u64) as u8;
        out_index += 1;
    }

    buffer.truncate(out_index);
    buffer
}

#[inline(always)]
fn encode_byte(byte: u8) -> u8 {
    ENCODE_MAP[byte as usize]
}

#[inline(always)]
fn decode_byte(byte: u8) -> u8 {
    let decoded = DECODE_MAP[byte as usize];

    if decoded == INVALID_BYTE {
        panic!("Unable to decode non-base64 character '{}'", byte as char)
    }
    decoded
}

#[inline(always)]
fn read_u128(bytes: &[u8], from: usize) -> u128 {
    u128::from_be_bytes(bytes[from..from + 16].try_into().unwrap())
}

#[inline(always)]
fn read_u128_partial(bytes: &[u8], from: usize) -> u128 {
    let size = min(bytes.len() - from, 16);
    let mut buffer = [0u8; 16];

    buffer[16 - size..].copy_from_slice(&bytes[from..from + size]);

    u128::from_be_bytes(buffer)
}

const fn construct_decode_map() -> [u8; 256] {
    let mut map = [INVALID_BYTE; 256];
    let mut index = 0;

    while index < 64 {
        map[ENCODE_MAP[index] as usize] = index as u8;
        index += 1;
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn should_encode_following_base64_spec() {
        assert_eq!(encode(b"Hello!"), "SGVsbG8h");
        assert_eq!(encode(b"0123456789"), "MDEyMzQ1Njc4OQ==");
        assert_eq!(
            encode(b"https://foo.bar/q?a=2&b=3#fr"),
            "aHR0cHM6Ly9mb28uYmFyL3E/YT0yJmI9MyNmcg=="
        );
        assert_eq!(encode(b"  "), "ICA=");
        assert_eq!(encode(b""), "");
        assert_eq!(encode(&0u32.to_ne_bytes()), "AAAAAA==");
    }

    #[test]
    fn should_decode_following_base64_spec() {
        assert_eq!(decode("SGVsbG8h"), b"Hello!");
        assert_eq!(decode("MDEyMzQ1Njc4OQ=="), b"0123456789");
        assert_eq!(
            decode("aHR0cHM6Ly9mb28uYmFyL3E/YT0yJmI9MyNmcg=="),
            b"https://foo.bar/q?a=2&b=3#fr"
        );
        assert_eq!(decode("ICA="), b"  ");
        assert_eq!(decode(""), b"");
        assert_eq!(decode("AAAAAA=="), 0u32.to_ne_bytes())
    }

    #[test]
    fn should_preserve_original_input() {
        for size in 0..512 {
            let bytes = random_bytes(size);
            assert_eq!(decode(&encode(&bytes)), bytes);
        }
        let large = random_bytes(3 * 1024 * 1024);
        assert_eq!(decode(&encode(&large)), large);
    }

    #[test]
    fn should_construct_matching_encode_decode_tables() {
        for byte in 0..64 {
            assert_eq!(
                construct_decode_map()[ENCODE_MAP[byte] as usize],
                byte as u8
            );
        }
    }

    #[test]
    #[should_panic(expected = "Unable to decode non-base64 character '^'")]
    fn should_panic_when_decode_non_base64_input() {
        decode("AAA^AAA==");
    }

    fn random_bytes(size: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(size);
        let mut r = SmallRng::from_entropy();
        while bytes.len() < size {
            bytes.push(r.gen::<u8>());
        }
        bytes
    }
}
