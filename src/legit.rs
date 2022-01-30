use crate::draw_bbs;
use crate::BoundingBox;
use crate::Coord;
use crate::visualization::VisualizedImage;

use image::Rgb;
use std::collections::BTreeMap;

/// Tries to generate bounding boxes of enemies based on a screenshot of the map view of the game.
/// Works better the more zoomed in you are.
pub fn process_red(i: &mut VisualizedImage) -> (Vec<BoundingBox>, i64) {
    let mut bbs = scan_rects(i, 3);
    i.save("masked_bbs.png").unwrap();
    let spawner_bb = deduce_spawner_size(&bbs);
    println!("Deduced w {}", spawner_bb.w());
    bbs.extend(scan_rects_of_size(i, &spawner_bb));
    i.save("masked_bbs2.png").unwrap();
    bbs.extend(scan_rects(i, 1));
    i.save("masked_bbs3.png").unwrap();
    bbs.extend(scan_isolated_rects(i, &spawner_bb));
    i.save("masked_bbs4.png").unwrap();
    bbs.extend(scan_rect_any_ratio(i, 3));
    i.save("masked_bbs5.png").unwrap();
    bbs.retain(|bb| bb.area() as f64 > spawner_bb.area() as f64 / 5.0);
    draw_bbs(&bbs);

    (bbs, spawner_bb.w())
}

fn scan_rects_of_size(img: &mut VisualizedImage, template: &BoundingBox) -> Vec<BoundingBox> {
    let mut bbs = vec![];
    for _pass in 0..=1 {
        for h in 0..img.dimensions().1 as i64 {
            for w in 0..img.dimensions().0 as i64 {
                if at_enemy_edge(img, w, h) {
                    let bb = scan_single_bb(&img, w, h);
                    if bb.w() >= template.w() && bb.h() >= template.h() {
                        // The template fits within this area
                        let new_bb = BoundingBox::new(bb.left_top, template.w(), template.h());
                        mask_bb(img, &new_bb);
                        bbs.push(new_bb);
                    } else if i64::abs((bb.w() + 1) - template.w()) < 2 {
                        // Width is within 1 pixel of expectation.
                        if img.get_pixel_default(bb.right_bottom.w, bb.right_bottom.h + 1)
                            == &Rgb([0, 0, 0])
                        {
                            // One pixel south is exactly pure black
                            // So we are probably on the edge of explored area
                            let new_bb = BoundingBox::new(bb.left_top, template.w(), template.h());
                            mask_bb(img, &new_bb);
                            bbs.push(new_bb);
                        }
                    }
                    if !has_well_defined_corners(img, &bb) {
                        if i64::abs((bb.w() + 1) - template.w()) < 2
                            || i64::abs((bb.h() + 1) - template.h()) < 2
                        {
                            // width or height would fit one of these
                            if get_corner_strict(img, bb.left_top.w, bb.left_top.h).is_some() {
                                let new_bb =
                                    BoundingBox::new(bb.left_top, template.w(), template.h());
                                mask_bb(img, &new_bb);
                                bbs.push(new_bb);
                            } else if get_corner_strict(img, bb.right_bottom.w, bb.right_bottom.h)
                                .is_some()
                            {
                                // We want to add such that the bb left bottom corner matches up
                                let new_lefttop = Coord {
                                    w: bb.right_bottom.w - template.w(),
                                    h: bb.right_bottom.h - template.h(),
                                };
                                let new_bb =
                                    BoundingBox::new(new_lefttop, template.w(), template.h());
                                mask_bb(img, &new_bb);
                                bbs.push(new_bb);
                            }
                        } else {
                            // neither width nor height could fit here
                            // so naively add the full bb
                            //mask_bb(img, &bb);
                            //bbs.push(bb);
                        }
                    }
                }
            }
        }
    }
    bbs
}

fn at_enemy_edge(img: &VisualizedImage, w: i64, h: i64) -> bool {
    if looks_like_enemy(img.get_pixel_default(w, h)) {
        if !looks_like_enemy(img.get_pixel_default(w - 1, h)) {
            // We are introduced to a enemy pixel, and the previous pixel is not an enemy
            return true;
        }
    }
    false
}

fn scan_single_bb_vert(img: &VisualizedImage, w: i64, h: i64) -> BoundingBox {
    let mut scanlength = 1;
    for i in 1.. {
        if looks_like_enemy(img.get_pixel_default(w, h + i)) {
            scanlength = i;
        } else {
            break;
        }
    }

    let mut columns = 0;

    'outer: for i in 1.. {
        for j in 0..scanlength {
            if !looks_like_enemy(img.get_pixel_default(w + i, h + j)) {
                break 'outer;
            }
        }
        columns = i;
    }

    BoundingBox::new(Coord { w, h }, columns, scanlength)
}

fn scan_single_bb(img: &VisualizedImage, w: i64, h: i64) -> BoundingBox {
    img.toggle_frame_collect();
    let mut scanlength = 1;
    for i in 1.. {
        if looks_like_enemy(img.get_pixel_default(w + i, h)) {
            scanlength = i;
        } else {
            break;
        }
    }

    let mut rows_above = 0;
    let mut rows_below = 0;

    'outer: for i in 1.. {
        for j in 0..scanlength {
            if !looks_like_enemy(img.get_pixel_default(w + j, h - i)) {
                break 'outer;
            }
        }
        rows_above = i;
    }

    // scan below
    'outer2: for i in 1.. {
        for j in 0..scanlength {
            if !looks_like_enemy(img.get_pixel_default(w + j, h + i)) {
                break 'outer2;
            }
        }
        rows_below = i;
    }

    let rows = rows_below + rows_above;

    img.toggle_frame_collect();

    BoundingBox::new(
        Coord {
            w,
            h: h - rows_above,
        },
        scanlength,
        rows,
    )
}

/// Scan for isolated rectangles, but cap the size to the size of the template.
/// This is to try to deduce two (or more) spawners adjactent that look 2x as big as normal
fn scan_isolated_rects(img: &mut VisualizedImage, template: &BoundingBox) -> Vec<BoundingBox> {
    let mut bbs = vec![];
    for h in 0..img.dimensions().1 as i64 {
        for w in 0..img.dimensions().0 as i64 {
            if at_enemy_edge(&img, w, h) {
                let bb = scan_single_bb(&img, w, h);
                let perfect = get_corner(img, bb.left_top.w, bb.left_top.h)
                    == Some(Corner::LeftTop)
                    && get_corner(img, bb.left_top.w, bb.right_bottom.h)
                        == Some(Corner::LeftBottom)
                    && get_corner(img, bb.right_bottom.w, bb.left_top.h) == Some(Corner::RightTop)
                    && get_corner(img, bb.right_bottom.w, bb.right_bottom.h)
                        == Some(Corner::RightBottom);
                if perfect {
                    if bb.w() > template.w() + 2 {
                        // This is too long
                        let new_bb = BoundingBox::new(bb.left_top, template.w(), bb.h());
                        mask_bb(img, &new_bb);
                        bbs.push(new_bb);
                    } else if bb.h() > template.h() + 2 {
                        // This is too tall
                        let new_bb = BoundingBox::new(bb.left_top, bb.w(), template.h());
                        mask_bb(img, &new_bb);
                        bbs.push(new_bb);
                    } else {
                        mask_bb(img, &bb);
                        if bb.area() > 4 {
                            bbs.push(bb);
                        }
                    }
                }
            }
        }
    }
    bbs
}

/// Scans for rectangles, prioritizing things that look like biter bases.
/// On the final pass it accepts things that look like worms as well.
fn scan_rects(img: &mut VisualizedImage, passes: i64) -> Vec<BoundingBox> {
    let mut bbs = vec![];
    for pass in 0..passes {
        for h in 0..img.dimensions().1 as i64 {
            for w in 0..img.dimensions().0 as i64 {
                if at_enemy_edge(&img, w, h) {
                    let bb = scan_single_bb(&img, w, h);
                    if bb.ratio() >= 1.25 && bb.ratio() <= 1.5 {
                        // Ratio appears close to what a spawner could be
                        mask_bb(img, &bb);
                        if bb.area() > 4 {
                            bbs.push(bb);
                        }
                    }
                    if pass + 1 == passes {
                        // Check for wormlike on last pass
                        if bb.ratio() >= 0.6 && bb.ratio() < 1.25 {
                            mask_bb(img, &bb);
                            if bb.area() > 4 {
                                bbs.push(bb);
                            }
                        }
                        if has_well_defined_corners(img, &bb) {
                            if bb.ratio() / 2.0 <= 1.20 && bb.ratio() / 2.0 >= 0.7 {
                                // Appears to be 2 worms side by side
                                let new_bb = BoundingBox::new(bb.left_top, bb.w() / 2, bb.h());
                                mask_bb(img, &new_bb);
                                bbs.push(bb);
                            }
                            if bb.ratio() * 2.0 <= 1.20 && bb.ratio() * 2.0 >= 0.7 {
                                // Appears to be 2 worms stacked on top of one another
                                let new_bb = BoundingBox::new(bb.left_top, bb.w(), bb.h() / 2);
                                mask_bb(img, &new_bb);
                                bbs.push(bb);
                            }
                        }
                    }
                }
            }
        }
    }
    bbs
}

/// Final cleanup to try to find anything left, scaning both horizonal and vertical
/// and prioritizing larger bb
fn scan_rect_any_ratio(img: &mut VisualizedImage, passes: i64) -> Vec<BoundingBox> {
    let mut bbs = vec![];
    for _ in 0..passes {
        for h in 0..img.dimensions().1 as i64 {
            for w in 0..img.dimensions().0 as i64 {
                if at_enemy_edge(&img, w, h) {
                    let bb = scan_single_bb(&img, w, h);
                    let other_bb = scan_single_bb_vert(&img, w, h);
                    if bb.area() > other_bb.area() {
                        mask_bb(img, &bb);
                        bbs.push(bb);
                    } else {
                        mask_bb(img, &other_bb);
                        bbs.push(other_bb);
                    }
                }
            }
        }
    }
    bbs
}

fn deduce_spawner_size(bbs: &[BoundingBox]) -> BoundingBox {
    let mut bbs = bbs.to_owned();
    bbs.sort_by_key(|bb| bb.area());
    bbs.retain(|bb| bb.ratio() >= 1.25 && bb.ratio() <= 1.5);
    let areas = bbs.iter().map(|bb| bb.area()).collect::<Vec<_>>();
    let mut area_cts = BTreeMap::new();
    for area in areas {
        let entry = area_cts.entry(area).or_insert(0);
        *entry += 1;
    }

    for _pass in 0..=1 {
        let mut iter = area_cts.iter_mut().peekable();
        loop {
            if let Some((area, count)) = iter.next() {
                if let Some((next_area, next_count)) = iter.peek_mut() {
                    if (**next_area as f64 / *area as f64) < 1.05 {
                        if count > *next_count {
                            *count += **next_count;
                            **next_count = 0;
                        } else {
                            **next_count += *count;
                            *count = 0;
                        }
                    }
                }
            } else {
                break;
            }
        }
        area_cts.retain(|_k, v| *v > 1);
    }
    let likely_area = area_cts.keys().rev().next().unwrap_or(&0);
    *bbs.iter()
        .filter(|bb| bb.area() == *likely_area)
        .next()
        .unwrap_or(&BoundingBox::default())
}

fn looks_like_enemy(px: &Rgb<u8>) -> bool {
    let r = px[0];
    let g = px[1];
    let b = px[2];
    // Magic numbers that are in range of what colors enemy bases appear as on the map
    r > 150 && g < 35 && b < 36 && g > 11 && b > 14
}


fn mask_bb(img: &mut VisualizedImage, bb: &BoundingBox) {
    for (w, h) in bb.enumerate() {
        if w < 0 || h < 0 {
            continue;
        }
        if w as u32 >= img.width() || h as u32 >= img.height() {
            continue;
        }
        let mut px = *img.get_pixel(w as u32, h as u32);
        // Add some green so this no longer looks like an enemy
        px[0] = 0xff;
        px[1] = 0xff;
        px[2] = 0xff;
        img.put_pixel(w as u32, h as u32, px);
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Corner {
    LeftTop,
    LeftBottom,
    RightTop,
    RightBottom,
}

fn has_well_defined_corners(img: &VisualizedImage, bb: &BoundingBox) -> bool {
    return get_corner_strict(&img, bb.left_top.w, bb.left_top.h).is_some()
        && get_corner_strict(&img, bb.left_top.w, bb.right_bottom.h).is_some()
        && get_corner_strict(&img, bb.right_bottom.w, bb.left_top.h).is_some()
        && get_corner_strict(&img, bb.right_bottom.w, bb.right_bottom.h).is_some();
}

/// The same as get_corner, but enforces that all corners are NOT bordering a fully black pixel
fn get_corner_strict(img: &VisualizedImage, w: i64, h: i64) -> Option<Corner> {
    if let Some(corn) = get_corner(img, w, h) {
        match corn {
            Corner::LeftTop => {
                if img.get_pixel_checked(w, h - 1) == Some(&Rgb([0; 3]))
                    || img.get_pixel_checked(w - 1, h) == Some(&Rgb([0; 3]))
                {
                    return None;
                }
                return Some(corn);
            }
            Corner::LeftBottom => {
                if img.get_pixel_checked(w, h + 1) == Some(&Rgb([0; 3]))
                    || img.get_pixel_checked(w - 1, h) == Some(&Rgb([0; 3]))
                {
                    return None;
                }
                return Some(corn);
            }
            Corner::RightTop => {
                if img.get_pixel_checked(w, h - 1) == Some(&Rgb([0; 3]))
                    || img.get_pixel_checked(w + 1, h) == Some(&Rgb([0; 3]))
                {
                    return None;
                }
                return Some(corn);
            }
            Corner::RightBottom => {
                if img.get_pixel_checked(w, h + 1) == Some(&Rgb([0; 3]))
                    || img.get_pixel_checked(w + 1, h) == Some(&Rgb([0; 3]))
                {
                    return None;
                }
                return Some(corn);
            }
        }
    }
    None
}

fn get_corner(img: &VisualizedImage, w: i64, h: i64) -> Option<Corner> {
    if let Some(px) = img.get_pixel_checked(w, h) {
        if looks_like_enemy(px) {
            let p1 = img.get_pixel_checked(w - 1, h);
            let p2 = img.get_pixel_checked(w, h - 1);
            let p3 = img.get_pixel_checked(w + 1, h);
            let p4 = img.get_pixel_checked(w, h + 1);
            let p1 = p1.is_some() && looks_like_enemy(p1.unwrap());
            let p2 = p2.is_some() && looks_like_enemy(p2.unwrap());
            let p3 = p3.is_some() && looks_like_enemy(p3.unwrap());
            let p4 = p4.is_some() && looks_like_enemy(p4.unwrap());
            if p1 && p2 && !p3 && !p4 {
                return Some(Corner::RightBottom);
            } else if !p1 && p2 && p3 && !p4 {
                return Some(Corner::LeftBottom);
            } else if !p1 && !p2 && p3 && p4 {
                return Some(Corner::LeftTop);
            } else if p1 && !p2 && !p3 && p4 {
                return Some(Corner::RightTop);
            }
        }
    }

    None
}
