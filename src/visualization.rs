use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::File;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;

use image::Delay;
use image::DynamicImage;
use image::Frame;
use image::ImageResult;
use image::Pixel;
use image::Rgb;
use image::RgbImage;
use image::Rgba;
use image::RgbaImage;
use image::gif::GifEncoder;

use webp_animation::Encoder;

pub struct VisualizedImage {
    img: RgbImage,
    orig_image: RgbaImage,
    read_indices: RefCell<VecDeque<(u32, u32)>>,
    frames: RefCell<Vec<Frame>>,
    toggle_frame_collect: AtomicBool,
}

impl VisualizedImage {
    pub fn new(img: RgbImage) -> Self {
        let orig_image = DynamicImage::ImageRgb8(img.clone());
        Self {
            img,
            orig_image: orig_image.into_rgba8(),
            read_indices: RefCell::new(VecDeque::with_capacity(100)),
            frames: RefCell::new(Vec::new()),
            toggle_frame_collect: AtomicBool::new(false),
        }
    }

    pub fn get_pixel_mut(&mut self, w: u32, h: u32) -> &mut Rgb<u8> {
        self.increment(w, h);
        self.img.get_pixel_mut(w, h)
    }

    pub fn get_pixel(&self, w: u32, h: u32) -> &Rgb<u8> {
        self.increment(w, h);
        self.img.get_pixel(w, h)
    }


    /// Get the pixel or return a default pixel one tick from pure black
    pub fn get_pixel_default(&self, w: i64, h: i64) -> &Rgb<u8> {
        self.get_pixel_checked(w, h).unwrap_or(&Rgb([1; 3]))
    }

    pub fn get_pixel_checked(&self, w: i64, h: i64) -> Option<&Rgb<u8>> {
        if w < 0 || h < 0 {
            return None;
        }
        if w as u32 >= self.img.dimensions().0 || h as u32 >= self.img.dimensions().1 {
            return None;
        }
        Some(self.get_pixel(w as u32, h as u32))
    }

    pub fn put_pixel(&mut self, w: u32, h: u32, px: Rgb<u8>) {
        self.img.put_pixel(w, h, px)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.img.dimensions()
    }

    pub fn width(&self) -> u32 {
        self.img.width()
    }

    pub fn height(&self) -> u32 {
        self.img.height()
    }

    pub fn save(&self, filename: &str) -> ImageResult<()> {
        self.img.save(filename)
    }

    pub fn toggle_frame_collect(&self) {
        let toggle = self.toggle_frame_collect.fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
        if toggle {
            // init encoder. uses by default lossless encoding,
            // for other alternatives see documentation about
            // `new_with_options`
            let mut encoder = Encoder::new(self.dimensions()).unwrap();
            for (i, frame) in self.frames.take().into_iter().enumerate() {
                encoder.add_frame(&frame.into_buffer().into_raw(), i as i32);
            }

            // get encoded webp data
            let final_timestamp = 1_000;
            let webp_data = encoder.finalize(500).unwrap();
            std::fs::write("my_animation.webp", webp_data).unwrap();

            panic!();
        }
    }

    fn increment(&self, w: u32, h: u32) {
        if self.toggle_frame_collect.load(std::sync::atomic::Ordering::Relaxed) {
            let mut read_indices = self.read_indices.borrow_mut();
            if read_indices.len() == read_indices.capacity() {
                read_indices.pop_front();
            }
            read_indices.push_back((w, h));
            let mut frames = self.frames.borrow_mut();
            let mut img = self.orig_image.clone();
            for (w, h) in read_indices.iter().rev() {
                img.put_pixel(*w, *h, Rgba([255, 255, 255, 255]));
            }
            frames.push(Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(20, 1)));
        }
    }
}
