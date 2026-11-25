//! Image validation and processing.
//!
//! - The real file type is detected from magic numbers, never from the extension alone.
//! - Uploads are decoded and re-encoded (max 1600px wide), which strips EXIF metadata, into
//!   the smallest format browsers support well:
//!   - still images (PNG, JPEG, WebP, single-frame GIF) become AVIF;
//!   - animated GIFs become animated WebP (kept as GIF if that is somehow larger).

use std::{io::Cursor, net::IpAddr, time::Duration};

use image::{
    AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbaImage,
    codecs::gif::GifDecoder, imageops::FilterType,
};

use crate::error::AppError;

pub const MAX_WIDTH: u32 = 1600;

/// AVIF quality (0–100). 75 keeps text in screenshots crisp at a fraction of PNG/JPEG size.
const AVIF_QUALITY: f32 = 75.0;
/// AVIF encoder speed (1 = slowest/smallest, 10 = fastest). 6 is a good size/time balance.
const AVIF_SPEED: u8 = 6;
/// Animated WebP quality (0–100).
const WEBP_QUALITY: f32 = 75.0;
/// Guards for animated GIFs: frame count and total decoded RGBA bytes.
const MAX_GIF_FRAMES: usize = 1000;
const MAX_GIF_DECODED_BYTES: usize = 512 * 1024 * 1024;

/// The upload's real type, detected from its magic number.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Webp,
    Gif,
}

impl ImageKind {
    fn format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Webp => ImageFormat::WebP,
            Self::Gif => ImageFormat::Gif,
        }
    }

    /// Kind implied by a file name's extension (used to cross-check the magic number).
    pub fn from_filename(name: &str) -> Option<Self> {
        let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
        match ext.as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::Webp),
            "gif" => Some(Self::Gif),
            _ => None,
        }
    }
}

/// Detect the image type from the first bytes of the file.
pub fn sniff(bytes: &[u8]) -> Option<ImageKind> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(ImageKind::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageKind::Jpeg)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(ImageKind::Gif)
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(ImageKind::Webp)
    } else {
        None
    }
}

/// The format an image is stored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredFormat {
    Avif,
    Webp,
    Gif,
}

impl StoredFormat {
    pub fn ext(self) -> &'static str {
        match self {
            Self::Avif => "avif",
            Self::Webp => "webp",
            Self::Gif => "gif",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Avif => "image/avif",
            Self::Webp => "image/webp",
            Self::Gif => "image/gif",
        }
    }
}

pub struct ProcessedImage {
    pub bytes: Vec<u8>,
    pub kind: StoredFormat,
}

/// Validate, then resize and re-encode. CPU-heavy: call through `spawn_blocking`.
pub fn process(bytes: Vec<u8>, filename: Option<&str>) -> Result<ProcessedImage, AppError> {
    let kind = sniff(&bytes).ok_or_else(|| {
        AppError::UnsupportedMediaType("only PNG, JPEG, WebP and GIF images are accepted".into())
    })?;

    if let Some(name) = filename {
        match ImageKind::from_filename(name) {
            Some(by_ext) if same_family(by_ext, kind) => {}
            Some(_) => {
                return Err(AppError::UnsupportedMediaType(
                    "the file extension does not match the image content".into(),
                ));
            }
            None if name.contains('.') => {
                return Err(AppError::UnsupportedMediaType(
                    "allowed extensions: png, jpg, jpeg, webp, gif".into(),
                ));
            }
            None => {} // e.g. pasted clipboard images with no extension
        }
    }

    if kind == ImageKind::Gif {
        let frames = decode_gif_frames(&bytes)?;
        if frames.len() > 1 {
            let webp = encode_animated_webp(&frames)?;
            // Tiny pixel-art GIFs can beat lossy WebP; GIF has no EXIF, so keeping it is safe.
            return Ok(if webp.len() < bytes.len() {
                ProcessedImage {
                    bytes: webp,
                    kind: StoredFormat::Webp,
                }
            } else {
                ProcessedImage {
                    bytes,
                    kind: StoredFormat::Gif,
                }
            });
        }
        let still = frames
            .into_iter()
            .next()
            .map(|(img, _)| img)
            .ok_or_else(|| AppError::BadRequest("the image could not be decoded".into()))?;
        return encode_avif(DynamicImage::ImageRgba8(still));
    }

    encode_avif(decode(&bytes, kind)?)
}

/// Shrink to at most `MAX_WIDTH` wide, keeping the aspect ratio.
fn fit_width(img: DynamicImage) -> DynamicImage {
    if img.width() <= MAX_WIDTH {
        return img;
    }
    let new_height =
        ((img.height() as f64) * (MAX_WIDTH as f64) / (img.width() as f64)).round() as u32;
    img.resize_exact(MAX_WIDTH, new_height.max(1), FilterType::Lanczos3)
}

fn encode_avif(img: DynamicImage) -> Result<ProcessedImage, AppError> {
    let img = fit_width(img).into_rgba8();
    let pixels: Vec<ravif::RGBA8> = img
        .pixels()
        .map(|p| ravif::RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    let encoded = ravif::Encoder::new()
        .with_quality(AVIF_QUALITY)
        .with_alpha_quality(AVIF_QUALITY)
        .with_speed(AVIF_SPEED)
        .encode_rgba(ravif::Img::new(
            pixels.as_slice(),
            img.width() as usize,
            img.height() as usize,
        ))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("AVIF encode failed: {e}")))?;
    Ok(ProcessedImage {
        bytes: encoded.avif_file,
        kind: StoredFormat::Avif,
    })
}

/// Decode every frame of a GIF (fully composited) with its delay in milliseconds.
fn decode_gif_frames(bytes: &[u8]) -> Result<Vec<(RgbaImage, u32)>, AppError> {
    let bad = || AppError::BadRequest("the image could not be decoded".into());
    let mut decoder = GifDecoder::new(Cursor::new(bytes)).map_err(|_| bad())?;
    decoder.set_limits(decode_limits()).map_err(|_| bad())?;

    let mut frames = Vec::new();
    let mut total_bytes = 0usize;
    for frame in decoder.into_frames() {
        let frame = frame.map_err(|_| bad())?;
        let (num, den) = frame.delay().numer_denom_ms();
        let delay_ms = num.checked_div(den).unwrap_or(0);
        let buffer = frame.into_buffer();
        total_bytes += buffer.as_raw().len();
        if frames.len() >= MAX_GIF_FRAMES || total_bytes > MAX_GIF_DECODED_BYTES {
            return Err(AppError::PayloadTooLarge(
                "this GIF has too many frames or is too large to process".into(),
            ));
        }
        frames.push((buffer, delay_ms));
    }
    Ok(frames)
}

fn encode_animated_webp(frames: &[(RgbaImage, u32)]) -> Result<Vec<u8>, AppError> {
    let fail =
        |what: &str| AppError::Internal(anyhow::anyhow!("animated WebP encode failed: {what}"));
    let frames: Vec<(RgbaImage, u32)> = frames
        .iter()
        .map(|(img, delay)| {
            let img = fit_width(DynamicImage::ImageRgba8(img.clone())).into_rgba8();
            // Browsers treat GIF delays under 20 ms as 100 ms; do the same so speed matches.
            (img, if *delay < 20 { 100 } else { *delay })
        })
        .collect();
    let (width, height) = frames[0].0.dimensions();

    let mut config = webp::WebPConfig::new().map_err(|_| fail("config"))?;
    config.lossless = 0;
    config.quality = WEBP_QUALITY;
    config.method = 4;

    let mut encoder = webp::AnimEncoder::new(width, height, &config);
    encoder.set_loop_count(0); // loop forever, like GIFs
    let mut timestamp = 0i32;
    for (img, delay) in &frames {
        encoder.add_frame(webp::AnimFrame::from_rgba(
            img.as_raw(),
            width,
            height,
            timestamp,
        ));
        timestamp += *delay as i32;
    }
    let mut out = encoder
        .try_encode()
        .map_err(|e| fail(&format!("{e:?}")))?
        .to_vec();
    // `webp` 0.3 closes the animation with timestamp 0, so libwebp guesses the last frame's
    // duration. Write the real one into the file.
    let last_delay = frames.last().map_or(100, |(_, d)| *d);
    set_last_frame_duration(&mut out, last_delay).ok_or_else(|| fail("no ANMF chunk"))?;
    Ok(out)
}

/// Overwrite the duration of the last `ANMF` (animation frame) chunk of a WebP file.
/// ANMF payload: X(3) Y(3) width-1(3) height-1(3) duration(3, little endian) flags(1) ...
fn set_last_frame_duration(webp: &mut [u8], duration_ms: u32) -> Option<()> {
    let mut pos = 12; // after "RIFF" <size> "WEBP"
    let mut last_anmf = None;
    while pos + 8 <= webp.len() {
        let size = u32::from_le_bytes(webp[pos + 4..pos + 8].try_into().ok()?) as usize;
        if &webp[pos..pos + 4] == b"ANMF" {
            last_anmf = Some(pos + 8);
        }
        pos += 8 + size + (size & 1); // chunks are padded to an even size
    }
    let at = last_anmf? + 12;
    let d = duration_ms.min(0xFF_FFFF).to_le_bytes();
    webp.get_mut(at..at + 3)?.copy_from_slice(&d[..3]);
    Some(())
}

fn same_family(a: ImageKind, b: ImageKind) -> bool {
    a == b
}

/// Guard against decompression bombs.
fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(12_000);
    limits.max_image_height = Some(12_000);
    limits.max_alloc = Some(512 * 1024 * 1024);
    limits
}

fn decode(bytes: &[u8], kind: ImageKind) -> Result<DynamicImage, AppError> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), kind.format());
    reader.limits(decode_limits());
    reader
        .decode()
        .map_err(|_| AppError::BadRequest("the image could not be decoded".into()))
}

/// Download an image from a public http(s) URL (used by the bulk import).
/// Refuses private/loopback addresses and redirects to limit SSRF from staff input.
pub async fn download(url: &str, max_bytes: usize) -> Result<Vec<u8>, AppError> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| AppError::Validation(format!("invalid image_url: {url}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::Validation(format!(
            "image_url must be http(s): {url}"
        )));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::Validation(format!("image_url has no host: {url}")))?;
    let port = parsed.port_or_known_default().unwrap_or(443);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| AppError::Validation(format!("cannot resolve host for {url}")))?;
    for addr in addrs {
        if !is_public_ip(addr.ip()) {
            return Err(AppError::Validation(format!(
                "image_url points to a private address: {url}"
            )));
        }
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut resp = client
        .get(parsed)
        .send()
        .await
        .map_err(|_| AppError::Validation(format!("could not download {url}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Validation(format!(
            "downloading {url} returned HTTP {}",
            resp.status()
        )));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|_| AppError::Validation(format!("download interrupted: {url}")))?
    {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > max_bytes {
            return Err(AppError::PayloadTooLarge(format!(
                "image at {url} is larger than the upload limit"
            )));
        }
    }
    Ok(bytes)
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_multicast()
                || v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64) // CGNAT 100.64/10
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00 // unique local
                || (first & 0xffc0) == 0xfe80) // link local
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_magic_numbers() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\nrest"), Some(ImageKind::Png));
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(ImageKind::Jpeg));
        assert_eq!(sniff(b"GIF89a...."), Some(ImageKind::Gif));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some(ImageKind::Webp));
        assert_eq!(sniff(b"<html>"), None);
    }

    #[test]
    fn rejects_html_disguised_as_png() {
        let err = process(b"<script>alert(1)</script>".to_vec(), Some("x.png"));
        assert!(matches!(err, Err(AppError::UnsupportedMediaType(_))));
    }

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let img = DynamicImage::new_rgb8(width, height);
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn rejects_extension_mismatch() {
        assert!(process(png_bytes(8, 8), Some("photo.jpg")).is_err());
    }

    #[test]
    fn still_images_become_avif() {
        let out = process(png_bytes(64, 32), Some("photo.png")).unwrap();
        assert_eq!(out.kind, StoredFormat::Avif);
        assert_eq!(&out.bytes[4..12], b"ftypavif");
    }

    #[test]
    fn resizes_to_max_width() {
        let img = fit_width(DynamicImage::new_rgb8(2000, 1000));
        assert_eq!((img.width(), img.height()), (MAX_WIDTH, 800));
        let small = fit_width(DynamicImage::new_rgb8(300, 200));
        assert_eq!((small.width(), small.height()), (300, 200));
    }

    fn gif_bytes(delays_ms: &[u32]) -> Vec<u8> {
        use image::{Delay, Frame, Rgba, codecs::gif::GifEncoder};
        let mut out = Vec::new();
        {
            let mut enc = GifEncoder::new(&mut out);
            for (i, d) in delays_ms.iter().enumerate() {
                // Busy pixels so the GIF is bigger than the WebP.
                let img = RgbaImage::from_fn(120, 120, |x, y| {
                    Rgba([
                        (x * 7 + i as u32 * 40) as u8,
                        (y * 5) as u8,
                        ((x ^ y) * 3) as u8,
                        255,
                    ])
                });
                enc.encode_frame(Frame::from_parts(
                    img,
                    0,
                    0,
                    Delay::from_numer_denom_ms(*d, 1),
                ))
                .unwrap();
            }
        }
        out
    }

    #[test]
    fn animated_gif_becomes_animated_webp_with_same_timing() {
        use image::codecs::webp::WebPDecoder;
        let delays = [100, 200, 300];
        let out = process(gif_bytes(&delays), Some("anim.gif")).unwrap();
        assert_eq!(out.kind, StoredFormat::Webp);

        let decoded: Vec<u32> = WebPDecoder::new(Cursor::new(out.bytes))
            .unwrap()
            .into_frames()
            .map(|f| {
                let (n, d) = f.unwrap().delay().numer_denom_ms();
                n / d
            })
            .collect();
        assert_eq!(decoded, delays);
    }

    #[test]
    fn single_frame_gif_becomes_avif() {
        let out = process(gif_bytes(&[100]), Some("still.gif")).unwrap();
        assert_eq!(out.kind, StoredFormat::Avif);
    }

    #[test]
    fn private_ips_are_not_public() {
        assert!(!is_public_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_public_ip("10.1.2.3".parse().unwrap()));
        assert!(!is_public_ip("::1".parse().unwrap()));
        assert!(is_public_ip("1.1.1.1".parse().unwrap()));
    }
}
