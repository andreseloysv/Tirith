//! An RSA public key as DER: what `ring` verifies with, from a JWK's modulus and exponent.

fn length(n: usize, out: &mut Vec<u8>) {
    match n {
        0..=127 => out.push(n as u8),
        128..=255 => out.extend([0x81, n as u8]),
        _ => out.extend([0x82, (n >> 8) as u8, n as u8]),
    }
}

fn integer(big_endian: &[u8], out: &mut Vec<u8>) {
    let digits = &big_endian[big_endian.iter().take_while(|b| **b == 0).count()..];
    let pad = digits.first().is_some_and(|b| b & 0x80 != 0);
    out.push(0x02);
    length(digits.len() + usize::from(pad), out);
    out.extend(pad.then_some(0));
    out.extend(digits);
}

/// `RSAPublicKey ::= SEQUENCE { modulus INTEGER, publicExponent INTEGER }`.
pub fn public_key(n: &[u8], e: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    integer(n, &mut body);
    integer(e, &mut body);
    let mut key = vec![0x30];
    length(body.len(), &mut key);
    key.extend(body);
    key
}
