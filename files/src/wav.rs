// Ported from Nanolathe (https://github.com/nanolathe-gg/nanolathe), formats/wav.go, commit 9e8daaf1bc9429d55157a7f5abf2a1e35ecd5e46.
// Copyright (c) 2026 Daniel G. Taylor and Nanolathe contributors. MIT License; see files/LICENSE-nanolathe.

//! WAV sound metadata: RIFF/WAVE plus the legacy containers retail plays.
//!
//! [`Wav::parse`] reads the first `fmt ` and the first `data` chunk of a RIFF file with
//! retail's unpadded chunk stride (the next chunk starts right after the payload, with no
//! pad byte) and stops at the declared RIFF span. [`Wav::parse_audio`] first classifies
//! the bytes: a `DIGI` container with `HSHD` at 8 and `SDAT` at 32 is read at fixed
//! positions; a RIFF/WAVE header goes to [`Wav::parse`]; anything else is raw 8-bit
//! unsigned mono PCM at 11025 Hz.
//!
//! The sample bytes stay in the caller's buffer: the result holds only their offset and
//! size.
//!
//! Differences from the Go source:
//! - `EncodeWAV` (a writer) is not part of the API. It is kept as a test fixture builder
//!   because Go's tests use it to check that normalized audio re-parses.
//! - Go's `LoadWAVFile`-style VFS read in `TestLoadAudioNormalizesRetailLegacyContainers`
//!   is not ported; the raw-audio case it covers is tested on bytes.
//! - A chunk id in an error is kept as four bytes and shown with lossy UTF-8; Go
//!   formatted the bytes as a string.
//! - Go reads `int32(chunkSize)`, which is negative for sizes of 2 GiB and over. The
//!   conversion is explicit here (`as_i32`) and behaves the same.

use std::fmt;

use crate::le::{u16_at, u32_at};

/// Which container the audio came in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// RIFF/WAVE.
    Riff,
    /// A `DIGI` sample: 40-byte header, then 8-bit PCM.
    Digi,
    /// Headerless 8-bit unsigned mono PCM at 11025 Hz.
    Raw,
}

impl Container {
    /// The label Go used: "RIFF", "DIGI" or "raw".
    pub fn as_str(self) -> &'static str {
        match self {
            Container::Riff => "RIFF",
            Container::Digi => "DIGI",
            Container::Raw => "raw",
        }
    }
}

/// Everything that can go wrong reading audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavError {
    /// No `RIFF` ... `WAVE` header.
    MissingRiffHeader,
    /// A chunk header runs past the end of the file.
    ChunkHeaderTruncated,
    /// A chunk's payload runs past the end of the file. Holds the chunk id.
    ChunkTruncated { id: [u8; 4] },
    /// A `fmt ` chunk of fewer than 16 bytes.
    FmtChunkTooSmall,
    /// A `data` chunk of zero bytes (or 2 GiB and over, which Go reads as negative), or
    /// one whose payload starts beyond 4 GiB.
    InvalidDataChunkSize,
    /// The declared span ends before both a `fmt ` and a `data` chunk were seen.
    MissingChunk,
    /// A `DIGI` sample shorter than 40 bytes, or with a payload over 4 GiB.
    DigiTruncatedOrTooLarge,
    /// Raw audio that is empty or over 4 GiB.
    RawEmptyOrTooLarge,
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WavError::MissingRiffHeader => write!(f, "wav: missing RIFF/WAVE header"),
            WavError::ChunkHeaderTruncated => write!(f, "wav: chunk header is truncated"),
            WavError::ChunkTruncated { id } => {
                write!(f, "wav: {} chunk is truncated", String::from_utf8_lossy(id))
            }
            WavError::FmtChunkTooSmall => write!(f, "wav: fmt chunk is too small"),
            WavError::InvalidDataChunkSize => write!(f, "wav: invalid data chunk size"),
            WavError::MissingChunk => write!(f, "wav: missing fmt or data chunk"),
            WavError::DigiTruncatedOrTooLarge => {
                write!(f, "wav: DIGI sample is truncated or too large")
            }
            WavError::RawEmptyOrTooLarge => {
                write!(f, "wav: raw audio is empty or too large")
            }
        }
    }
}

impl std::error::Error for WavError {}

/// PCM metadata and where the samples are. The authored words are kept as they are;
/// playback policy belongs to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wav {
    pub container: Container,
    pub audio_format: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub byte_rate: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    /// Offset of the samples in the source bytes.
    pub data_offset: u32,
    /// Size of the samples in bytes.
    pub data_size: u32,
}

/// Go's `int32(uint32)`: two's-complement reinterpretation, no saturation.
fn as_i32(value: u32) -> i32 {
    i32::from_le_bytes(value.to_le_bytes())
}

impl Wav {
    /// Read the first `fmt ` and `data` records of a RIFF/WAVE file, using retail's
    /// unpadded chunk stride and the declared RIFF span.
    pub fn parse(data: &[u8]) -> Result<Wav, WavError> {
        if data.len() < 12
            || data.get(..4) != Some(b"RIFF".as_slice())
            || data.get(8..12) != Some(b"WAVE".as_slice())
        {
            return Err(WavError::MissingRiffHeader);
        }
        let mut result = Wav {
            container: Container::Riff,
            audio_format: 0,
            channels: 0,
            sample_rate: 0,
            byte_rate: 0,
            block_align: 0,
            bits_per_sample: 0,
            data_offset: 0,
            data_size: 0,
        };
        let (mut have_format, mut have_data) = (false, false);
        // The declared span: the RIFF size word plus the eight bytes before it. A `u64`,
        // so a size near 4 GiB does not wrap.
        let span = u64::from(u32_at(data, 4).ok_or(WavError::MissingRiffHeader)?) + 8;
        let len = data.len() as u64;
        let mut offset: u64 = 12;
        while offset < span {
            if offset + 8 > len {
                return Err(WavError::ChunkHeaderTruncated);
            }
            let at = usize::try_from(offset).map_err(|_| WavError::ChunkHeaderTruncated)?;
            let mut id = [0u8; 4];
            id.copy_from_slice(data.get(at..at + 4).ok_or(WavError::ChunkHeaderTruncated)?);
            let chunk_size = u32_at(data, at + 4).ok_or(WavError::ChunkHeaderTruncated)?;
            let payload = offset + 8;
            // Checked slices are a host-safety policy for malformed containers.
            if u64::from(chunk_size) > len - payload {
                return Err(WavError::ChunkTruncated { id });
            }
            let payload_at = at + 8;
            if &id == b"fmt " && !have_format {
                if as_i32(chunk_size) < 16 {
                    return Err(WavError::FmtChunkTooSmall);
                }
                // `chunk_size >= 16` and the bounds check above make these reads fit.
                let short = WavError::ChunkTruncated { id };
                result.audio_format = u16_at(data, payload_at).ok_or(short)?;
                result.channels = u16_at(data, payload_at + 2).ok_or(short)?;
                result.sample_rate = u32_at(data, payload_at + 4).ok_or(short)?;
                result.byte_rate = u32_at(data, payload_at + 8).ok_or(short)?;
                result.block_align = u16_at(data, payload_at + 12).ok_or(short)?;
                result.bits_per_sample = u16_at(data, payload_at + 14).ok_or(short)?;
                have_format = true;
            } else if &id == b"data" && !have_data {
                if as_i32(chunk_size) <= 0 {
                    return Err(WavError::InvalidDataChunkSize);
                }
                result.data_offset =
                    u32::try_from(payload).map_err(|_| WavError::InvalidDataChunkSize)?;
                result.data_size = chunk_size;
                have_data = true;
            }
            if have_format && have_data {
                return Ok(result);
            }
            offset = payload + u64::from(chunk_size);
        }
        Err(WavError::MissingChunk)
    }

    /// The WAV-family classifier and payload locator. An unrecognized signature selects
    /// raw PCM, including a damaged `DIGI` signature; a recognized but truncated header
    /// is an error.
    pub fn parse_audio(data: &[u8]) -> Result<Wav, WavError> {
        if data.len() >= 36
            && data.get(..4) == Some(b"DIGI".as_slice())
            && data.get(8..12) == Some(b"HSHD".as_slice())
            && data.get(32..36) == Some(b"SDAT".as_slice())
        {
            return Self::parse_digi(data);
        }
        if data.len() >= 12
            && data.get(..4) == Some(b"RIFF".as_slice())
            && data.get(8..12) == Some(b"WAVE".as_slice())
        {
            return Self::parse(data);
        }
        let size = u32::try_from(data.len()).map_err(|_| WavError::RawEmptyOrTooLarge)?;
        if size == 0 {
            return Err(WavError::RawEmptyOrTooLarge);
        }
        Ok(Wav {
            container: Container::Raw,
            audio_format: 1,
            channels: 1,
            sample_rate: 11025,
            byte_rate: 11025,
            block_align: 1,
            bits_per_sample: 8,
            data_offset: 0,
            data_size: size,
        })
    }

    fn parse_digi(data: &[u8]) -> Result<Wav, WavError> {
        let payload = data
            .len()
            .checked_sub(40)
            .ok_or(WavError::DigiTruncatedOrTooLarge)?;
        let data_size = u32::try_from(payload).map_err(|_| WavError::DigiTruncatedOrTooLarge)?;
        // Fixed positions, independent of all advertised chunk sizes.
        let mut rate = u32_at(data, 22).ok_or(WavError::DigiTruncatedOrTooLarge)?;
        if rate == 11000 {
            rate = 11025;
        }
        Ok(Wav {
            container: Container::Digi,
            audio_format: 1,
            channels: 1,
            sample_rate: rate,
            byte_rate: rate,
            block_align: 1,
            bits_per_sample: 8,
            data_offset: 40,
            data_size,
        })
    }

    /// The sample bytes within `source`, or `None` when the span lies outside it.
    pub fn samples<'a>(&self, source: &'a [u8]) -> Option<&'a [u8]> {
        let start = usize::try_from(self.data_offset).ok()?;
        let end = start.checked_add(usize::try_from(self.data_size).ok()?)?;
        source.get(start..end)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )]

    use super::*;

    /// Port of `EncodeWAV`, a writer, kept only to build and check fixtures: a canonical
    /// PCM RIFF/WAVE wrapper around the samples `w` describes in `data`.
    fn encode_wav(data: &[u8], w: &Wav) -> Result<Vec<u8>, &'static str> {
        if w.audio_format != 1
            || w.channels == 0
            || w.sample_rate == 0
            || w.bits_per_sample == 0
            || !w.bits_per_sample.is_multiple_of(8)
        {
            return Err("only non-empty PCM metadata can be encoded");
        }
        let expected_block_align = u64::from(w.channels) * u64::from(w.bits_per_sample) / 8;
        if expected_block_align > u64::from(u16::MAX)
            || u64::from(w.block_align) != expected_block_align
        {
            return Err("block alignment does not match PCM metadata");
        }
        let expected_byte_rate = u64::from(w.sample_rate) * expected_block_align;
        if expected_byte_rate > u64::from(u32::MAX) || u64::from(w.byte_rate) != expected_byte_rate
        {
            return Err("byte rate does not match PCM metadata");
        }
        let pcm = w.samples(data).ok_or("audio data is outside source")?;
        if !(pcm.len() as u64).is_multiple_of(expected_block_align) {
            return Err("PCM data is not aligned to a sample");
        }
        if pcm.len() as u64 > u64::from(u32::MAX) - 36 {
            return Err("encoded audio is too large");
        }
        let mut out = Vec::with_capacity(44 + pcm.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&((44 + pcm.len() - 8) as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&w.audio_format.to_le_bytes());
        out.extend_from_slice(&w.channels.to_le_bytes());
        out.extend_from_slice(&w.sample_rate.to_le_bytes());
        out.extend_from_slice(&w.byte_rate.to_le_bytes());
        out.extend_from_slice(&w.block_align.to_le_bytes());
        out.extend_from_slice(&w.bits_per_sample.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
        out.extend_from_slice(pcm);
        Ok(out)
    }

    /// Port of `TestWAVDecodesPCMMetadata`.
    #[test]
    fn decodes_pcm_metadata() {
        let mut data = vec![0u8; 48];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&40u32.to_le_bytes());
        data[8..12].copy_from_slice(b"WAVE");
        data[12..16].copy_from_slice(b"fmt ");
        data[16..20].copy_from_slice(&16u32.to_le_bytes());
        data[20..22].copy_from_slice(&1u16.to_le_bytes());
        data[22..24].copy_from_slice(&1u16.to_le_bytes());
        data[24..28].copy_from_slice(&11025u32.to_le_bytes());
        data[28..32].copy_from_slice(&11025u32.to_le_bytes());
        data[32..34].copy_from_slice(&1u16.to_le_bytes());
        data[34..36].copy_from_slice(&8u16.to_le_bytes());
        data[36..40].copy_from_slice(b"data");
        data[40..44].copy_from_slice(&4u32.to_le_bytes());
        let wav = Wav::parse(&data).unwrap();
        assert_eq!(wav.container, Container::Riff);
        assert_eq!(
            (
                wav.audio_format,
                wav.channels,
                wav.sample_rate,
                wav.data_size
            ),
            (1, 1, 11025, 4)
        );
        assert_eq!(wav.data_offset, 44);
        assert_eq!(
            (wav.byte_rate, wav.block_align, wav.bits_per_sample),
            (11025, 1, 8)
        );
    }

    /// Port of `TestLoadAudioNormalizesRetailLegacyContainers`, minus the VFS-backed
    /// file read.
    #[test]
    fn load_audio_normalizes_retail_legacy_containers() {
        let raw = [0x80u8, 0x90, 0x70];
        let audio = Wav::parse_audio(&raw).unwrap();
        assert_eq!(audio.container, Container::Raw);
        assert_eq!(audio.container.as_str(), "raw");
        assert_eq!((audio.data_offset, audio.data_size), (0, 3));
        assert_eq!(audio.sample_rate, 11025);
        let encoded = encode_wav(&raw, &audio).unwrap();
        assert_eq!(&encoded[44..], &raw);

        let mut digi = vec![0u8; 43];
        digi[0..4].copy_from_slice(b"DIGI");
        digi[8..12].copy_from_slice(b"HSHD");
        digi[12..16].copy_from_slice(&24u32.to_be_bytes());
        digi[22..26].copy_from_slice(&11000u32.to_le_bytes());
        digi[32..36].copy_from_slice(b"SDAT");
        digi[36..40].copy_from_slice(&11u32.to_be_bytes());
        digi[40..].copy_from_slice(&raw);
        let audio = Wav::parse_audio(&digi).unwrap();
        assert_eq!(audio.container, Container::Digi);
        assert_eq!(audio.container.as_str(), "DIGI");
        assert_eq!((audio.data_offset, audio.data_size), (40, 3));
        assert_eq!(audio.sample_rate, 11025, "11000 Hz is normalized to 11025");
        assert_eq!(audio.byte_rate, 11025);
        let encoded = encode_wav(&digi, &audio).unwrap();
        let decoded = Wav::parse(&encoded).unwrap();
        assert_eq!(decoded.data_size, 3);
        assert_eq!(&encoded[44..], &raw);

        let mut bad = audio;
        bad.byte_rate = 0;
        assert!(
            encode_wav(&raw, &bad).is_err(),
            "invalid PCM metadata was encoded"
        );
    }

    fn authored_wav_chunk(name: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut chunk = name.to_vec();
        chunk.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        chunk.extend_from_slice(payload);
        chunk
    }

    fn authored_riff(chunks: &[&[u8]]) -> Vec<u8> {
        let mut data = vec![0u8; 12];
        data[0..4].copy_from_slice(b"RIFF");
        data[8..12].copy_from_slice(b"WAVE");
        for chunk in chunks {
            data.extend_from_slice(chunk);
        }
        let size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&size.to_le_bytes());
        data
    }

    fn fmt_body(rate: u32) -> Vec<u8> {
        let mut body = vec![0u8; 16];
        body[0..2].copy_from_slice(&1u16.to_le_bytes());
        body[2..4].copy_from_slice(&1u16.to_le_bytes());
        body[4..8].copy_from_slice(&rate.to_le_bytes());
        body[14..16].copy_from_slice(&8u16.to_le_bytes());
        body
    }

    /// Port of `TestWAVFirstChunksAndDeclaredSpan`.
    #[test]
    fn first_chunks_and_declared_span() {
        let first_fmt = authored_wav_chunk(b"fmt ", &fmt_body(11025));
        let second_fmt = authored_wav_chunk(b"fmt ", &fmt_body(22050));
        let first_data = authored_wav_chunk(b"data", &[1, 2, 3]);
        let second_data = authored_wav_chunk(b"data", &[9]);
        let junk = authored_wav_chunk(b"JUNK", &[8, 7, 6]);
        let orders: [Vec<&[u8]>; 2] = [
            vec![&junk, &first_fmt, &second_fmt, &first_data, &second_data],
            vec![&first_data, &second_data, &first_fmt, &second_fmt],
        ];
        for chunks in &orders {
            let data = authored_riff(chunks);
            let w = Wav::parse(&data).unwrap();
            assert_eq!(w.sample_rate, 11025);
            assert_eq!(w.data_size, 3);
            assert_eq!(w.samples(&data), Some([1u8, 2, 3].as_slice()));
        }
        // A data chunk beyond the declared RIFF span is not read.
        let mut data = authored_riff(&[&first_fmt]);
        data.extend_from_slice(&first_data);
        assert_eq!(Wav::parse(&data), Err(WavError::MissingChunk));
        // Bytes after the span do not change the selected records.
        let mut data = authored_riff(&[&first_fmt, &first_data]);
        data.extend_from_slice(b"unparsed outside span");
        assert!(
            Wav::parse(&data).is_ok(),
            "trailing bytes changed selected records"
        );
        // A data chunk that claims more bytes than the file has.
        for payload in [&[][..], &[1, 2][..]] {
            let chunk = authored_wav_chunk(b"data", payload);
            let mut data = authored_riff(&[&first_fmt, &chunk]);
            let at = data.len() - payload.len() - 4;
            data[at..at + 4].copy_from_slice(&10u32.to_le_bytes());
            assert_eq!(
                Wav::parse(&data),
                Err(WavError::ChunkTruncated { id: *b"data" }),
                "accepted short data read"
            );
        }
    }

    /// Port of `TestWAVFixedSignatureFallbackAndEmptyData`.
    #[test]
    fn fixed_signature_fallback_and_empty_data() {
        let mut data = vec![0u8; 40];
        data[0..4].copy_from_slice(b"DIGI");
        data[8..12].copy_from_slice(b"HSHD");
        // A missing fixed SDAT marker selects raw, even with a DIGI prefix.
        let w = Wav::parse_audio(&data).unwrap();
        assert_eq!((w.container, w.data_offset), (Container::Raw, 0));
        data[32..36].copy_from_slice(b"SDAT");
        assert_eq!(
            Wav::parse_audio(&data[..36]),
            Err(WavError::DigiTruncatedOrTooLarge),
            "accepted truncated recognized DIGI"
        );
        // Forty bytes is a DIGI with no samples.
        assert_eq!(Wav::parse_audio(&data).unwrap().data_size, 0);
        let empty_data = authored_wav_chunk(b"data", &[]);
        let fmt = authored_wav_chunk(b"fmt ", &[0u8; 16]);
        assert_eq!(
            Wav::parse(&authored_riff(&[&fmt, &empty_data])),
            Err(WavError::InvalidDataChunkSize),
            "accepted zero-length RIFF data"
        );
    }

    #[test]
    fn header_and_chunk_errors() {
        assert_eq!(Wav::parse(&[]), Err(WavError::MissingRiffHeader));
        assert_eq!(
            Wav::parse(b"RIFF\0\0\0\0WAVX"),
            Err(WavError::MissingRiffHeader)
        );
        assert_eq!(Wav::parse_audio(&[]), Err(WavError::RawEmptyOrTooLarge));
        // A span that reaches the end with room for less than a chunk header.
        let mut data = b"RIFF\0\0\0\0WAVE".to_vec();
        data[4..8].copy_from_slice(&100u32.to_le_bytes());
        data.extend_from_slice(b"fmt ");
        assert_eq!(Wav::parse(&data), Err(WavError::ChunkHeaderTruncated));
        // A span of zero declares no chunks at all.
        assert_eq!(Wav::parse(b"RIFF\0\0\0\0WAVE"), Err(WavError::MissingChunk));
        // A fmt chunk under 16 bytes.
        let small = authored_wav_chunk(b"fmt ", &[0u8; 15]);
        assert_eq!(
            Wav::parse(&authored_riff(&[&small])),
            Err(WavError::FmtChunkTooSmall)
        );
        // Chunk sizes of 2 GiB and over read as negative: rejected for fmt and data.
        let fmt = authored_wav_chunk(b"fmt ", &fmt_body(11025));
        let mut data = authored_riff(&[&fmt, b"data\xff\xff\xff\xff"]);
        assert_eq!(
            Wav::parse(&data),
            Err(WavError::ChunkTruncated { id: *b"data" }),
            "a huge size claims more than the file holds"
        );
        // The same size in a fmt chunk is rejected the same way.
        data = authored_riff(&[b"fmt \x00\x00\x00\x80"]);
        assert_eq!(
            Wav::parse(&data),
            Err(WavError::ChunkTruncated { id: *b"fmt " })
        );
    }

    /// Hostile: a RIFF size word of 4 GiB, and chunk sizes that would overflow a 32-bit
    /// offset sum, must neither wrap nor loop.
    #[test]
    fn hostile_sizes_neither_wrap_nor_loop() {
        let fmt = authored_wav_chunk(b"fmt ", &fmt_body(11025));
        let mut data = authored_riff(&[&fmt, &authored_wav_chunk(b"JUNK", &[1])]);
        data[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        // The span now reaches far past the end: the walk ends at a truncated header.
        assert_eq!(Wav::parse(&data), Err(WavError::ChunkHeaderTruncated));
        // Offsets advance by at least eight bytes a chunk, so any file terminates.
        let mut many = vec![0u8; 12];
        many[0..4].copy_from_slice(b"RIFF");
        many[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        many[8..12].copy_from_slice(b"WAVE");
        for _ in 0..1000 {
            many.extend_from_slice(&authored_wav_chunk(b"JUNK", &[]));
        }
        assert_eq!(Wav::parse(&many), Err(WavError::ChunkHeaderTruncated));
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let fmt = authored_wav_chunk(b"fmt ", &fmt_body(11025));
        let chunk = authored_wav_chunk(b"data", &[1, 2, 3]);
        let data = authored_riff(&[&fmt, &chunk]);
        assert!(Wav::parse(&data).is_ok());
        for len in 0..data.len() {
            assert!(Wav::parse(&data[..len]).is_err(), "prefix {len}");
            let _ = Wav::parse_audio(&data[..len]);
        }
    }

    #[test]
    fn samples_outside_the_source_are_none() {
        let w = Wav::parse_audio(&[1, 2, 3]).unwrap();
        assert_eq!(w.samples(&[1, 2, 3]), Some([1u8, 2, 3].as_slice()));
        assert_eq!(w.samples(&[1, 2]), None);
    }
}
