//! Image validation and processing.
//!
//! - The real file type is detected from magic numbers, never from the extension alone.
//! - PNG/JPEG/WebP are decoded and re-encoded (max 1600px wide), which strips EXIF metadata.
//! - GIFs are stored as-is to keep animations (GIF has no EXIF block).

use std::{io::Cursor, net::IpAddr, time::Duration};

use image::{ImageFormat, ImageReader, Limits, imageops::FilterType};

use crate::error::AppError;

pub const MAX_WIDTH: u32 = 1600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Webp,
    Gif,
}

impl ImageKind {
    pub fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
            Self::Gif => "gif",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Gif => "image/gif",
        }
    }

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

pub struct ProcessedImage {
    pub bytes: Vec<u8>,
    pub kind: ImageKind,
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
        // Make sure it actually decodes, but keep the original bytes (animation).
        decode(&bytes, kind)?;
        return Ok(ProcessedImage { bytes, kind });
    }

    let mut img = decode(&bytes, kind)?;
    if img.width() > MAX_WIDTH {
        let new_height =
            ((img.height() as f64) * (MAX_WIDTH as f64) / (img.width() as f64)).round() as u32;
        img = img.resize(MAX_WIDTH, new_height.max(1), FilterType::Lanczos3);
    }

    let mut out = Cursor::new(Vec::new());
    let result = match kind {
        ImageKind::Jpeg => {
            let rgb = image::DynamicImage::ImageRgb8(img.to_rgb8());
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
            rgb.write_with_encoder(encoder)
        }
        ImageKind::Webp => {
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut out);
            img.write_with_encoder(encoder)
        }
        _ => img.write_to(&mut out, ImageFormat::Png),
    };
    result.map_err(|e| AppError::Internal(anyhow::anyhow!("image encode failed: {e}")))?;

    Ok(ProcessedImage {
        bytes: out.into_inner(),
        kind,
    })
}

fn same_family(a: ImageKind, b: ImageKind) -> bool {
    a == b
}

fn decode(bytes: &[u8], kind: ImageKind) -> Result<image::DynamicImage, AppError> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), kind.format());
    // Guard against decompression bombs.
    let mut limits = Limits::default();
    limits.max_image_width = Some(12_000);
    limits.max_image_height = Some(12_000);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
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

    #[test]
    fn rejects_extension_mismatch_and_resizes() {
        let img = image::DynamicImage::new_rgb8(2000, 1000);
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();
        let bytes = buf.into_inner();

        assert!(process(bytes.clone(), Some("photo.jpg")).is_err());

        let out = process(bytes, Some("photo.png")).unwrap();
        let decoded = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!(decoded.width(), MAX_WIDTH);
        assert_eq!(decoded.height(), 800);
    }

    #[test]
    fn private_ips_are_not_public() {
        assert!(!is_public_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_public_ip("10.1.2.3".parse().unwrap()));
        assert!(!is_public_ip("::1".parse().unwrap()));
        assert!(is_public_ip("1.1.1.1".parse().unwrap()));
    }
}
