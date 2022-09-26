use crate::st::error::Error;

/// translated from main/lib/protocol/luhn.go
pub fn luhn32(string: &str) -> Result<char, Error> {
    let mut factor = 1;
    let mut sum = 0;
    const N: u32 = 32;
    const LUHN_BASE_32: [char; 32] = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V',
        'W', 'X', 'Y', 'Z', '2', '3', '4', '5', '6', '7',
    ];

    for char in string.chars() {
        let codepoint = codepoint(&char)?;
        let addend = factor * codepoint;
        factor = if factor == 2 { 1 } else { 2 };
        sum += (addend / N) + (addend % N)
    }
    let remainder = sum % N;
    let check_code_point = (N - remainder) % N;
    Ok(LUHN_BASE_32[check_code_point as usize])
}

/// translated from main/lib/protocol/luhn.go
pub fn codepoint(c: &char) -> Result<u32, Error> {
    match *c {
        char if ('A'..='Z').contains(&char) => Ok(char as u32 - 'A' as u32),
        char if ('2'..='7').contains(&char) => Ok(char as u32 + 26 - '2' as u32),
        _ => Err(Error::Codepoint),
    }
}
