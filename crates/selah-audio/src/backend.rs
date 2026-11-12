//! Where the mixer's output goes: a buffer for tests, or a real device.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, ErrorKind, SampleFormat, StreamConfig};

use crate::mixer::Mixer;

/// Renders into memory. Used by tests, so none needs a real device.
pub struct OfflineBackend {
    mixer: Mixer,
    channels: usize,
}

impl OfflineBackend {
    pub fn new(mixer: Mixer, channels: usize) -> Self {
        Self { mixer, channels }
    }

    /// The next `frames` frames, interleaved.
    pub fn render(&mut self, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * self.channels];
        self.mixer.render(&mut out, self.channels);
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeviceError {
    #[error("No audio output device was found. Check the cable or choose another in settings.")]
    NoDevice,
    #[error("The audio device does not support the sample format Selah Stage needs.")]
    UnsupportedFormat,
    #[error("The audio device could not be started: {0}")]
    Start(String),
}

/// An output device chosen but not yet playing.
pub struct OutputDevice {
    device: cpal::Device,
    config: StreamConfig,
    name: String,
}

impl OutputDevice {
    /// The system's default output, optionally with a fixed buffer size in
    /// frames (smaller is lower latency but easier to underrun).
    pub fn default_output(buffer_frames: Option<u32>) -> Result<Self, DeviceError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(DeviceError::NoDevice)?;
        let supported = device
            .default_output_config()
            .map_err(|e| DeviceError::Start(e.to_string()))?;
        if supported.sample_format() != SampleFormat::F32 {
            return Err(DeviceError::UnsupportedFormat);
        }
        let mut config = supported.config();
        if let Some(frames) = buffer_frames {
            config.buffer_size = BufferSize::Fixed(frames);
        }
        let name = device
            .description()
            .map_or_else(|_| "Audio output".to_owned(), |d| d.name().to_owned());
        Ok(Self {
            device,
            config,
            name,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    pub fn channels(&self) -> usize {
        usize::from(self.config.channels)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Starts playing. Keep the result alive for as long as sound should play,
    /// and on the thread that created it (some platforms require this).
    pub fn start(self, mut mixer: Mixer) -> Result<RunningDevice, DeviceError> {
        let channels = self.channels();
        let lost = Arc::new(AtomicBool::new(false));
        let error_flag = lost.clone();
        let stream = self
            .device
            .build_output_stream(
                self.config,
                move |data: &mut [f32], _| mixer.render(data, channels),
                move |error| {
                    if matches!(
                        error.kind(),
                        ErrorKind::DeviceNotAvailable | ErrorKind::DeviceChanged
                    ) {
                        error_flag.store(true, Ordering::Release);
                    }
                },
                None,
            )
            .map_err(|e| DeviceError::Start(e.to_string()))?;
        stream
            .play()
            .map_err(|e| DeviceError::Start(e.to_string()))?;
        Ok(RunningDevice {
            _stream: stream,
            lost,
        })
    }
}

/// A device that is playing. Dropping it stops the sound.
pub struct RunningDevice {
    _stream: cpal::Stream,
    lost: Arc<AtomicBool>,
}

impl RunningDevice {
    /// True once the device has gone away. The host should warn the operator
    /// and, when a device returns, call `AudioEngine::reattach` and start again.
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
}
