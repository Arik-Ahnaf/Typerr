//! Bounded image loading, crop geometry, and atomic per-user avatar storage.
use iced::{Point, Rectangle, Vector, widget::image::Handle};
use image::{ImageDecoder, ImageFormat, ImageReader, RgbaImage};
use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, mpsc},
};

const MAX_BYTES: u64 = 20 * 1024 * 1024;
const CROP_SIZE: f32 = 100.;

#[derive(Clone, Debug)]
pub struct Source {
    pixels: Arc<RgbaImage>,
    pub handle: Handle,
}
impl Source {
    fn new(pixels: RgbaImage) -> Self {
        let handle = Handle::from_rgba(pixels.width(), pixels.height(), pixels.as_raw().clone());
        Self {
            pixels: Arc::new(pixels),
            handle,
        }
    }
    pub fn size(&self) -> (f32, f32) {
        (self.pixels.width() as f32, self.pixels.height() as f32)
    }
}

fn decode(bytes: &[u8]) -> Result<Source, String> {
    let format = image::guess_format(bytes).map_err(|_| "Choose a PNG or JPG image.")?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return Err("Choose a PNG or JPG image.".into());
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| format!("Cannot read image: {e}"))?;
    let orientation = decoder
        .orientation()
        .map_err(|e| format!("Cannot read image: {e}"))?;
    let mut image = image::DynamicImage::from_decoder(decoder)
        .map_err(|e| format!("Cannot read image: {e}"))?;
    image.apply_orientation(orientation);
    // Bound the working copy while retaining ample resolution for a 512px avatar.
    if image.width().max(image.height()) > 2048 {
        image = image.resize(2048, 2048, image::imageops::FilterType::Lanczos3);
    }
    Ok(Source::new(image.to_rgba8()))
}

pub fn read(path: &Path) -> Result<Source, String> {
    let file = fs::File::open(path).map_err(|e| format!("Cannot open image: {e}"))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Choose an image file.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Cannot read image: {e}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Choose an image smaller than 20 MiB.".into());
    }
    decode(&bytes)
}

pub fn placeholder() -> Source {
    static DEFAULT: OnceLock<Source> = OnceLock::new();
    DEFAULT
        .get_or_init(|| {
            decode(include_bytes!("../assets/icons/profile-placeholder.jpg"))
                .expect("bundled avatar")
        })
        .clone()
}

#[derive(Clone, Debug)]
pub struct Crop {
    pub source: Source,
    pub zoom: f32,
    pub offset: Vector,
}
impl Crop {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            zoom: 1.,
            offset: Vector::ZERO,
        }
    }
    pub fn scale(&self) -> f32 {
        let (w, h) = self.source.size();
        CROP_SIZE / w.min(h) * self.zoom
    }
    pub fn set_zoom(&mut self, zoom: f32) {
        if !zoom.is_finite() {
            return;
        }
        let next = zoom.clamp(1., 4.);
        self.offset *= next / self.zoom;
        self.zoom = next;
        self.clamp();
    }
    pub fn pan(&mut self, offset: Vector) {
        if offset.x.is_finite() && offset.y.is_finite() {
            self.offset = offset;
            self.clamp();
        }
    }
    fn clamp(&mut self) {
        let (w, h) = self.source.size();
        let scale = self.scale();
        let x = ((w * scale - CROP_SIZE) / 2.).max(0.);
        let y = ((h * scale - CROP_SIZE) / 2.).max(0.);
        self.offset.x = self.offset.x.clamp(-x, x);
        self.offset.y = self.offset.y.clamp(-y, y);
    }
    pub fn bounds(&self, center: Point) -> Rectangle {
        let (w, h) = self.source.size();
        let scale = self.scale();
        Rectangle {
            x: center.x + self.offset.x - w * scale / 2.,
            y: center.y + self.offset.y - h * scale / 2.,
            width: w * scale,
            height: h * scale,
        }
    }
    pub fn output(&self) -> RgbaImage {
        let bounds = self.bounds(Point::ORIGIN);
        let side = CROP_SIZE / self.scale();
        let (width, height) = self.source.size();
        let left = (-50. - bounds.x) / self.scale();
        let top = (-50. - bounds.y) / self.scale();
        // Sample the exact fractional preview rectangle, even for tiny images.
        RgbaImage::from_fn(512, 512, |x, y| {
            let u = ((left + (x as f32 + 0.5) * side / 512.) / width).clamp(0., 1.);
            let v = ((top + (y as f32 + 0.5) * side / 512.) / height).clamp(0., 1.);
            image::imageops::sample_bilinear(&*self.source.pixels, u, v)
                .expect("nonempty image and bounded crop")
        })
    }
}

pub fn circular(pixels: &RgbaImage) -> Handle {
    let mut pixels = pixels.clone();
    let radius = pixels.width() as f32 / 2.;
    for (x, y, pixel) in pixels.enumerate_pixels_mut() {
        let distance =
            ((x as f32 + 0.5 - radius).powi(2) + (y as f32 + 0.5 - radius).powi(2)).sqrt();
        pixel[3] = (pixel[3] as f32 * (radius - distance + 0.5).clamp(0., 1.)) as u8;
    }
    Handle::from_rgba(pixels.width(), pixels.height(), pixels.into_raw())
}

pub fn save(path: &Path, crop: &Crop) -> Result<Handle, String> {
    let pixels = crop.output();
    let parent = path.parent().ok_or("No profile directory is available.")?;
    fs::create_dir_all(parent).map_err(|e| format!("Cannot save image: {e}"))?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| format!("Cannot save image: {e}"))?;
    image::DynamicImage::ImageRgba8(pixels.clone())
        .write_to(&mut temporary, ImageFormat::Png)
        .map_err(|e| format!("Cannot save image: {e}"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| format!("Cannot save image: {e}"))?;
    temporary
        .persist(path)
        .map_err(|e| format!("Cannot save image: {e}"))?;
    Ok(circular(&pixels))
}

enum Reply {
    Chosen(Result<Option<Source>, String>),
    Saved(Result<Handle, String>),
}
pub struct Picture {
    pub avatar: Handle,
    pub crop: Option<Crop>,
    pub error: Option<String>,
    pub focus: usize,
    path: Option<PathBuf>,
    saving: bool,
    reply: Option<mpsc::Receiver<Reply>>,
}
impl Default for Picture {
    fn default() -> Self {
        static AVATAR: OnceLock<Handle> = OnceLock::new();
        Self {
            avatar: AVATAR
                .get_or_init(|| circular(&Crop::new(placeholder()).output()))
                .clone(),
            crop: None,
            error: None,
            focus: 3,
            path: None,
            saving: false,
            reply: None,
        }
    }
}
impl Picture {
    pub fn load(path: Option<PathBuf>) -> Self {
        let mut picture = Self {
            path,
            ..Self::default()
        };
        if let Some(path) = picture.path.as_ref().filter(|p| p.exists()) {
            match read(path) {
                Ok(source) => picture.avatar = circular(&Crop::new(source).output()),
                Err(error) => eprintln!("Profile picture: {error}"),
            }
        }
        picture
    }
    pub fn saving(&self) -> bool {
        self.saving
    }
    pub fn busy(&self) -> bool {
        self.reply.is_some()
    }
    pub fn modal(&self) -> bool {
        self.busy() || self.crop.is_some() || self.error.is_some()
    }
    pub fn choose(&mut self) {
        if self.modal() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.reply = Some(rx);
        std::thread::spawn(move || {
            let result = rfd::FileDialog::new()
                .set_title("Choose profile picture")
                .add_filter(
                    "PNG or JPG image",
                    &["png", "jpg", "jpeg", "PNG", "JPG", "JPEG"],
                )
                .pick_file()
                .map(|path| read(&path))
                .transpose();
            let _ = tx.send(Reply::Chosen(result));
        });
    }
    pub fn discard(&mut self) {
        if !self.busy() {
            self.crop = None;
            self.error = None;
            self.focus = 3;
        }
    }
    pub fn save(&mut self) {
        if self.busy() {
            return;
        }
        let Some(crop) = self.crop.clone() else {
            return;
        };
        let Some(path) = self.path.clone() else {
            self.error = Some("No user configuration directory is available.".into());
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.reply = Some(rx);
        self.error = None;
        self.saving = true;
        std::thread::spawn(move || {
            let _ = tx.send(Reply::Saved(save(&path, &crop)));
        });
    }
    pub fn poll(&mut self) {
        let Some(rx) = &self.reply else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(reply) => reply,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.reply = None;
                self.saving = false;
                self.error = Some("Image operation could not finish. Please try again.".into());
                return;
            }
        };
        self.reply = None;
        self.saving = false;
        match result {
            Reply::Chosen(Ok(source)) => {
                self.crop = source.map(Crop::new);
                self.focus = 3;
            }
            Reply::Saved(Ok(avatar)) => {
                self.avatar = avatar;
                self.discard();
            }
            Reply::Chosen(Err(error)) | Reply::Saved(Err(error)) => self.error = Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(width: u32, height: u32) -> Source {
        Source::new(RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 100, 255])
        }))
    }
    #[test]
    fn crop_covers_circle_at_every_zoom_and_pan_boundary() {
        for (width, height) in [(400, 200), (200, 400), (200, 200), (1, 1000), (1000, 1)] {
            let mut crop = Crop::new(source(width, height));
            for zoom in [1., 1.03, 2., 3.99, 4.] {
                crop.set_zoom(zoom);
                for offset in [
                    Vector::new(-10000., -10000.),
                    Vector::new(10000., 10000.),
                    Vector::ZERO,
                ] {
                    crop.pan(offset);
                    let bounds = crop.bounds(Point::ORIGIN);
                    assert!(bounds.x <= -50. + 0.001 && bounds.y <= -50. + 0.001);
                    assert!(
                        bounds.x + bounds.width >= 50. - 0.001
                            && bounds.y + bounds.height >= 50. - 0.001
                    );
                    if width == 1 || height == 1 {
                        let output = crop.output();
                        assert_eq!(output.dimensions(), (512, 512));
                        assert!(output.pixels().all(|pixel| pixel[3] == 255));
                    }
                }
            }
            crop.set_zoom(f32::NAN);
            crop.pan(Vector::new(f32::INFINITY, 0.));
            assert!(crop.zoom.is_finite() && crop.offset.x.is_finite());
        }
    }
    #[test]
    fn pan_changes_saved_pixels_and_zoom_preserves_source_center() {
        let mut crop = Crop::new(source(400, 200));
        let initial = crop.output();
        crop.pan(Vector::new(40., 0.));
        let panned = crop.output();
        assert_ne!(initial, panned);
        let center = crop.offset / crop.scale();
        crop.set_zoom(2.);
        assert_eq!(crop.offset / crop.scale(), center);
        assert_ne!(crop.output(), panned);
    }
    #[test]
    fn png_jpeg_and_atomic_save_roundtrip_without_original_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpg");
        image::DynamicImage::ImageRgba8(source(200, 300).pixels.as_ref().clone())
            .to_rgb8()
            .save(&input)
            .unwrap();
        let crop = Crop::new(read(&input).unwrap());
        let destination = dir.path().join("config/avatar.png");
        save(&destination, &crop).unwrap();
        fs::remove_file(input).unwrap();
        let loaded = read(&destination).unwrap();
        assert_eq!(loaded.size(), (512., 512.));
        assert_eq!(*loaded.pixels, crop.output());
        let before = fs::read(&destination).unwrap();
        let mut picture = Picture::load(Some(destination.clone()));
        let avatar = picture.avatar.clone();
        picture.crop = Some(Crop::new(source(400, 300)));
        picture.discard();
        assert_eq!(picture.avatar, avatar);
        assert_eq!(fs::read(destination).unwrap(), before);
    }
    #[test]
    fn invalid_formats_dimensions_and_failed_save_preserve_current_avatar() {
        assert!(decode(b"not an image").is_err());
        assert!(decode(b"GIF89a").is_err());
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(RgbaImage::new(8193, 1))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        assert!(decode(bytes.get_ref()).is_err());
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("file");
        fs::write(&blocker, b"keep").unwrap();
        assert!(save(&blocker.join("avatar.png"), &Crop::new(placeholder())).is_err());
        assert_eq!(fs::read(&blocker).unwrap(), b"keep");
        let mut picture = Picture::default();
        let avatar = picture.avatar.clone();
        picture.crop = Some(Crop::new(placeholder()));
        picture.save();
        assert!(picture.error.is_some() && picture.crop.is_some());
        assert_eq!(picture.avatar, avatar);
    }
    #[test]
    fn worker_cancel_failure_and_success_are_distinct_and_not_repeated() {
        let mut picture = Picture::default();
        let original = picture.avatar.clone();
        let (tx, rx) = mpsc::channel();
        picture.reply = Some(rx);
        picture.poll();
        assert!(picture.busy());
        tx.send(Reply::Chosen(Ok(None))).unwrap();
        picture.poll();
        assert!(!picture.modal());
        assert_eq!(picture.avatar, original);
        let (tx, rx) = mpsc::channel();
        picture.reply = Some(rx);
        tx.send(Reply::Chosen(Err("bad image".into()))).unwrap();
        picture.poll();
        assert!(picture.modal());
        assert!(picture.crop.is_none());
        picture.discard();
        let (tx, rx) = mpsc::channel();
        picture.reply = Some(rx);
        tx.send(Reply::Chosen(Ok(Some(placeholder())))).unwrap();
        picture.poll();
        assert!(picture.crop.is_some());
        assert!(!picture.busy());
        let dir = tempfile::tempdir().unwrap();
        picture.path = Some(dir.path().join("avatar.png"));
        picture.save();
        picture.discard();
        assert!(picture.busy());
        for _ in 0..100 {
            picture.poll();
            if !picture.busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!picture.modal());
        assert!(dir.path().join("avatar.png").exists());
        assert_ne!(picture.avatar, original);
    }
}
