//! Module to generate clicks from a list of bounding boxes

use std::sync::{Arc, Mutex};
use tiny_rng::{Rand, Rng};

use crate::screen::BoundingBox;
use crate::screen::Coord;

const NUM_RANDOM_GUESSES: usize = 10;
const NUM_RANDOM_SAMPLES: usize = 1000;

/// Generate clicks from a set of bounding boxes
/// bbs: a slice of BoundingBox objects to target
/// remote_radius: estimated size of the artillery remote target area (in pixels)
/// w: width of the image in pixels (to keep generated clicks in bounds)
/// h: height of the image in pixels (to keep generated clicks in bounds)
pub fn gen_clicks_from_bbs_rand(
    bbs: &[BoundingBox],
    remote_radius: u32,
    w: u32,
    h: u32,
) -> Vec<Coord> {
    let clicks: Arc<Mutex<Vec<Vec<Coord>>>> = Arc::new(Mutex::new(vec![]));

    println!(
        "{} * {} = {}",
        std::mem::size_of::<BoundingBox>(),
        bbs.len(),
        std::mem::size_of::<BoundingBox>() * bbs.len()
    );

    let it = std::time::Instant::now();

    std::thread::scope(|scope| {
        let mut returns = vec![];
        for id in 0..NUM_RANDOM_GUESSES {
            let mut bbs: Vec<BoundingBox> = bbs.to_vec();
            let clicks = clicks.clone();

            returns.push(scope.spawn(move || {
                let mut rng = tiny_rng::Rng::from_seed(id as u64);
                let mut current_clicks = vec![];
                while !bbs.is_empty() {
                    let bb = bbs.pop().unwrap();
                    // Default to click the corner
                    let mut best_click = bb.left_top;
                    let mut best_hits = 0;
                    for _ in 0..NUM_RANDOM_SAMPLES {
                        // Generate a new random click, that likely hits this bb
                        let test_click = get_rand_click(&mut rng, &bb, remote_radius, w, h);
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
                    /*if current_clicks.len() > best_clicks.len() && !best_clicks.is_empty() {
                        break;
                    }*/
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
                clicks.lock().unwrap().push(current_clicks);
            }));
        }
    });

    // Take the found clicks out
    let v = Arc::try_unwrap(clicks).unwrap().into_inner().unwrap();
    let mut counts = v.iter().map(|bucket| bucket.len()).collect::<Vec<_>>();
    counts.sort();

    println!("Threads found {:#?} clicks in {:?}", counts, it.elapsed());
    v.into_iter().min_by_key(|bucket| bucket.len()).unwrap()
}

pub fn gen_clicks_from_bbs_fixed(
    bbs: &[BoundingBox],
    remote_radius: u32,
    screen_w: u32,
    screen_h: u32,
) -> Vec<Coord> {
    let mut bbs = bbs.to_vec();
    let mut current_clicks = vec![];
    while !bbs.is_empty() {
        let bb = bbs.pop().unwrap();
        // Default to click the corner
        let mut best_click = bb.left_top;
        let mut best_hits = 0;

        for w in (i64::max(bb.left_top.w - remote_radius as i64, 0)
            ..=i64::min(bb.right_bottom.w + remote_radius as i64, screen_w as i64))
            .rev()
        {
            for h in (i64::max(bb.left_top.h - remote_radius as i64, 0)
                ..=i64::min(bb.right_bottom.h + remote_radius as i64, screen_h as i64))
                .rev()
            {
                let test_click = Coord { w, h };
                /*if bb.collides_with_point(test_click) {
                    // Skip checking if we're a direct hit. So we try to maximize collateral damage.
                    continue;
                }*/
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
        }
        current_clicks.push(best_click);
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
    current_clicks
}

fn get_rand_click(rng: &mut Rng, bb: &BoundingBox, remote_radius: u32, w: u32, h: u32) -> Coord {
    Coord {
        w: rng
            .rand_range_i64(
                bb.left_top.w - remote_radius as i64,
                bb.right_bottom.w + remote_radius as i64,
            )
            .clamp(0, w as i64),
        h: rng
            .rand_range_i64(
                bb.left_top.h - remote_radius as i64,
                bb.right_bottom.h + remote_radius as i64,
            )
            .clamp(0, h as i64),
    }
}

fn count_collisions_single(bbs: &[BoundingBox], remote_radius: u32, click: Coord) -> usize {
    let mut ct = 0;
    for bb in bbs {
        if bb.collides_with_circle(click, remote_radius) {
            ct += 1;
        }
    }
    ct
}
