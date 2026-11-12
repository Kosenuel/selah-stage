//! Bringing older show files up to the current format.
//!
//! The file is handled as plain JSON during migration, so old formats never
//! need their Rust types kept around. `MIGRATIONS[n]` upgrades format `n + 1`
//! to `n + 2`. Add one entry whenever `CURRENT_SCHEMA_VERSION` goes up, and a
//! fixture of the old version under `fixtures/shows/`.

use selah_core::CURRENT_SCHEMA_VERSION;
use serde_json::Value;

use crate::error::PersistError;

/// One upgrade step.
pub type Migration = fn(Value) -> Value;

/// Format 1 is the first, so there is nothing to migrate from yet.
pub const MIGRATIONS: &[Migration] = &[];

/// Upgrades `value` to the current format.
pub fn to_current(value: Value) -> Result<Value, PersistError> {
    upgrade(value, MIGRATIONS, CURRENT_SCHEMA_VERSION)
}

pub(crate) fn upgrade(
    mut value: Value,
    migrations: &[Migration],
    target: u32,
) -> Result<Value, PersistError> {
    let found = version_of(&value).ok_or_else(|| PersistError::Corrupt("show file".into()))?;
    if found > target {
        return Err(PersistError::TooNew {
            found,
            supported: target,
        });
    }
    for step in found..target {
        let Some(migration) = migrations.get(step as usize - 1) else {
            return Err(PersistError::Corrupt("show file".into()));
        };
        value = migration(value);
        if let Some(object) = value.as_object_mut() {
            object.insert("schemaVersion".into(), Value::from(step + 1));
        }
    }
    Ok(value)
}

fn version_of(value: &Value) -> Option<u32> {
    let version = value.get("schemaVersion")?.as_u64()?;
    u32::try_from(version).ok().filter(|v| *v >= 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn add_notes(mut v: Value) -> Value {
        v["notes"] = json!("added in v2");
        v
    }

    fn rename_title(mut v: Value) -> Value {
        v["name"] = v["title"].take();
        v
    }

    #[test]
    fn runs_each_step_in_order_and_stamps_the_version() {
        let old = json!({ "schemaVersion": 1, "title": "Sunday" });
        let out = upgrade(old, &[add_notes, rename_title], 3).unwrap();
        assert_eq!(out["schemaVersion"], 3);
        assert_eq!(out["notes"], "added in v2");
        assert_eq!(out["name"], "Sunday");
    }

    #[test]
    fn a_current_file_is_left_alone() {
        let v = json!({ "schemaVersion": 2, "x": 1 });
        assert_eq!(upgrade(v.clone(), &[add_notes], 2).unwrap(), v);
    }

    #[test]
    fn a_newer_file_is_refused_with_a_clear_error() {
        let v = json!({ "schemaVersion": 9 });
        assert_eq!(
            upgrade(v, &[], 2),
            Err(PersistError::TooNew {
                found: 9,
                supported: 2
            })
        );
    }

    #[test]
    fn a_file_with_no_version_is_corrupt() {
        assert!(matches!(
            upgrade(json!({ "title": "x" }), &[], 1),
            Err(PersistError::Corrupt(_))
        ));
    }
}
