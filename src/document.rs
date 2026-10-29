use std::fs;

use crate::info;
use anyhow::{anyhow, Result};
use base64::Engine;
use colored::Colorize;
use handlebars::Handlebars;
use image::{DynamicImage, ImageFormat};
use lol_html::{element, HtmlRewriter, Settings};
use serde::Serialize;
use std::ffi::OsStr;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::included::{TEMPLATES_DIR, VENDOR_DIR};
use crate::themes::Theme;

pub struct Document {
    pub text: String,
    pub options: RenderOptions,
    /// Directory that relative local image paths are resolved against: the
    /// markdown file's directory, or the current directory for --string/stdin.
    pub base_dir: PathBuf,
}

/// Where an `<img src>` points, as far as image inlining is concerned.
#[derive(Debug, PartialEq)]
enum ImageSource {
    Remote(String),
    Local(PathBuf),
    /// Empty (e.g. a data: URI the markdown compiler stripped) or an
    /// unsupported scheme such as mailto:.
    Unsupported,
}

/// The directory relative image paths in `path` refer to; the current
/// directory when the markdown didn't come from a file.
pub fn base_dir_for(path: Option<&Path>) -> PathBuf {
    match path.and_then(Path::parent) {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Decodes HTML character references in an attribute value. lol_html returns
/// attribute values raw, and the markdown compiler escapes `&`, `<`, `>`, `"`.
fn decode_html_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];

        let decoded = rest.find(';').and_then(|semi| {
            let entity = &rest[1..semi];
            let ch = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => {
                    let code = if let Some(hex) = entity
                        .strip_prefix(['#'])
                        .and_then(|e| e.strip_prefix(['x', 'X']))
                    {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        entity
                            .strip_prefix('#')
                            .and_then(|dec| dec.parse::<u32>().ok())
                    };
                    code.and_then(char::from_u32)
                }
            };
            ch.map(|c| (c, semi + 1))
        });

        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }

    out.push_str(rest);
    out
}

fn classify_image_source(src: &str, base_dir: &Path) -> ImageSource {
    let decode = |s: &str| {
        percent_encoding::percent_decode_str(s)
            .decode_utf8_lossy()
            .into_owned()
    };

    if src.is_empty() {
        return ImageSource::Unsupported;
    }

    if src.starts_with("//") {
        return ImageSource::Remote(format!("https:{}", src));
    }

    // A scheme is letters/digits/+-. before the first ':', with no '/' before it.
    // A single letter is a Windows drive (C:/...), not a scheme.
    if let Some((scheme, rest)) = src.split_once(':') {
        let is_scheme = scheme.len() > 1
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));

        if is_scheme {
            return match scheme.to_ascii_lowercase().as_str() {
                "http" | "https" => ImageSource::Remote(src.to_string()),
                "file" => {
                    let rest = rest.split(['?', '#']).next().unwrap_or("");
                    // file://host/path: only an empty host or localhost means this machine.
                    let path = match rest.strip_prefix("//") {
                        Some(authority_and_path) => {
                            let (host, path) = authority_and_path
                                .find('/')
                                .map(|i| authority_and_path.split_at(i))
                                .unwrap_or((authority_and_path, ""));
                            if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
                                return ImageSource::Unsupported;
                            }
                            path
                        }
                        None => rest,
                    };
                    // file: URLs are always absolute; file:a.png means /a.png.
                    let path = decode(path);
                    if path.starts_with('/') {
                        ImageSource::Local(PathBuf::from(path))
                    } else {
                        ImageSource::Local(PathBuf::from(format!("/{}", path)))
                    }
                }
                _ => ImageSource::Unsupported,
            };
        }
    }

    let path = PathBuf::from(decode(src.split(['?', '#']).next().unwrap_or("")));
    if path.is_absolute() {
        ImageSource::Local(path)
    } else {
        ImageSource::Local(base_dir.join(path))
    }
}

#[derive(clap::ValueEnum, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IncludeLevel {
    Local,
    Remote,
    All,
}

#[derive(Clone)]
pub struct RenderOptions {
    pub theme: Theme,
    pub highlight: bool,
    pub math: bool,
    pub diagrams: bool,
    pub live: bool,
    pub include_images: Option<IncludeLevel>,
    pub optimize_images: bool,
}

#[derive(Serialize)]
pub struct TemplateData {
    pub theme: String,
    pub highlight: bool,
    pub math: bool,
    pub diagrams: bool,
    pub compiled: String,
    pub title: String,
    pub websocket: String,
    pub script: String,
    pub live: bool,
}

impl Document {
    fn handlebars() -> Handlebars<'static> {
        let mut reg = Handlebars::new();
        let template_string = TEMPLATES_DIR
            .get_file("template.hbs")
            .expect("must be present")
            .contents_utf8()
            .expect("template must be a valid utf8");

        reg.register_template_string("html", template_string)
            .expect("must be a valid handlebars template");

        reg
    }

    pub fn render_body(&self) -> Result<String> {
        let markdown_options = markdown::Options {
            parse: markdown::ParseOptions {
                constructs: markdown::Constructs {
                    html_flow: true,
                    html_text: true,
                    math_flow: self.options.math,
                    math_text: self.options.math,
                    definition: true,
                    ..markdown::Constructs::gfm()
                },
                ..markdown::ParseOptions::gfm()
            },
            compile: markdown::CompileOptions {
                allow_dangerous_html: true,
                ..markdown::CompileOptions::gfm()
            },
        };

        let html = markdown::to_html_with_options(self.text.as_str(), &markdown_options)
            .expect("never errors with MDX disabled");

        if self.options.include_images.is_some() {
            self.include_images(html)
        } else {
            Ok(html)
        }
    }

    pub fn render(&self) -> Result<Vec<u8>> {
        let body = self.render_body()?;

        let script: String = {
            let mut minified_script = Vec::new();
            let script = VENDOR_DIR
                .get_file("js/script.js")
                .unwrap()
                .contents()
                .to_vec();

            let session = minify_js::Session::new();
            match minify_js::minify(
                &session,
                minify_js::TopLevelMode::Global,
                &script,
                &mut minified_script,
            ) {
                Ok(()) => String::from_utf8_lossy(minified_script.as_slice()).to_string(),
                Err(_) => String::from_utf8_lossy(script.as_slice()).to_string(),
            }
        };

        let html = Self::handlebars().render(
            "html",
            &TemplateData {
                theme: self.options.theme.resolve()?,
                highlight: self.options.highlight,
                math: self.options.math,
                diagrams: self.options.diagrams,
                compiled: body,
                title: self.title().unwrap_or("Document".into()),
                live: self.options.live,
                websocket: VENDOR_DIR
                    .get_file("js/reconnecting-websocket.js")
                    .unwrap()
                    .contents_utf8()
                    .unwrap()
                    .to_string(),
                script,
            },
        )?;

        Ok(html.into_bytes())
    }

    pub fn title(&self) -> Option<String> {
        match markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
            Ok(node) => Document::get_title_from_node(&node),
            Err(_) => None,
        }
    }

    fn get_title_from_node(node: &markdown::mdast::Node) -> Option<String> {
        match node {
            markdown::mdast::Node::Heading(_) => Some(node.to_string()),
            _ => match node.children() {
                Some(children) => {
                    for child in children.iter() {
                        if let Some(title) = Self::get_title_from_node(&child) {
                            return Some(title);
                        }

                        return None;
                    }

                    None
                }
                None => None,
            },
        }
    }

    fn include_images(&self, html_page: String) -> anyhow::Result<String> {
        let mut output = vec![];

        let mut rewriter = HtmlRewriter::new(
            Settings::new().append_element_content_handler(element!("img[src]", |el| {
                let src = el.get_attribute("src").expect("src was required");

                let data = {
                    let level = self.options.include_images.expect("must be not none");

                    let include_remote =
                        level == IncludeLevel::All || level == IncludeLevel::Remote;

                    let include_local = level == IncludeLevel::All || level == IncludeLevel::Local;

                    match classify_image_source(&decode_html_entities(&src), &self.base_dir) {
                        ImageSource::Remote(url) if include_remote => {
                            info!("Downloading {}", url);

                            if url.split(['?', '#']).next().unwrap_or("").ends_with(".svg") {
                                let svg_data = download_image(url.as_str())
                                    .map_err(|e| anyhow!("downloading {}: {}", url, e))?;
                                let base64_svg =
                                    base64::engine::general_purpose::STANDARD.encode(&svg_data);
                                el.set_attribute(
                                    "src",
                                    &format!("data:image/svg+xml;base64,{}", base64_svg),
                                )?;
                                None
                            } else {
                                Some(
                                    download_image(url.as_str())
                                        .map_err(|e| anyhow!("downloading {}: {}", url, e))?,
                                )
                            }
                        }
                        ImageSource::Local(path) if include_local => {
                            info!("Reading {}", path.display());

                            let is_svg = path.extension() == Some(OsStr::new("svg"));

                            if is_svg {
                                let svg_data = self
                                    .svg_to_base64(&path)
                                    .map_err(|e| anyhow!("reading {}: {}", path.display(), e))?;
                                el.set_attribute("src", &svg_data)?;
                                None
                            } else {
                                Some(
                                    fs::read(&path).map_err(|e| {
                                        anyhow!("reading {}: {}", path.display(), e)
                                    })?,
                                )
                            }
                        }
                        _ => {
                            info!("Skipping {}", src);
                            None
                        }
                    }
                };

                if let Some(data) = data {
                    info!("Encoding to base64",);
                    let img = image::load_from_memory(data.as_slice())
                        .map_err(|e| anyhow!("decoding {}: {}", src, e))?;
                    el.set_attribute("src", &self.image_to_base64(&img)?)?;
                }

                Ok(())
            })),
            |c: &[u8]| output.extend_from_slice(c),
        );

        rewriter.write(html_page.as_bytes())?;
        rewriter.end()?;

        Ok(String::from_utf8(output)?)
    }

    fn image_to_base64(&self, img: &DynamicImage) -> anyhow::Result<String> {
        let mut image_data: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut image_data), ImageFormat::Png)?;

        if self.options.optimize_images {
            info!("Optimizing image");
            match oxipng::optimize_from_memory(image_data.as_slice(), &oxipng::Options::default()) {
                Ok(optimized) => image_data = optimized,
                Err(e) => return Err(anyhow::Error::new(e)),
            }
        }

        let res_base64 = base64::engine::general_purpose::STANDARD.encode(image_data);
        Ok(format!("data:image/png;base64,{}", res_base64))
    }

    fn svg_to_base64(&self, path: &Path) -> anyhow::Result<String> {
        let svg_data = fs::read(path)?;
        let base64_svg = base64::engine::general_purpose::STANDARD.encode(svg_data);
        let svg_data_uri = format!("data:image/svg+xml;base64,{}", base64_svg);
        Ok(svg_data_uri)
    }
}

fn download_image(url: &str) -> anyhow::Result<Vec<u8>> {
    // Rendering runs on the tokio runtime, where reqwest's blocking client panics
    // unless the thread is marked as blocking.
    tokio::task::block_in_place(|| Ok(reqwest::blocking::get(url)?.bytes()?.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(p: &str) -> ImageSource {
        ImageSource::Local(PathBuf::from(p))
    }

    #[test]
    fn relative_paths_resolve_against_base_dir() {
        let base = Path::new("/docs");
        assert_eq!(
            classify_image_source("img/a.png", base),
            local("/docs/img/a.png")
        );
        assert_eq!(
            classify_image_source("../a.png", base),
            local("/docs/../a.png")
        );
        assert_eq!(
            classify_image_source("/abs/a.png", base),
            local("/abs/a.png")
        );
    }

    #[test]
    fn local_paths_are_percent_decoded_and_lose_query_and_fragment() {
        let base = Path::new("/docs");
        assert_eq!(
            classify_image_source("my%20img.png", base),
            local("/docs/my img.png")
        );
        assert_eq!(
            classify_image_source("dir/%C3%BC.png", base),
            local("/docs/dir/ü.png")
        );
        assert_eq!(
            classify_image_source("a.png?v=1#f", base),
            local("/docs/a.png")
        );
        assert_eq!(
            classify_image_source("file:///abs/my%20a.png", base),
            local("/abs/my a.png")
        );
        assert_eq!(
            classify_image_source("file://localhost/abs/a.png", base),
            local("/abs/a.png")
        );
        assert_eq!(
            classify_image_source("file://LOCALHOST/abs/a.png", base),
            local("/abs/a.png")
        );
        assert_eq!(
            classify_image_source("file:abs/a.png", base),
            local("/abs/a.png")
        );
        assert_eq!(
            classify_image_source("file://other/abs/a.png", base),
            ImageSource::Unsupported
        );
    }

    #[test]
    fn remote_and_unsupported_sources() {
        let base = Path::new("/docs");
        let remote = |u: &str| ImageSource::Remote(u.to_string());
        assert_eq!(
            classify_image_source("https://x.dev/a.png", base),
            remote("https://x.dev/a.png")
        );
        assert_eq!(
            classify_image_source("HTTP://x.dev/a.png", base),
            remote("HTTP://x.dev/a.png")
        );
        assert_eq!(
            classify_image_source("//x.dev/a.png", base),
            remote("https://x.dev/a.png")
        );
        assert_eq!(
            classify_image_source("httpfoo.png", base),
            local("/docs/httpfoo.png")
        );
        assert_eq!(classify_image_source("", base), ImageSource::Unsupported);
        assert_eq!(
            classify_image_source("mailto:a@b.c", base),
            ImageSource::Unsupported
        );
    }

    #[test]
    fn html_entities_are_decoded() {
        assert_eq!(decode_html_entities("a&amp;b.png"), "a&b.png");
        assert_eq!(decode_html_entities("x?a=1&amp;b=2"), "x?a=1&b=2");
        assert_eq!(
            decode_html_entities("&lt;&gt;&quot;&apos;&#39;&#x41;&#X42;"),
            "<>\"''AB"
        );
        assert_eq!(
            decode_html_entities("a&b; &unknown; &amp"),
            "a&b; &unknown; &amp"
        );
        assert_eq!(decode_html_entities("&#xD800;"), "&#xD800;");
    }

    #[test]
    fn base_dir_falls_back_to_current_directory() {
        assert_eq!(
            base_dir_for(Some(Path::new("/docs/a.md"))),
            PathBuf::from("/docs")
        );
        assert_eq!(
            base_dir_for(Some(Path::new("sub/a.md"))),
            PathBuf::from("sub")
        );
        assert_eq!(base_dir_for(Some(Path::new("a.md"))), PathBuf::from("."));
        assert_eq!(base_dir_for(None), PathBuf::from("."));
    }
}
