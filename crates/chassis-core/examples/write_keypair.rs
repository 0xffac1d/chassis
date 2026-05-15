//! Write hex-encoded ed25519 signing key (first line) and public key (second line) to two paths.

use std::env;
use std::fs;

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

fn main() {
    let mut it = env::args().skip(1);
    let sk_path = it.next().expect("priv path");
    let pk_path = it.next().expect("pub path");
    let mut rng = OsRng;
    let sk = SigningKey::generate(&mut rng);
    let vk = sk.verifying_key();
    fs::write(&sk_path, format_priv(&sk)).expect("write priv");
    fs::write(&pk_path, to_hex(&vk.to_bytes())).expect("write pub");
}

fn format_priv(sk: &SigningKey) -> String {
    to_hex(sk.as_bytes())
}

fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}
