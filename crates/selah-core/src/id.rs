//! Stable identifiers. Ids are UUID v7 and never change once assigned.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// A new id. Ids sort by creation time.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

id_type!(
    /// Identifies a cue.
    CueId
);
id_type!(
    /// Identifies an item in the service plan.
    ItemId
);
id_type!(
    /// Identifies an audio patch.
    PatchId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_serialise_as_plain_strings() {
        let (a, b) = (CueId::new(), CueId::new());
        assert_ne!(a, b);
        let json = serde_json::to_value(a).unwrap();
        assert!(json.is_string());
        assert_eq!(serde_json::from_value::<CueId>(json).unwrap(), a);
    }
}
