use crate::audio::stream_source::HttpStreamSource;
use crate::error::{AppError, AppResult};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioDecoder {
    format_reader: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: usize,
}

pub struct DecodedSamples {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

impl AudioDecoder {
    pub fn new(source: HttpStreamSource, codec_hint: Option<&str>) -> AppResult<Self> {
        log::info!("AudioDecoder::new with codec_hint={:?}", codec_hint);
        let mss = MediaSourceStream::new(Box::new(source), Default::default());

        let mut hint = Hint::new();
        if let Some(codec) = codec_hint {
            let ext = match codec.to_lowercase().as_str() {
                "flac" | "flac_hires" => Some("flac"),
                "aac" | "aaclc" | "mp4a" | "mp4a.40.2" => Some("m4a"),
                "heaacv1" | "mp4a.40.5" => Some("m4a"),
                "mp4" => Some("mp4"),
                "mp3" => Some("mp3"),
                _ => {
                    log::warn!("Unknown codec hint: {}", codec);
                    None
                }
            };
            if let Some(ext) = ext {
                log::info!("Using format hint extension: {}", ext);
                hint.with_extension(ext);
            }
        }

        log::info!("Probing audio format...");
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| AppError::Decode(format!("Failed to probe format: {}", e)))?;

        let format_reader = probed.format;

        let track = format_reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| AppError::Decode("No supported audio track found".into()))?;

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| AppError::Decode(format!("Failed to create decoder: {}", e)))?;

        log::info!(
            "AudioDecoder ready: track_id={}, sample_rate={}, channels={}",
            track_id,
            sample_rate,
            channels
        );

        Ok(Self {
            format_reader,
            decoder,
            track_id,
            sample_rate,
            channels,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Seek to a position in the stream (in seconds).
    pub fn seek(&mut self, position_seconds: f64) -> AppResult<()> {
        use symphonia::core::formats::SeekTo;
        use symphonia::core::units::Time;

        let time = Time {
            seconds: position_seconds as u64,
            frac: position_seconds.fract(),
        };

        self.format_reader
            .seek(
                symphonia::core::formats::SeekMode::Coarse,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| AppError::Decode(format!("Seek failed: {}", e)))?;

        // Reset the decoder state after seeking
        self.decoder.reset();

        Ok(())
    }

    /// Decode the next batch of samples. Returns None at EOF.
    pub fn decode_next(&mut self) -> AppResult<Option<DecodedSamples>> {
        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(AppError::Decode(format!("Failed to read packet: {}", e))),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(symphonia::core::errors::Error::DecodeError(msg)) => {
                    log::warn!("Decode error (skipping): {}", msg);
                    continue;
                }
                Err(e) => return Err(AppError::Decode(format!("Failed to decode: {}", e))),
            };

            let spec = *decoded.spec();
            let num_frames = decoded.frames();
            let channels = spec.channels.count();

            let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);

            return Ok(Some(DecodedSamples {
                samples: sample_buf.samples().to_vec(),
                sample_rate: spec.rate,
                channels,
            }));
        }
    }
}
