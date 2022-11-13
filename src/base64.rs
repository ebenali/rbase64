
pub fn encode(value: &[u8]) -> String {
    let mut encoded = String::new();
    let mut accumulator_value = 0u8;
    let mut accumulator_bits = 0u8;

    for byte in value {
        let mut mask = 128u8;

        while mask > 0 {
            let bit = if byte & mask > 0 { 1 } else { 0 };
            accumulator_value = (accumulator_value << 1) + bit;
            accumulator_bits += 1;

            if accumulator_bits >= 6 {
                encoded.push(to_base64_char(accumulator_value));
                accumulator_value = 0;
                accumulator_bits = 0;
            }
            mask = mask >> 1;
        }
    }

    if accumulator_bits != 0 {
        accumulator_value = accumulator_value << (6 - accumulator_bits);
        encoded.push(to_base64_char(accumulator_value));
    }

    while encoded.len() % 4 > 0 {
        encoded.push('=');
    }
    return encoded;
}

pub fn decode(encoded: &str) -> Vec<u8> {
    let mut decoded: Vec<u8> = Vec::new();
    let mut accumulator_value = 0u8;
    let mut accumulator_bits = 0u8;

    for b64_char in encoded.chars() {
        if b64_char == '=' {
            break;
        }

        let value = to_byte(b64_char);
        let mut mask = 32u8;

        while mask > 0 {
            let bit = if value & mask > 0 { 1 } else { 0 };
            accumulator_value = (accumulator_value << 1) + bit;
            accumulator_bits += 1;

            if accumulator_bits >= 8 {
                decoded.push(accumulator_value);
                accumulator_value = 0;
                accumulator_bits = 0;
            }
            mask = mask >> 1;
        }
    }
    return decoded;
}

fn to_base64_char(value: u8) -> char {
    return if value < 26 {
        char::from(('A' as u8) + value)
    } else if value < 52 {
        char::from(('a' as u8) + (value - 26))
    } else if value < 62 {
        char::from(('0' as u8) + (value - 52))
    } else if value < 63 {
        '+'
    } else if value < 64 {
        '/'
    } else {
        panic!("Input byte is not in base 64 ({})", value)
    };
}

fn to_byte(base64: char) -> u8 {
    return if base64 >= 'A' && base64 <= 'Z' {
        (base64 as u8) - ('A' as u8)
    } else if base64 >= 'a' && base64 <= 'z' {
        (base64 as u8) - ('a' as u8) + 26
    } else if base64 >= '0' && base64 <= '9' {
        (base64 as u8) - ('0' as u8) + 52
    } else if base64 == '+' {
        62
    } else if base64 == '/' {
        63
    } else {
        panic!("Character '{}' is not part of the base64 spec ([a-z][A-Z][0-9]+/=)", base64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_encode_following_base64_spec() {
        assert_eq!(encode(b"Hello!"), "SGVsbG8h");
        assert_eq!(encode(b"0123456789"), "MDEyMzQ1Njc4OQ==");
        assert_eq!(encode(b"https://foo.bar/q?a=2&b=3#fr"), "aHR0cHM6Ly9mb28uYmFyL3E/YT0yJmI9MyNmcg==");
        assert_eq!(encode(b"  "), "ICA=");
        assert_eq!(encode(b""), "");
    }

    #[test]
    fn should_decode_following_base64_spec() {
        assert_eq!(decode("SGVsbG8h"), b"Hello!");
        assert_eq!(decode("MDEyMzQ1Njc4OQ=="), b"0123456789");
        assert_eq!(decode("aHR0cHM6Ly9mb28uYmFyL3E/YT0yJmI9MyNmcg=="), b"https://foo.bar/q?a=2&b=3#fr");
        assert_eq!(decode("ICA="), b"  ");
        assert_eq!(decode(""), b"");
    }

    #[test]
    fn should_preserve_original_value() {
        assert_eq!(decode(&encode(b"Hello!")), b"Hello!");
        assert_eq!(decode(&encode(b"Large msg...")), b"Large msg...");
        assert_eq!(decode(&encode(b"!@#$%^&*()_+")), b"!@#$%^&*()_+")
    }

}