use crate::legit::process_red;
use captrs::Capturer;
use image::Rgb;
use image::RgbImage;
use rand::seq::SliceRandom;
use rand::Rng;

#[allow(unused_imports)]
pub(crate) use image::io::Reader as ImageReader;

use std::error::Error;

mod legit;

mod debug;
use debug::get_debug_spawner_clicks;
use debug::get_debug_worm_clicks;
use debug::remap_clicks_to_bb;
mod screen;
use screen::BoundingBox;
use screen::Coord;

const ARTY_REMOTE_RADIUS: u32 = 40;
const NUM_RANDOM_GUESSES: usize = 10;
const NUM_RANDOM_SAMPLES: usize = 1000;

fn gen_clicks_from_bbs_rand(bbs: &[BoundingBox], remote_radius: u32, w: u32, h: u32) -> Vec<Coord> {
    let mut rng = rand::thread_rng();

    let mut bbs: Vec<BoundingBox> = bbs.to_vec();
    let mut best_clicks = vec![];
    'guess_again: for _ in 0..NUM_RANDOM_GUESSES {
        bbs.shuffle(&mut rng);
        let mut bbs: Vec<BoundingBox> = bbs.clone();
        let mut current_clicks = vec![];
        while !bbs.is_empty() {
            let bb = bbs.pop().unwrap();
            // Default to click the corner
            let mut best_click = bb.left_top;
            let mut best_hits = 0;
            for _ in 0..NUM_RANDOM_SAMPLES {
                // Generate a new random click, that likely hits this bb
                let test_click = Coord {
                    w: rng
                        .gen_range(
                            (bb.left_top.w - remote_radius as i64)
                                ..=(bb.right_bottom.w + remote_radius as i64),
                        )
                        .clamp(0, w as i64),
                    h: rng
                        .gen_range(
                            (bb.left_top.h - remote_radius as i64)
                                ..=(bb.right_bottom.h + remote_radius as i64),
                        )
                        .clamp(0, h as i64),
                };
                if bb.collides_with_circle(test_click, remote_radius) {
                    // We must hit self, regardless of how many other bbs we might hit
                    let bbs = &bbs;
                    let hits = count_collisions_single(&bbs, remote_radius, test_click) + 1;
                    if hits > best_hits {
                        best_click = test_click;
                        best_hits = hits;
                    }
                }
            }
            current_clicks.push(best_click);
            if current_clicks.len() > best_clicks.len() && !best_clicks.is_empty() {
                continue 'guess_again;
            }
            // Remove anything hit by the most recent click
            bbs.retain(|bb| {
                if let Some(click) = current_clicks.last() {
                    if bb.collides_with_circle(*click, remote_radius) {
                        return false;
                    }
                }
                true
            });
        }

        if current_clicks.len() < best_clicks.len()
            || (best_clicks.is_empty() && !current_clicks.is_empty())
        {
            println!(
                "Updating {} with {}",
                best_clicks.len(),
                current_clicks.len()
            );
            best_clicks = current_clicks;
        }
    }
    best_clicks
}

fn count_collisions_single(bbs: &[BoundingBox], remote_radius: u32, click: Coord) -> usize {
    let mut ct = 0;
    for bb in bbs {
        if !bb.tagged {
            if bb.collides_with_circle(click, remote_radius) {
                ct += 1;
            }
        }
    }
    ct
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Spawned");
    let mut img = capture_image();
    println!("Got image");

    let clicks = get_debug_spawner_clicks(&img);
    let worm_clicks = get_debug_worm_clicks(&img);

    //img.save("blah.png")?;

    let spawner_mask = BoundingBox {
        tagged: false,
        left_top: Coord { w: -30, h: -11 },
        right_bottom: Coord { w: 23, h: 31 },
    };
    let spawner_bbs = remap_clicks_to_bb(&clicks, &spawner_mask, &mut img);

    let worm_mask = BoundingBox {
        tagged: false,
        left_top: Coord { w: -8, h: 1 },
        right_bottom: Coord { w: 15, h: 22 },
    };
    let worm_bbs = remap_clicks_to_bb(&worm_clicks, &worm_mask, &mut img);
    let mut combined_bbs = spawner_bbs.clone();
    combined_bbs.extend(&worm_bbs);

    let mut sorted_combined_bbs = combined_bbs.clone();
    sorted_combined_bbs.sort_by(|s, other| {
        let res = if s.left_top.h == other.left_top.h {
            s.left_top.w.cmp(&other.left_top.h)
        } else {
            s.left_top.h.cmp(&other.left_top.h)
        };
        res
    });

    let random = gen_clicks_from_bbs_rand(
        &sorted_combined_bbs,
        ARTY_REMOTE_RADIUS,
        img.width(),
        img.height(),
    );
    println!("Proccing red");
    let (bbs, spawner_width) = process_red(&mut img);
    let red_clicks = gen_clicks_from_bbs_rand(
        &bbs,
        (spawner_width as f64 * 0.50) as u32,
        img.width(),
        img.height(),
    );
    println!(
        "{} targets, {} clicks with red clicker",
        bbs.len(),
        red_clicks.len()
    );

    println!(
        "{} targets, {} clicks with random deduper",
        combined_bbs.len(),
        random.len()
    );
    click_arty(&red_clicks)?;

    Ok(())
}

#[allow(dead_code)]
fn click_arty(clicks: &[Coord]) -> Result<(), Box<(dyn Error)>> {
    let mut cmd = String::new();
    for click in clicks {
        cmd.push_str(&format!("mousemove {} {} click 1 ", click.w, click.h));
    }
    println!("{}", cmd);
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

    let i = RgbImage::from_raw(geometry.0, geometry.1, bytes).unwrap();
    //i.save("red.png").unwrap();

    /*let i = ImageReader::open("zoom/z2.png")
        .unwrap()
        .decode()
        .unwrap()
        .to_rgb8();*/
    i
}

fn draw_bbs(bbs: &[BoundingBox]) {
    let mut i = RgbImage::new(1920, 1080);
    for bb in bbs {
        let shift = bb.area() as u8;
        for w in bb.left_top.w..=bb.right_bottom.w {
            if w < 0 || bb.left_top.h < 0 {
                continue;
            }
            if w as u32 >= 1920 || bb.right_bottom.h as u32 >= 1080 {
                continue;
            }
            i.put_pixel(w as u32, bb.left_top.h as u32, Rgb([shift, shift, 0xff]));
            i.put_pixel(
                w as u32,
                bb.right_bottom.h as u32,
                Rgb([shift, shift, 0xff]),
            );
        }
        for h in bb.left_top.h..=bb.right_bottom.h {
            if bb.right_bottom.w < 0 || h < 0 {
                continue;
            }
            if bb.right_bottom.w as u32 >= 1920 || h as u32 >= 1080 {
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

#[cfg(test)]
mod tests {
    use crate::debug::get_debug_enemy_clicks;
    use crate::legit::process_red;
    use crate::ImageReader;

    #[test]
    fn test_scan_rects() {
        let mut i = ImageReader::open("zoom/z10.png")
            .unwrap()
            .decode()
            .unwrap()
            .into_rgb8();
        let bbs = process_red(&mut i);
        println!("Found {} targets from red", bbs.0.len());
    }

    #[test]
    fn test_img_readstuff() {
        let i = ImageReader::open("zoom/z1.png")
            .unwrap()
            .decode()
            .unwrap()
            .to_rgb8();

        let clicks = get_debug_enemy_clicks(&i);
        println!("Found {} enemy targets", clicks.len());
    }
}
