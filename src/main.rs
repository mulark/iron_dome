use captrs::Capturer;
use image::GrayImage;
use image::Rgb;
use imageproc::filter::sharpen3x3;

use std::error::Error;

const EXCLUSION_BOX: isize = 24;

fn main() -> Result<(), Box<dyn Error>> {
    let (bytes, coord) = capture_frame();
    let mut click_coords: Vec<(isize, isize)> = Vec::new();
    for h in 0..coord.1 {
        'next_px: for w in 0..coord.0 {
            let px = bytes[w][h];
            for click in click_coords.iter() {
                if isize::abs(w as isize - click.0) < EXCLUSION_BOX
                    && isize::abs(h as isize - click.1) < EXCLUSION_BOX
                {
                    continue 'next_px;
                }
            }
            let r = px.0[0];
            let g = px.0[1];
            let b = px.0[2];
            if (r == 0 || (r == 0xff && false)) && g == 0 && b == 0xff {
                click_coords.push((w as isize, h as isize));
            }
        }
    }

    println!("Generated {} clicks", click_coords.len());
    let mut cmd = String::new();
    for click in click_coords {
        cmd.push_str(&format!("mousemove {} {} click 1 ", click.0, click.1));
    }
    println!("{}", cmd);
    let o = std::process::Command::new("xdotool")
        .args(cmd.split_whitespace())
        .output()?;
    println!("{:?}", o);

    Ok(())
}

fn capture_frame() -> (Vec<Vec<Rgb<u8>>>, (usize, usize)) {
    let mut c = Capturer::new(0).unwrap();

    let bytes = c.capture_frame().unwrap();
    let geometry = c.geometry();
    let mut img_bytes: Vec<Vec<Rgb<u8>>> =
        vec![Vec::with_capacity(geometry.1 as usize); geometry.0 as usize];
    for (i, by) in bytes.iter().enumerate() {
        let w = i % geometry.0 as usize;
        let _h = i / geometry.0 as usize;
        img_bytes[w].push(Rgb {
            0: [by.r, by.g, by.b],
        });
    }

    (img_bytes, (geometry.0 as usize, geometry.1 as usize))
}

use image::io::Reader as ImageReader;
use image::Luma;
use imageproc::distance_transform::Norm;
use imageproc::map::*;
use imageproc::morphology::*;

fn postprocess(testcase: &str) -> Result<(), Box<dyn Error>> {
    let file = testcase.to_owned() + ".png";
    let img = ImageReader::open(&file)?.decode()?.to_rgb8();
    let img: GrayImage = map_pixels(&img, |_w, _h, pixel| {
        let (r, g, b) = unsafe { std::mem::transmute::<[u8; 3], (u8, u8, u8)>(pixel.0) };
        if r > 150 && g < 30 && b < 36 && g > 11 && b > 14 {
            Luma([255; 1])
        } else {
            Luma([0; 1])
        }
    });
    img.save(testcase.to_owned() + "-1.png")?;
    let img = map_pixels(&img, |w, h, pixel| {
        let mut num = 0;
        num += (img.get_pixel(w, h - 1).0[0] == 0) as u8;
        num += (img.get_pixel(w, h + 1).0[0] == 0) as u8;
        num += (img.get_pixel(w - 1, h).0[0] == 0) as u8;
        num += (img.get_pixel(w + 1, h).0[0] == 0) as u8;
        if num > 3 {
            Luma([0; 1])
        } else {
            Luma([255; 1])
        }
    });

    let img = dilate(&img, Norm::L1, 1);
    img.save(testcase.to_owned() + "-3.png")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::postprocess;
    use std::fs::read_dir;

    #[test]
    fn test_red() {
        for f in read_dir("testcases").unwrap() {
            let p = f.unwrap().path();
            println!("{:?}", p);
            let testcase = format!("testcases/{}", p.file_stem().unwrap().to_string_lossy());
            if testcase.contains('-') {
                std::fs::remove_file(testcase + ".png").unwrap();
            }
        }
        for f in read_dir("testcases").unwrap() {
            let p = f.unwrap().path();
            println!("{:?}", p);
            let testcase = format!("testcases/{}", p.file_stem().unwrap().to_string_lossy());
            if !testcase.contains('-') {
                postprocess(&testcase).unwrap();
            }
        }
    }

    #[test]
    fn test() {
        assert_eq!(
            std::mem::size_of::<[u8; 3]>(),
            std::mem::size_of::<(u8, u8, u8)>()
        )
    }
}
