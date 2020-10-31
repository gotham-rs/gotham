use rand::rngs::adapter::ReseedingRng;
use rand::rngs::OsRng;
use rand::SeedableRng;
use rand_chacha::ChaChaCore;

// A `ChaChaRng` which is periodically reseeded from an `OsRng`. This was originally using an
// `OsRng`, but sourcing entropy from the kernel was measured to be a performance bottleneck.
// Conventional wisdom seems to be that a securely seeded ChaCha20 PRNG is secure enough for
// cryptographic purposes, so it's certainly secure enough for generating unpredictable session
// identifiers.
pub(super) type SessionIdentifierRng = ReseedingRng<ChaChaCore, OsRng>;

pub(super) fn session_identifier_rng() -> SessionIdentifierRng {
    let os_rng = OsRng::default();
    let rng = ChaChaCore::from_entropy();

    // Reseed every 32KiB.
    ReseedingRng::new(rng, 32_768, os_rng)
}
