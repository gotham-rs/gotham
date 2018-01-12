use rand::{OsRng, Rng, SeedableRng};
use rand::reseeding::{Reseeder, ReseedingRng};
use rand::chacha::ChaChaRng;

pub struct OsRngReseeder {
    os_rng: OsRng,
}

impl Reseeder<ChaChaRng> for OsRngReseeder {
    fn reseed(&mut self, rng: &mut ChaChaRng) {
        let bytes: Vec<u32> = self.os_rng.gen_iter::<u32>().take(8).collect();
        rng.reseed(&bytes[..]);
    }
}

// A `ChaChaRng` which is periodically reseeded from an `OsRng`. This was originally using an
// `OsRng`, but sourcing entropy from the kernel was measured to be a performance bottleneck.
// Conventional wisdom seems to be that a securely seeded ChaCha20 PRNG is secure enough for
// cryptographic purposes, so it's certainly secure enough for generating unpredictable session
// identifiers.
pub type SessionIdentifierRng = ReseedingRng<ChaChaRng, OsRngReseeder>;

pub fn session_identifier_rng() -> SessionIdentifierRng {
    let os_rng = match OsRng::new() {
        Ok(rng) => rng,
        Err(e) => {
            error!(
                "Backend::random_identifier failed at rand::OsRng::new(), \
                 is the system RNG missing? {:?}",
                e
            );
            unreachable!("no rng available, this should never happen");
        }
    };

    let mut rng = ChaChaRng::new_unseeded();
    let mut reseeder = OsRngReseeder { os_rng };
    reseeder.reseed(&mut rng);

    // Reseed every 32KiB.
    ReseedingRng::new(rng, 32_768, reseeder)
}
