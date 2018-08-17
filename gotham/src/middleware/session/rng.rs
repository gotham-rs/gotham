use rand::prng::chacha::ChaChaCore;
use rand::rngs::adapter::ReseedingRng;
use rand::rngs::OsRng;
use rand::FromEntropy;

// A `ChaChaRng` which is periodically reseeded from an `OsRng`. This was originally using an
// `OsRng`, but sourcing entropy from the kernel was measured to be a performance bottleneck.
// Conventional wisdom seems to be that a securely seeded ChaCha20 PRNG is secure enough for
// cryptographic purposes, so it's certainly secure enough for generating unpredictable session
// identifiers.
pub(super) type SessionIdentifierRng = ReseedingRng<ChaChaCore, OsRng>;

pub(super) fn session_identifier_rng() -> SessionIdentifierRng {
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

    let rng = ChaChaCore::from_entropy();

    // Reseed every 32KiB.
    ReseedingRng::new(rng, 32_768, os_rng)
}
