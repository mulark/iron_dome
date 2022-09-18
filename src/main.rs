use crate::legit::process_red;
use captrs::Capturer;
use image::Rgb;
use image::RgbImage;

pub(crate) use image::io::Reader as ImageReader;

use std::error::Error;

mod legit;

mod debug;
use debug::find_spawner_positions;
use debug::find_worm_positions;
use debug::remap_positions_to_bb;

mod generator;
use generator::gen_clicks_from_bbs_fixed;
use generator::gen_clicks_from_bbs_rand;

mod screen;
use screen::BoundingBox;
use screen::Coord;

const SCREEN_W: i64 = 2560;
const SCREEN_H: i64 = 1440;

const ARTY_REMOTE_RADIUS: u32 = 40;

#[derive(Debug)]
struct Gui {
    scan_debug: bool,
    scan_red: bool,
}

impl Default for Gui {
    fn default() -> Self {
        Gui {
            scan_debug: true,
            scan_red: true,
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.checkbox(&mut self.scan_debug, "Scan Debug");
            ui.checkbox(&mut self.scan_red, "Scan Red");
            let butt = egui::Button::new("Shoot");
            //let butt = butt.fill(egui::Rgba::from_rgb(0.6, 0.2, 0.2));
            let butt = ui.add_sized(egui::vec2(84.3, 42.3), butt);
            if butt.clicked() {
                let debug = self.scan_debug;
                let red = self.scan_red;
                let img = capture_image();
                let clicks = process_image_into_clicks(img, debug, red);
                click_arty(&clicks).unwrap();
            }
        });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut static_images = false;
    for arg in std::env::args().skip(1) {
        let now = std::time::Instant::now();
        let img = ImageReader::open(arg)?.decode()?;
        let img = img.into_rgb8();
        let clicks = process_image_into_clicks(img, true, true);
        println!(
            "Image processing took {:?} and generated {} clicks",
            now.elapsed(),
            clicks.len()
        );
        // If user provides list of images, don't run the normal gui
        static_images = true;
    }
    if !static_images {
        let mut options = eframe::NativeOptions::default();
        options.initial_window_size = Some(egui::vec2(100., 100.));
        eframe::run_native(
            "Iron Dome",
            options,
            Box::new(|_cc| Box::new(Gui::default())),
        );
    }
    Ok(())
}

fn process_image_into_clicks(mut img: RgbImage, scan_debug: bool, scan_red: bool) -> Vec<Coord> {
    if scan_debug {
        let mut debug_img = img.clone();

        let debug_spawner_positions = find_spawner_positions(&debug_img);
        let debug_worm_positions = find_worm_positions(&debug_img);

        let spawner_mask = BoundingBox {
            left_top: Coord { w: -30, h: -11 },
            right_bottom: Coord { w: 23, h: 31 },
        };
        let spawner_bbs =
            remap_positions_to_bb(&debug_spawner_positions, &spawner_mask, &mut debug_img);

        let worm_mask = BoundingBox {
            left_top: Coord { w: -8, h: 1 },
            right_bottom: Coord { w: 15, h: 22 },
        };
        let worm_bbs = remap_positions_to_bb(&debug_worm_positions, &worm_mask, &mut debug_img);
        let mut combined_bbs = spawner_bbs;
        combined_bbs.extend(&worm_bbs);
        combined_bbs.sort_by(|s, other| {
            let res = if s.left_top.h == other.left_top.h {
                s.left_top.w.cmp(&other.left_top.h)
            } else {
                s.left_top.h.cmp(&other.left_top.h)
            };
            res
        });

        let mut debug_clicks =
            gen_clicks_from_bbs_rand(&combined_bbs, ARTY_REMOTE_RADIUS, img.width(), img.height());
        remove_clicks_in_excluded_areas(&mut debug_clicks);
        println!(
            "Debug found {} targets, generated {} clicks",
            combined_bbs.len(),
            debug_clicks.len()
        );
        if !debug_clicks.is_empty() {
            return debug_clicks;
        }
    }
    if scan_red {
        let (bbs, spawner_width) = process_red(&mut img);
        let mut red_clicks = gen_clicks_from_bbs_rand(
            &bbs,
            (spawner_width as f64 * 0.43) as u32,
            img.width(),
            img.height(),
        );
        /*let mut red_clicks = gen_clicks_from_bbs_fixed(
            &bbs,
            (spawner_width as f64 * 0.43) as u32,
            img.width(),
            img.height(),
        );*/
        remove_clicks_in_excluded_areas(&mut red_clicks);
        //remove_clicks_in_excluded_areas(&mut alt_red_clicks);
        //dbg!(alt_red_clicks.len());
        red_clicks.sort_by(|first, second| first.h.cmp(&(second.h)));
        println!(
            "Red found {} targets, generated {} clicks",
            bbs.len(),
            red_clicks.len()
        );
        return red_clicks;
    }
    vec![]
}

fn remove_clicks_in_excluded_areas(clicks: &mut Vec<Coord>) {
    clicks
        .retain(|click| !(click.h < (493 * SCREEN_H / 1080) && click.w > (1664 * SCREEN_W / 1920)));
    clicks.retain(|click| {
        !(click.h > (985 * SCREEN_H / 1080)
            && click.w > (703 * SCREEN_W / 1920)
            && click.w < (1433 * SCREEN_W / 1920))
    });
}

#[allow(dead_code)]
fn click_arty(clicks: &[Coord]) -> Result<(), Box<(dyn Error)>> {
    let mut cmd = String::new();
    for click in clicks {
        cmd.push_str(&format!("mousemove {} {} click 1 ", click.w, click.h));
    }

    let _o = std::process::Command::new("xdotool")
        .args(cmd.split_whitespace())
        .output()?;
    Ok(())
}

fn capture_image() -> RgbImage {
    let mut c = Capturer::new(0).unwrap();

    let bytes = c.capture_frame().unwrap();
    let bytes = bytes
        .into_iter()
        .map(|bgr| [bgr.r, bgr.g, bgr.b])
        .flatten()
        .collect::<Vec<u8>>();
    let geometry = c.geometry();

    RgbImage::from_raw(geometry.0, geometry.1, bytes).unwrap()
}

#[allow(dead_code)]
fn draw_bbs(bbs: &[BoundingBox]) {
    let mut i = RgbImage::new(SCREEN_W as u32, SCREEN_H as u32);
    for bb in bbs {
        if bb.left_top.w < 0 || bb.left_top.h < 0 {
            continue;
        }
        if bb.right_bottom.w >= SCREEN_W || bb.right_bottom.h >= SCREEN_H {
            continue;
        }
        if bb.left_top.w >= SCREEN_W || bb.left_top.h >= SCREEN_H {
            continue;
        }
        let shift = bb.area() as u8;
        for w in bb.left_top.w..bb.right_bottom.w {
            if w >= SCREEN_W {
                continue;
            }
            i.put_pixel(w as u32, bb.left_top.h as u32, Rgb([shift, shift, 0xff]));
            i.put_pixel(
                w as u32,
                bb.right_bottom.h as u32,
                Rgb([shift, shift, 0xff]),
            );
        }
        for h in bb.left_top.h..bb.right_bottom.h {
            if h >= SCREEN_H {
                continue;
            }
            i.put_pixel(bb.left_top.w as u32, h as u32, Rgb([shift, shift, 0xff]));
            i.put_pixel(
                bb.right_bottom.w as u32,
                h as u32,
                Rgb([shift, shift, 0xff]),
            );
        }
    }
    i.save("bbs.png").unwrap();
}
