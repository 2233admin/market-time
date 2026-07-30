//! Explicit UTC instants used by the pure decision core.

/// An absolute UTC instant represented as nanoseconds since the Unix epoch.
///
/// Construction always requires a caller-supplied value. This type deliberately has no
/// `now()` constructor, so the core cannot read a clock through its public interface.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UtcInstant {
    nanos_since_unix_epoch: i128,
}

impl UtcInstant {
    /// Constructs an instant from a caller-supplied Unix-epoch nanosecond count.
    #[must_use]
    pub const fn from_nanos_since_unix_epoch(nanos: i128) -> Self {
        Self {
            nanos_since_unix_epoch: nanos,
        }
    }

    /// Returns the represented Unix-epoch nanosecond count without loss.
    #[must_use]
    pub const fn as_nanos_since_unix_epoch(self) -> i128 {
        self.nanos_since_unix_epoch
    }
}
