//! Cross-platform sound playback.
//!
//! The web build and the macOS/Windows builds use Macroquad's audio backend.
//! Native Linux builds use a small PulseAudio backend that is loaded at
//! runtime, so building the game never requires ALSA development packages
//! (the ALSA device is usually owned by PulseAudio or PipeWire anyway).

/// Folder that holds the game assets, both on disk and in the browser.
pub const ASSETS_FOLDER: &str = "assets";

#[cfg(any(target_arch = "wasm32", target_os = "macos", target_os = "windows"))]
pub use backend_macroquad::{AudioPlayer, Sound};

#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
pub use backend_linux::{AudioPlayer, Sound};

#[cfg(any(target_arch = "wasm32", target_os = "macos", target_os = "windows"))]
mod backend_macroquad {
    use macroquad::audio::{self, PlaySoundParams, Sound as MacroquadSound};

    /// A decoded sound effect.
    pub struct Sound(MacroquadSound);

    /// Plays sounds through Macroquad's audio backend.
    pub struct AudioPlayer;

    impl AudioPlayer {
        pub fn new() -> Self {
            AudioPlayer
        }

        pub async fn load(&self, path: &str) -> Option<Sound> {
            match audio::load_sound(path).await {
                Ok(sound) => Some(Sound(sound)),
                Err(error) => {
                    eprintln!("Could not load sound {path}: {error}");
                    None
                }
            }
        }

        pub fn play(&self, sound: &Sound, volume: f32) {
            audio::play_sound(
                &sound.0,
                PlaySoundParams {
                    looped: false,
                    volume,
                },
            );
        }
    }
}

#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
mod backend_linux {
    use std::ffi::{CString, c_char, c_int, c_void};
    use std::path::{Path, PathBuf};
    use std::process::Stdio;
    use std::sync::Arc;

    use hound::{SampleFormat, WavReader};

    use super::ASSETS_FOLDER;

    const PA_STREAM_PLAYBACK: c_int = 1;
    const PA_SAMPLE_FLOAT32LE: i32 = 5;

    /// A decoded sound effect, kept as mono floating point samples.
    pub struct Sound {
        samples: Arc<Vec<f32>>,
        rate: u32,
        path: PathBuf,
    }

    enum Backend {
        Pulse(PulseApi),
        Command(String),
        Silent,
    }

    /// Plays sounds through PulseAudio, with a command line fallback.
    pub struct AudioPlayer {
        backend: Backend,
        fallback: Option<String>,
    }

    impl AudioPlayer {
        pub fn new() -> Self {
            let pulse = PulseApi::load();
            let fallback = find_command(&["paplay", "pw-play", "aplay"]);

            let backend = match (&pulse, &fallback) {
                (Some(api), _) => Backend::Pulse(api.clone()),
                (None, Some(command)) => Backend::Command(command.clone()),
                (None, None) => {
                    eprintln!(
                        "No audio backend found. Install PulseAudio (libpulse-simple) to hear \
                         sound effects."
                    );
                    Backend::Silent
                }
            };

            AudioPlayer { backend, fallback }
        }

        pub async fn load(&self, path: &str) -> Option<Sound> {
            let file_path = resolve_path(path);
            let (samples, rate) = decode_wav(&file_path)?;

            Some(Sound {
                samples: Arc::new(samples),
                rate,
                path: file_path,
            })
        }

        pub fn play(&self, sound: &Sound, volume: f32) {
            match &self.backend {
                Backend::Pulse(api) => {
                    let api = api.clone();
                    let samples = Arc::clone(&sound.samples);
                    let rate = sound.rate;
                    let fallback = self.fallback.clone();
                    let path = sound.path.clone();

                    std::thread::spawn(move || {
                        if let Err(message) = api.play(&samples, rate, volume) {
                            warn_once(&format!("PulseAudio playback failed: {message}"));

                            if let Some(command) = fallback {
                                play_with_command(&command, &path);
                            }
                        }
                    });
                }
                Backend::Command(command) => play_with_command(command, &sound.path),
                Backend::Silent => {}
            }
        }
    }

    static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    fn warn_once(message: &str) {
        if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("{message}");
        }
    }

    fn play_with_command(command: &str, path: &Path) {
        let command = command.to_owned();
        let path = path.to_path_buf();

        std::thread::spawn(move || {
            let _ = std::process::Command::new(&command)
                .arg(&path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        });
    }

    fn resolve_path(path: &str) -> PathBuf {
        let with_folder = Path::new(ASSETS_FOLDER).join(path);
        if with_folder.exists() {
            with_folder
        } else {
            PathBuf::from(path)
        }
    }

    fn decode_wav(path: &Path) -> Option<(Vec<f32>, u32)> {
        let mut reader = WavReader::open(path).ok()?;
        let spec = reader.spec();
        let channels = spec.channels.max(1) as usize;

        let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Int, 8) => reader
                .samples::<i8>()
                .map(|sample| sample.ok().map(|value| value as f32 / 128.0))
                .collect::<Option<Vec<_>>>()?,
            (SampleFormat::Int, 16) => reader
                .samples::<i16>()
                .map(|sample| sample.ok().map(|value| value as f32 / 32_768.0))
                .collect::<Option<Vec<_>>>()?,
            (SampleFormat::Int, 32) => reader
                .samples::<i32>()
                .map(|sample| sample.ok().map(|value| value as f32 / 2_147_483_648.0))
                .collect::<Option<Vec<_>>>()?,
            (SampleFormat::Float, _) => reader
                .samples::<f32>()
                .map(|sample| sample.ok())
                .collect::<Option<Vec<_>>>()?,
            _ => return None,
        };

        if channels == 1 {
            return Some((samples, spec.sample_rate));
        }

        let mono = samples
            .chunks(channels)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect();

        Some((mono, spec.sample_rate))
    }

    fn find_command(candidates: &[&str]) -> Option<String> {
        let path = std::env::var_os("PATH")?;

        for candidate in candidates {
            for directory in std::env::split_paths(&path) {
                let executable = directory.join(candidate);
                if executable.is_file() {
                    return Some(executable.to_string_lossy().into_owned());
                }
            }
        }

        None
    }

    #[repr(C)]
    struct PaSampleSpec {
        format: i32,
        rate: u32,
        channels: u8,
    }

    type SimpleNew = unsafe extern "C" fn(
        *const c_char,
        *const c_char,
        c_int,
        *const c_char,
        *const c_char,
        *const PaSampleSpec,
        *const c_void,
        *const c_void,
        *mut c_int,
    ) -> *mut c_void;
    type SimpleWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize, *mut c_int) -> c_int;
    type SimpleDrain = unsafe extern "C" fn(*mut c_void, *mut c_int) -> c_int;
    type SimpleFree = unsafe extern "C" fn(*mut c_void);
    type StrError = unsafe extern "C" fn(c_int) -> *const c_char;

    /// Minimal binding to `libpulse-simple`, loaded at runtime.
    #[derive(Clone)]
    struct PulseApi {
        _library: Arc<libloading::Library>,
        _pulse_library: Option<Arc<libloading::Library>>,
        new: SimpleNew,
        write: SimpleWrite,
        drain: SimpleDrain,
        free: SimpleFree,
        strerror: Option<StrError>,
    }

    impl PulseApi {
        fn load() -> Option<Self> {
            // SAFETY: the library is loaded with the documented symbol names and
            // the function signatures match `pulse/simple.h`.
            unsafe {
                let library = libloading::Library::new("libpulse-simple.so.0").ok()?;
                let new = *library.get::<SimpleNew>(b"pa_simple_new\0").ok()?;
                let write = *library.get::<SimpleWrite>(b"pa_simple_write\0").ok()?;
                let drain = *library.get::<SimpleDrain>(b"pa_simple_drain\0").ok()?;
                let free = *library.get::<SimpleFree>(b"pa_simple_free\0").ok()?;

                let pulse_library = libloading::Library::new("libpulse.so.0").ok();
                let strerror = pulse_library
                    .as_ref()
                    .and_then(|pulse| pulse.get::<StrError>(b"pa_strerror\0").ok())
                    .map(|symbol| *symbol);

                Some(PulseApi {
                    _library: Arc::new(library),
                    _pulse_library: pulse_library.map(Arc::new),
                    new,
                    write,
                    drain,
                    free,
                    strerror,
                })
            }
        }

        fn error_message(&self, error: c_int) -> String {
            match self.strerror {
                Some(strerror) => {
                    let message = unsafe { strerror(error) };
                    if message.is_null() {
                        format!("error {error}")
                    } else {
                        unsafe { std::ffi::CStr::from_ptr(message) }
                            .to_string_lossy()
                            .into_owned()
                    }
                }
                None => format!("error {error}"),
            }
        }

        fn play(&self, samples: &[f32], rate: u32, volume: f32) -> Result<(), String> {
            let spec = PaSampleSpec {
                format: PA_SAMPLE_FLOAT32LE,
                rate,
                channels: 2,
            };

            let name = CString::new("Invader Patrol").expect("stream name has no null bytes");
            let mut error: c_int = 0;

            // SAFETY: the sample spec and strings outlive the call, and the
            // returned stream is closed before the function returns.
            let stream = unsafe {
                (self.new)(
                    std::ptr::null(),
                    name.as_ptr(),
                    PA_STREAM_PLAYBACK,
                    std::ptr::null(),
                    name.as_ptr(),
                    &spec,
                    std::ptr::null(),
                    std::ptr::null(),
                    &mut error,
                )
            };

            if stream.is_null() {
                return Err(format!(
                    "could not open a stream: {}",
                    self.error_message(error)
                ));
            }

            let mut buffer = Vec::with_capacity(samples.len() * 2);
            for sample in samples {
                let value = sample * volume;
                buffer.push(value);
                buffer.push(value);
            }

            unsafe {
                (self.write)(
                    stream,
                    buffer.as_ptr() as *const c_void,
                    buffer.len() * std::mem::size_of::<f32>(),
                    &mut error,
                );
                (self.drain)(stream, &mut error);
                (self.free)(stream);
            }

            Ok(())
        }
    }
}
