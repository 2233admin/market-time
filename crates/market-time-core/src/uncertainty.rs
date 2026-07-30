//! Uncertainty attached to published boundaries and answers.

/// Whether a boundary is a state-change instant or the start of a venue process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryCharacter {
    /// The source describes a state change at the boundary.
    Instantaneous,
    /// The source describes the scheduled start of a process whose completion is not a
    /// single instant.
    ProcessStart,
}

/// What is known about the precision and accuracy of a published boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Uncertainty {
    /// Publication quantum in nanoseconds, when the source publishes one.
    pub granularity_ns: Option<u64>,
    /// Symmetric venue-published bound in nanoseconds, when present.
    pub published_bound_ns: Option<u64>,
    /// The semantic character of the boundary.
    pub boundary_character: BoundaryCharacter,
    /// Venue-published process spread in nanoseconds, when present.
    pub process_spread_ns: Option<u64>,
    /// Whether the boundary was derived rather than directly observed.
    pub is_derived: bool,
}

impl Uncertainty {
    /// Expresses exactness only to the representation, with no accuracy claim beyond it.
    #[must_use]
    pub const fn exact() -> Self {
        Self {
            granularity_ns: None,
            published_bound_ns: None,
            boundary_character: BoundaryCharacter::Instantaneous,
            process_spread_ns: None,
            is_derived: false,
        }
    }

    /// Conservatively combines two uncertainty statements without narrowing either.
    #[must_use]
    pub fn combine(left: &Self, right: &Self) -> Self {
        Self {
            granularity_ns: max_optional(left.granularity_ns, right.granularity_ns),
            published_bound_ns: max_optional(left.published_bound_ns, right.published_bound_ns),
            boundary_character: if left.boundary_character == BoundaryCharacter::ProcessStart
                || right.boundary_character == BoundaryCharacter::ProcessStart
            {
                BoundaryCharacter::ProcessStart
            } else {
                BoundaryCharacter::Instantaneous
            },
            process_spread_ns: max_optional(left.process_spread_ns, right.process_spread_ns),
            is_derived: left.is_derived || right.is_derived,
        }
    }
}

const fn max_optional(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(if left > right { left } else { right }),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}
