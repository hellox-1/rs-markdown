use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use image::{DynamicImage, ImageFormat, ImageReader, Rgba};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

const MAX_IMAGE_BYTES: u64 = 12 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 20_000_000;
const MAX_PREVIEW_HEIGHT: u32 = 96;
const MAX_PREVIEW_WIDTH: u32 = 160;

#[derive(Default)]
pub(crate) struct ImageCache {
    decoded: HashMap<(PathBuf, String), std::result::Result<DynamicImage, String>>,
}

impl ImageCache {
    pub(crate) fn render_sized(
        &mut self,
        source: &str,
        base_dir: &Path,
        width: usize,
        size: Option<(u32, Option<u32>)>,
    ) -> Result<Vec<Line<'static>>> {
        let key = (base_dir.to_owned(), source.to_owned());
        if !self.decoded.contains_key(&key) {
            let result = load_image(source, base_dir).map_err(|error| format!("{error:#}"));
            self.decoded.insert(key.clone(), result);
        }

        match self.decoded.get(&key) {
            Some(Ok(image)) => Ok(to_terminal_lines(image, width, size)),
            Some(Err(error)) => Err(anyhow!("{error}")),
            None => unreachable!("image cache entry was inserted"),
        }
    }
}

fn load_image(source: &str, base_dir: &Path) -> Result<DynamicImage> {
    let bytes = load_bytes(source, base_dir)?;
    let preview_width = MAX_PREVIEW_WIDTH;
    if is_svg(source, &bytes) {
        return decode_svg(&bytes, preview_width, MAX_PREVIEW_HEIGHT);
    }

    let (width, height) = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .context("could not detect image format")?
        .into_dimensions()
        .context("could not read image dimensions")?;
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        bail!("image dimensions exceed the {MAX_IMAGE_PIXELS}-pixel limit");
    }

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("could not detect image format")?;
    let image = reader.decode().context("could not decode image")?;
    Ok(image.thumbnail(preview_width.min(width), MAX_PREVIEW_HEIGHT.min(height)))
}

fn load_bytes(source: &str, base_dir: &Path) -> Result<Vec<u8>> {
    let lower = source.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(3))
            .timeout_read(Duration::from_secs(6))
            .build();
        let response = agent
            .get(source)
            .call()
            .with_context(|| format!("could not download image from {source}"))?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .take(MAX_IMAGE_BYTES + 1)
            .read_to_end(&mut bytes)
            .context("could not read downloaded image")?;
        if bytes.len() as u64 > MAX_IMAGE_BYTES {
            bail!("image exceeds the {MAX_IMAGE_BYTES}-byte download limit");
        }
        return Ok(bytes);
    }

    let path = source.strip_prefix("file://").unwrap_or(source);
    let path = Path::new(path);
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        base_dir.join(path)
    };
    let file = fs::File::open(&path)
        .with_context(|| format!("could not open local image {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("could not read local image {}", path.display()))?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        bail!("image exceeds the {MAX_IMAGE_BYTES}-byte file limit");
    }
    Ok(bytes)
}

fn is_svg(source: &str, bytes: &[u8]) -> bool {
    let path = source.split(['?', '#']).next().unwrap_or(source);
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
        || bytes
            .windows(4)
            .any(|window| window.eq_ignore_ascii_case(b"<svg"))
}

fn decode_svg(bytes: &[u8], max_width: u32, max_height: u32) -> Result<DynamicImage> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default())
        .context("could not parse SVG image")?;
    let size = tree.size();
    let scale = (max_width as f32 / size.width())
        .min(max_height as f32 / size.height())
        .min(1.0);
    let width = (size.width() * scale).ceil().max(1.0) as u32;
    let height = (size.height() * scale).ceil().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| anyhow!("could not allocate SVG preview"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let png = pixmap
        .encode_png()
        .context("could not rasterize SVG image")?;
    image::load_from_memory_with_format(&png, ImageFormat::Png)
        .context("could not decode rasterized SVG")
}

fn to_terminal_lines(
    image: &DynamicImage,
    width: usize,
    requested_size: Option<(u32, Option<u32>)>,
) -> Vec<Line<'static>> {
    let (requested_width, requested_height) =
        requested_size.unwrap_or((MAX_PREVIEW_WIDTH, Some(MAX_PREVIEW_HEIGHT)));
    let width = width
        .clamp(1, MAX_PREVIEW_WIDTH as usize)
        .min(requested_width.max(1) as usize) as u32;
    let height = requested_height
        .unwrap_or(MAX_PREVIEW_HEIGHT)
        .clamp(1, MAX_PREVIEW_HEIGHT)
        .min(MAX_PREVIEW_HEIGHT);
    let height = height.min(image.height());
    let width = width.min(image.width());
    let image = image.thumbnail(width, height).to_rgba8();
    let mut lines = Vec::with_capacity(image.height().div_ceil(2) as usize);

    for y in (0..image.height()).step_by(2) {
        let mut spans = Vec::new();
        let mut run = String::new();
        let mut run_colors = None;
        for x in 0..image.width() {
            let top = opaque_rgb(*image.get_pixel(x, y));
            let bottom = if y + 1 < image.height() {
                opaque_rgb(*image.get_pixel(x, y + 1))
            } else {
                [0, 0, 0]
            };
            let colors = (top, bottom);
            if run_colors != Some(colors) && !run.is_empty() {
                spans.push(color_span(std::mem::take(&mut run), run_colors.unwrap()));
            }
            run_colors = Some(colors);
            run.push('▀');
        }
        if !run.is_empty() {
            spans.push(color_span(
                run,
                run_colors.expect("a non-empty run has colors"),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines
}

fn opaque_rgb(pixel: Rgba<u8>) -> [u8; 3] {
    let alpha = u16::from(pixel[3]);
    std::array::from_fn(|index| ((u16::from(pixel[index]) * alpha) / 255) as u8)
}

fn color_span(text: String, (foreground, background): ([u8; 3], [u8; 3])) -> Span<'static> {
    Span::styled(
        text,
        Style::default()
            .fg(Color::Rgb(foreground[0], foreground[1], foreground[2]))
            .bg(Color::Rgb(background[0], background[1], background[2])),
    )
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Read, Write},
        net::TcpListener,
        thread,
        time::SystemTime,
    };

    use image::{DynamicImage, ImageFormat, Rgb, RgbImage, RgbaImage};
    use unicode_width::UnicodeWidthStr;

    use super::*;

    fn sample_png() -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])));
        let mut encoded = Cursor::new(Vec::new());
        image.write_to(&mut encoded, ImageFormat::Png).unwrap();
        encoded.into_inner()
    }

    fn temporary_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "rs-markdown-image-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn previews_a_local_image_relative_to_the_markdown_file() {
        let directory = temporary_directory();
        fs::create_dir_all(&directory).unwrap();
        let image_path = directory.join("sample.png");
        fs::write(&image_path, sample_png()).unwrap();

        let mut cache = ImageCache::default();
        let decoded = load_image("sample.png", &directory).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        let lines = cache
            .render_sized("sample.png", &directory, 60, None)
            .unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "▀▀");
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Rgb(255, 0, 0)));
        let markdown_lines = crate::markdown::render_markdown(
            "![sample preview](sample.png)",
            60,
            &directory,
            &mut cache,
        );
        assert!(markdown_lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.fg == Some(Color::Rgb(255, 0, 0)))
        }));
        assert!(markdown_lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains("sample preview"))
        }));
        let sized_lines = crate::markdown::render_markdown(
            "![small](sample.png =1x1)",
            60,
            &directory,
            &mut cache,
        );
        assert!(sized_lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| UnicodeWidthStr::width(span.content.as_ref()) == 1)
        }));

        fs::remove_file(image_path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn uses_available_terminal_columns_for_image_detail() {
        let directory = temporary_directory();
        fs::create_dir_all(&directory).unwrap();
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(120, 40, Rgb([20, 80, 140])));
        let mut encoded = Cursor::new(Vec::new());
        image.write_to(&mut encoded, ImageFormat::Png).unwrap();
        let image_path = directory.join("wide.png");
        fs::write(&image_path, encoded.into_inner()).unwrap();

        let lines = ImageCache::default()
            .render_sized("wide.png", &directory, 100, None)
            .unwrap();
        let rendered_width = lines[0]
            .spans
            .iter()
            .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
            .sum::<usize>();
        assert_eq!(rendered_width, 100);

        fs::remove_file(image_path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn downloads_and_previews_a_remote_image() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let bytes = sample_png();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            stream.write_all(&bytes).unwrap();
        });

        let mut cache = ImageCache::default();
        let lines = cache
            .render_sized(
                &format!("http://{address}/sample.png"),
                Path::new("."),
                60,
                None,
            )
            .unwrap();
        server.join().unwrap();
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn previews_svg_content() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"><rect width="2" height="2" fill="#ff0000"/></svg>"##;
        let rendered = decode_svg(svg, 60, 24).unwrap();
        assert_eq!(rendered.width(), 2);
        assert_eq!(rendered.height(), 2);
    }
}
