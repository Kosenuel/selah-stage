//! Working out how a cue's channels reach the device's channels.

use crate::mixer::{MAX_DEVICE_CHANNELS, MAX_SOURCE_CHANNELS, Matrix};

/// Builds the matrix for a source with `source_channels` channels sent to the
/// given patches, each a list of device channels, at linear `level`.
///
/// - Mono goes to every channel of a patch.
/// - Stereo goes left to right across a stereo patch, and is summed into a
///   mono one at half level each.
/// - Device channels the device does not have are ignored.
pub fn build_matrix(
    source_channels: usize,
    patches: &[&[u16]],
    level: f32,
    device_channels: usize,
) -> Matrix {
    let mut matrix = [[0.0; MAX_DEVICE_CHANNELS]; MAX_SOURCE_CHANNELS];
    let usable = device_channels.min(MAX_DEVICE_CHANNELS);
    for outs in patches {
        let outs: Vec<usize> = outs
            .iter()
            .map(|c| usize::from(*c))
            .filter(|c| *c < usable)
            .collect();
        if source_channels == 1 {
            for out in &outs {
                matrix[0][*out] += level;
            }
        } else {
            match outs.as_slice() {
                [] => {}
                [only] => {
                    matrix[0][*only] += level * 0.5;
                    matrix[1][*only] += level * 0.5;
                }
                [left, right, ..] => {
                    matrix[0][*left] += level;
                    matrix[1][*right] += level;
                }
            }
        }
    }
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_to_a_stereo_patch_goes_left_to_right() {
        let m = build_matrix(2, &[&[2, 3]], 1.0, 8);
        assert_eq!(m[0][2], 1.0);
        assert_eq!(m[1][3], 1.0);
        assert_eq!(m[0][3], 0.0);
    }

    #[test]
    fn mono_goes_to_every_channel_of_the_patch() {
        let m = build_matrix(1, &[&[0, 1]], 0.5, 2);
        assert_eq!((m[0][0], m[0][1]), (0.5, 0.5));
    }

    #[test]
    fn stereo_into_a_mono_patch_is_summed_at_half_each() {
        let m = build_matrix(2, &[&[4]], 1.0, 8);
        assert_eq!((m[0][4], m[1][4]), (0.5, 0.5));
    }

    #[test]
    fn several_patches_add_and_missing_channels_are_ignored() {
        let m = build_matrix(2, &[&[0, 1], &[0, 1], &[6, 7]], 1.0, 2);
        assert_eq!(m[0][0], 2.0);
        assert_eq!(m[0][6], 0.0);
    }
}
