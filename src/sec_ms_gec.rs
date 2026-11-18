//! Edge's Sec-MS-GEC v2 token combines CS64 and a salted SHA-256 digest.
//!
//! Protocol reference: HarryXiaoCN's edge-tts implementation (LGPL-3.0).

use std::{fmt::Write, num::Wrapping as W};

use sha2::{compress256, digest::generic_array::GenericArray};

const SHA256_INITIAL: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];
// Fixed state and schedule contributions from the service's SHA-256 salt.
const SALT_STATE: [u32; 8] = [
    0xFD9FBEA2, 0x91243CE7, 0xBE499094, 0xCD2E1E0A, 0x6C006F49, 0x04BD35AB, 0xFEB0A7E4, 0x03EFFBD2,
];
const SALT_SCHEDULE: [u32; 14] = [
    0xEB24BF9E, 0x1558691F, 0xA0C09EE2, 0x0CFA32BA, 0xA622EE8C, 0xD1DD9491, 0xC0DD25A0, 0xF9E70AB6,
    0xF0C31573, 0x7058CE4E, 0x7188E606, 0x00E5AB33, 0xD258B060, 0x72766564,
];
// Standard SHA-256 round constants from round 14 onward, with round 14
// replaced by the service's adjusted constant.
const SALTED_ROUND_CONSTANTS: [u32; 50] = [
    0xC9DC06A7, 0xC19BF174, 0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC, 0x2DE92C6F, 0x4A7484AA,
    0x5CB0A9DC, 0x76F988DA, 0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7, 0xC6E00BF3, 0xD5A79147,
    0x06CA6351, 0x14292967, 0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13, 0x650A7354, 0x766A0ABB,
    0x81C2C92E, 0x92722C85, 0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3, 0xD192E819, 0xD6990624,
    0xF40E3585, 0x106AA070, 0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5, 0x391C0CB3, 0x4ED8AA4A,
    0x5B9CCA4F, 0x682E6FF3, 0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208, 0x90BEFFFA, 0xA4506CEB,
    0xBEF9A3F7, 0xC67178F2,
];
const CS64_SALT: [(u32, u32); 14] = [
    (0xE82E91B1, 0xB32700D0),
    (0x44DE24C6, 0x88DFF51D),
    (0x3BEB65BC, 0x29514038),
    (0xC89C5933, 0xB328CBDF),
    (0x8D1EC211, 0xD405B4D5),
    (0x314EF5D5, 0x26E55F3C),
    (0xE92491B1, 0x607801C6),
    (0xF0A5B9D5, 0x82B2C1FB),
    (0x06064CF3, 0xFA51A3DA),
    (0xDA79FA60, 0x207CC464),
    (0xBED20560, 0x3C3C74F3),
    (0x89F9C9F8, 0xD2D14B61),
    (0x35FA38C6, 0xBB04A2C1),
    (0xCB49E3C6, 0xC5626167),
];

fn salted_sha256_state(prefix: &[u8]) -> [u32; 8] {
    let mut schedule = [W(0u32); 64];
    schedule[14] = W(u32::from_be_bytes([0, prefix[0], prefix[1], prefix[2]]));
    schedule[15] = W(u32::from_be_bytes(prefix[3..7].try_into().unwrap()));

    for i in 16..64 {
        let x = schedule[i - 15].0;
        let y = schedule[i - 2].0;
        let mut sigma0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
        let mut sigma1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
        // Two schedule terms combine with the salt before modular addition.
        if i == 16 {
            sigma1 ^= 0x000B92C0;
        }
        if i == 29 {
            sigma0 ^= 0x059C0B80;
        }
        let salt = match i {
            16..=29 => SALT_SCHEDULE[i - 16],
            30 => 0xC9DC06A7u32.wrapping_sub(0x9BDC06A7),
            _ => 0,
        };
        schedule[i] = schedule[i - 16] + W(sigma0) + schedule[i - 7] + W(sigma1) + W(salt);
    }

    let mut working = SALT_STATE.map(W);
    for (&word, &constant) in schedule[14..].iter().zip(&SALTED_ROUND_CONSTANTS) {
        let [a, b, c, d, e, f, g, h] = working;
        let sum_e = e.0.rotate_right(6) ^ e.0.rotate_right(11) ^ e.0.rotate_right(25);
        let sum_a = a.0.rotate_right(2) ^ a.0.rotate_right(13) ^ a.0.rotate_right(22);
        let choice = (e & f) ^ (!e & g);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let t1 = h + W(sum_e) + choice + W(constant) + word;
        let t2 = W(sum_a) + majority;
        working = [t1 + t2, a, b, c, d + t1, e, f, g];
    }
    std::array::from_fn(|i| (W(SHA256_INITIAL[i]) + working[i]).0)
}

fn salted_sha256(input: &[u8]) -> [u8; 32] {
    // The salt occupies 57 bytes, so the first block consumes seven input
    // bytes. The remaining blocks are standard SHA-256, including padding.
    let mut state = salted_sha256_state(&input[..7]);
    let mut tail = input[7..].to_vec();
    tail.push(0x80);
    let zero_count = (64 - (tail.len() + 8) % 64) % 64;
    tail.resize(tail.len() + zero_count, 0);
    let bit_length = (57 + input.len() as u64) * 8;
    tail.extend_from_slice(&bit_length.to_be_bytes());

    for block in tail.chunks_exact(64) {
        compress256(&mut state, &[*GenericArray::from_slice(block)]);
    }
    let mut digest = [0; 32];
    for (bytes, word) in digest.chunks_exact_mut(4).zip(state) {
        bytes.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

fn rotate_half(word: W<u32>) -> W<u32> {
    W(word.0.rotate_left(16))
}

fn cs64(input: &[u8], digest: &[u8; 32]) -> [W<u32>; 2] {
    let keys = std::array::from_fn::<_, 2, _>(|i| {
        W(u32::from_le_bytes(digest[i * 4..i * 4 + 4].try_into().unwrap()) | 1)
    });
    let factors = keys.map(|key| key * W(0x82893933));
    let mut terms = Vec::from(CS64_SALT.map(|(coefficient, offset)| (W(coefficient), W(offset))));
    for (i, (term, _)) in terms.iter_mut().enumerate() {
        *term *= factors[i % 2];
    }

    let mut padded = vec![0];
    padded.extend_from_slice(input);
    padded.resize((padded.len() + 7) / 8 * 8, 0);
    for (i, bytes) in padded.chunks_exact(4).enumerate() {
        let word = W(u32::from_le_bytes(bytes.try_into().unwrap()));
        let term = word * keys[i % 2];
        let offset = rotate_half(word) * [W(0xEF0569FB), W(0xC31713DB)][i % 2];
        terms.push((term, offset));
    }
    terms[CS64_SALT.len()].0 += W(0xF2F90B1A) * factors[0];
    terms[CS64_SALT.len()].1 += W(0x0B1A0000);

    let [mut swap_total, mut swap_last, mut state, mut total] = [W(0); 4];
    for pair in terms.chunks_exact(2) {
        let [(x, x_offset), (y, y_offset)] = [pair[0], pair[1]];
        let q = rotate_half(x + x_offset) * W(0x76F50F34);
        swap_total += rotate_half(q) * W(0xC3EFEA97) + q;
        let q = rotate_half(y + y_offset) * W(0x1881CE3C);
        swap_last = rotate_half(q) * W(0x35BD1EC9) + q;
        swap_total += swap_last;

        state = state * keys[0] + x;
        for multiplier in [0xCF98B111, 0x87085B9F, 0x12CEB96D, 0x257E1D83] {
            state = rotate_half(state) * W(multiplier);
        }
        total += state;
        state = state * keys[1] + y;
        for multiplier in [0xA27416F5, 0xD38396FF, 0x7C932B89, 0xBFA49F69] {
            state = rotate_half(state) * W(multiplier);
        }
        total += state;
    }
    [swap_last ^ state, swap_total ^ total]
}

pub(super) fn digest(input: &[u8]) -> String {
    let sha = salted_sha256(input);
    let mut token = String::with_capacity(81);
    for word in cs64(input, &sha) {
        write!(token, "{:08X}", word.0.swap_bytes()).unwrap();
    }
    token.push('-');
    for byte in sha {
        write!(token, "{byte:02X}").unwrap();
    }
    token
}
