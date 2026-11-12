//! Cover art: fetching, caching and rendering anime artwork in the terminal.
//!
//! Two paths are supported:
//!
//! * **Half-block art** - the JPEG is decoded and downscaled, then every pair
//!   of vertical pixels is drawn as one `▀` cell (upper half in the
//!   foreground colour, lower half in the background). This works in every
//!   colour terminal, so the poster card is never empty.
//! * **Native graphics** - kitty and iTerm2 can show the JPEG untouched, and
//!   are detected automatically when present.

use std::path::PathBuf;

use ratatui::style::Color;

use crate::error::{AnirelError, Result};

/// A decoded cover, downscaled to a sane maximum size.
#[derive(Debug, Clone)]
pub struct CoverArt {
    pub width: usize,
    pub height: usize,
    /// RGB triples, row major.
    pub pixels: Vec<u8>,
}

impl CoverArt {
    /// Box-filter the cover down to `width` x `height` pixels.
    pub fn resample(&self, width: usize, height: usize) -> Vec<u8> {
        let mut out = vec![0u8; width * height * 3];
        if self.width == 0 || self.height == 0 || width == 0 || height == 0 {
            return out;
        }
        for y in 0..height {
            let y0 = y * self.height / height;
            let y1 = (((y + 1) * self.height) / height)
                .max(y0 + 1)
                .min(self.height);
            for x in 0..width {
                let x0 = x * self.width / width;
                let x1 = (((x + 1) * self.width) / width).max(x0 + 1).min(self.width);
                let mut r = 0u32;
                let mut g = 0u32;
                let mut b = 0u32;
                let mut count = 0u32;
                for sy in y0..y1 {
                    for sx in x0..x1 {
                        let index = (sy * self.width + sx) * 3;
                        r += self.pixels[index] as u32;
                        g += self.pixels[index + 1] as u32;
                        b += self.pixels[index + 2] as u32;
                        count += 1;
                    }
                }
                let count = count.max(1);
                let index = (y * width + x) * 3;
                out[index] = (r / count) as u8;
                out[index + 1] = (g / count) as u8;
                out[index + 2] = (b / count) as u8;
            }
        }
        out
    }
}

/// Which native image protocol the terminal understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsSupport {
    None,
    Kitty,
    ITerm2,
}

/// Detect the terminal's native graphics protocol.
pub fn detect_graphics() -> GraphicsSupport {
    let term = std::env::var("TERM").unwrap_or_default();
    if term.contains("kitty") || std::env::var("KITTY_WINDOW_ID").is_ok() {
        return GraphicsSupport::Kitty;
    }
    if std::env::var("TERM_PROGRAM").as_deref() == Ok("iTerm.app") {
        let version = std::env::var("TERM_PROGRAM_VERSION")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0);
        if version >= 2 {
            return GraphicsSupport::ITerm2;
        }
    }
    GraphicsSupport::None
}

/// True when the terminal can render 24 bit colour.
pub fn supports_truecolor() -> bool {
    match std::env::var("COLORTERM").as_deref() {
        Ok("truecolor") | Ok("24bit") => true,
        _ => std::env::var("TERM").is_ok_and(|term| term.contains("truecolor")),
    }
}

/// Convert an RGB triple into a terminal colour.
pub fn color(r: u8, g: u8, b: u8) -> Color {
    if supports_truecolor() {
        Color::Rgb(r, g, b)
    } else {
        Color::Indexed(xterm256(r, g, b))
    }
}

/// Map an RGB triple onto the xterm 256 colour palette.
fn xterm256(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return ((r as u16 - 8) / 10) as u8 + 232;
    }
    let index = |value: u8| -> u16 {
        match value {
            0..=48 => 0,
            49..=115 => 1,
            116..=154 => 2,
            155..=193 => 3,
            194..=232 => 4,
            _ => 5,
        }
    };
    16 + 36 * index(r) as u8 + 6 * index(g) as u8 + index(b) as u8
}

/// Where a cover image is cached.
pub fn cover_path(url: &str) -> PathBuf {
    let name = format!("{}.jpg", fnv1a(url));
    crate::config::cache_dir().join("covers").join(name)
}

/// Fetch a cover image into the cache, if it is not there yet.
pub async fn ensure_cached(url: &str) -> Option<PathBuf> {
    let path = cover_path(url);
    if path.exists() {
        return Some(path);
    }
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return None;
    }
    let response = match reqwest::get(url).await {
        Ok(response) => response,
        Err(error) => {
            if std::env::var_os("ANIREL_DEBUG").is_some() {
                eprintln!("cover request failed: {error}");
            }
            return None;
        }
    };
    if !response.status().is_success() {
        if std::env::var_os("ANIREL_DEBUG").is_some() {
            eprintln!("cover status {}", response.status());
        }
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    if std::fs::write(&path, &bytes).is_err() {
        return None;
    }
    Some(path)
}

/// Decode a cached cover into pixels.
pub fn decode(path: &std::path::Path) -> Result<CoverArt> {
    let image = image::open(path).map_err(|e| AnirelError::Other(format!("cover: {e}")))?;
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let (max_width, max_height) = (220u32, 300u32);
    let scale = (width as f32 / max_width as f32)
        .max(height as f32 / max_height as f32)
        .max(1.0);
    let target_width = ((width as f32 / scale) as u32).max(1);
    let target_height = ((height as f32 / scale) as u32).max(1);

    let resized = if target_width == width && target_height == height {
        rgb
    } else {
        image::imageops::resize(
            &rgb,
            target_width,
            target_height,
            image::imageops::FilterType::Triangle,
        )
    };

    Ok(CoverArt {
        width: resized.width() as usize,
        height: resized.height() as usize,
        pixels: resized.into_raw(),
    })
}

/// Base64 encode bytes for the terminal image protocols.
pub fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Escape sequence that places a JPEG in the terminal, if the protocol exists.
pub fn native_image(protocol: GraphicsSupport, x: u16, y: u16, jpeg: &[u8]) -> Option<String> {
    let payload = base64(jpeg);
    match protocol {
        GraphicsSupport::Kitty => {
            // kitty wants the data in chunks of at most 4096 bytes.
            let mut out = format!("\x1b[{};{}H", y + 1, x + 1);
            let chunks: Vec<&str> = payload
                .as_bytes()
                .chunks(4096)
                .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
                .collect();
            let last = chunks.len().saturating_sub(1);
            for (index, chunk) in chunks.iter().enumerate() {
                if index == 0 {
                    out.push_str(&format!("\x1b_Ga=T,f=100,t=d;{chunk}\x1b\\"));
                } else if index == last {
                    out.push_str(&format!("\x1b_Gm=0;{chunk}\x1b\\"));
                } else {
                    out.push_str(&format!("\x1b_Gm=1;{chunk}\x1b\\"));
                }
            }
            Some(out)
        }
        GraphicsSupport::ITerm2 => Some(
            format!(
                "\x1b]1337;File=inline=1;preserveAspectRatio=1;size={}:{}\x07",
                jpeg.len(),
                jpeg.len()
            ) + &payload
                + "\x07",
        ),
        GraphicsSupport::None => None,
    }
}

fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn resample_produces_the_requested_size() {
        let art = CoverArt {
            width: 4,
            height: 4,
            pixels: vec![255; 4 * 4 * 3],
        };
        let small = art.resample(2, 2);
        assert_eq!(small.len(), 2 * 2 * 3);
        assert!(small.iter().all(|byte| *byte == 255));
    }

    #[test]
    fn xterm_palette_maps_greys_and_colours() {
        assert_eq!(xterm256(0, 0, 0), 16);
        assert_eq!(xterm256(255, 255, 255), 231);
        assert!(xterm256(255, 0, 0) >= 196);
    }
}
