//! Module for the debug zoom levels

use crate::screen::Coord;
use crate::BoundingBox;
use image::Rgb;
use image::RgbImage;

/*
Blue pixel to collision box
X -30, +23
Y -11, +31

Arty remote radius
40
*/

/// Size in pixels of a spawner blue debug circle. Same size regardless of zoom level.
const SPAWNER_DEBUG_MIN_CLICK_DIST: i64 = 30;
/// Size in pixels of a worm magenta debug circle. Same size regardless of zoom level.
const WORM_DEBUG_MIN_CLICK_DIST: i64 = 30;

pub fn find_worm_positions(img: &RgbImage) -> Vec<Coord> {
    find_debug_positions(img, WORM_DEBUG_MIN_CLICK_DIST, Rgb([0xff, 0, 0xff]))
}

pub fn find_spawner_positions(img: &RgbImage) -> Vec<Coord> {
    find_debug_positions(img, SPAWNER_DEBUG_MIN_CLICK_DIST, Rgb([0, 0, 0xff]))
}

/// Generate clicks based on a image. Clicks must be spread min_click_dist apart
/// pixel_classification function to find pixels of desired color to click
fn find_debug_positions(img: &RgbImage, min_click_dist: i64, pixel_color: Rgb<u8>) -> Vec<Coord> {
    let mut clicks: Vec<Coord> = Vec::new();

    'next_px: for (w, h, px) in img.enumerate_pixels() {
        // Only add clicks if they're far enough away from other clicks
        for click in clicks.iter() {
            if i64::abs(w as i64 - click.w) < min_click_dist
                && i64::abs(h as i64 - click.h) < min_click_dist
            {
                continue 'next_px;
            }
        }
        if pixel_color == *px {
            clicks.push(Coord {
                w: w as i64,
                h: h as i64,
            });
        }
    }

    clicks
}

/// Reformat a set of clicks into a list of bounding boxes
pub fn remap_positions_to_bb(
    clicks: &[Coord],
    bb_mask: &BoundingBox,
    img: &mut RgbImage,
) -> Vec<BoundingBox> {
    let bounding_boxes = clicks
        .iter()
        .map(|click| BoundingBox {
            left_top: Coord {
                w: click.w + bb_mask.left_top.w,
                h: click.h + bb_mask.left_top.h,
            },
            right_bottom: Coord {
                w: click.w + bb_mask.right_bottom.w,
                h: click.h + bb_mask.right_bottom.h,
            },
        })
        .collect::<Vec<BoundingBox>>();

    for bb in bounding_boxes.iter() {
        for w in bb.left_top.w..=bb.right_bottom.w {
            if w < 0 {
                continue;
            }
            if w as u32 >= img.width() {
                continue;
            }

            // If we are off the screen to the left
            if 0 - bb.left_top.w > 0 {}

            for h in bb.left_top.h..=bb.right_bottom.h {
                if h < 0 {
                    continue;
                }
                if h as u32 >= img.height() {
                    continue;
                }

                // If we are off the screen at the top
                if 0 - bb.left_top.h > 0 {
                    // Then don't consider as many pixels down
                    if bb.right_bottom.h - (0 - bb.left_top.h) < h {
                        continue;
                    }
                    // We will also be biased to the left (due to the way we scan for clicks, and scanning the blue circle)
                    // So therefore ignore some left pixels as well
                    if bb.left_top.w + 8 > w {
                        continue;
                    }
                }
                let px = img.get_pixel_mut(w as u32, h as u32);
                if px[0] == 255 && px[1] == 0 && px[2] == 0 {
                    continue;
                }
                *px = Rgb([0, 255, 0]);
            }
        }
    }

    bounding_boxes
}
