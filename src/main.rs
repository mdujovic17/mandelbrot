use num::Complex;
use std::str::FromStr;

use image::{ExtendedColorType, ImageEncoder};
use image::codecs::png::PngEncoder;
use std::fs::File;

use std::env;

fn main() {
    let args:Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!("Usage: {} FILE PIXELS, UPPER_LEFT LOWER_RIGHT", args[0]);
        eprintln!("Example: {} mandel.png 1000x750 -1.20,0.35 -1.0,0.20", args[0]);
        std::process::exit(1);
    }



    let bounds: Option<(usize, usize)> = parse_pair(&args[2], 'x');

    let upper_left = parse_complex(&args[3]);
    let lower_right = parse_complex(&args[4]);

    let mut pixels = vec![0; bounds.unwrap().0 * bounds.unwrap().1];

    // Single Threaded, non-concurrent:
    //
    // render(&mut pixels, bounds.unwrap(), upper_left.unwrap(), lower_right.unwrap());
    // write_image(&args[1], &pixels, bounds.unwrap()).expect("Error writing PNG File!");

    //Multi Threaded, concurrent:
    let threads = 16;
    let rows_per_band = bounds.unwrap().1 / threads + 1;

    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.unwrap().0).collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.unwrap().0;
                let band_bounds = (bounds.unwrap().0, height);
                let band_upper_left = pixel_to_point(bounds.unwrap(), (0, top), upper_left.unwrap(), lower_right.unwrap());
                let band_lower_right = pixel_to_point(bounds.unwrap(), (bounds.unwrap().0, top + height), upper_left.unwrap(), lower_right.unwrap());

                spawner.spawn(move |_| {
                    render(band, band_bounds, band_upper_left, band_lower_right);
                });
            }
        }).unwrap();
    }

    write_image(&args[1], &pixels, bounds.unwrap()).expect("Error writing PNG File!");
}

// fn sqrt(mut x: f64) {
//     loop {
//         x = x * x;
//     }
// }
//
// fn sqrt_add(c: f64) {
//     let mut x = 0.;
//     loop {
//         x = x * x + c;
//     }
// }
//
// fn complex_sqrt_add(c: Complex<f64>) {
//     let mut z = Complex { re: 0.0, im: 0.0 };
//     loop {
//         z = z * z + c;
//     }
// }


fn escape_time(c: Complex<f64>, limit: usize) -> Option<usize> {
    let mut z = Complex { re: 0.0, im: 0.0 };

    for i in 0..limit {
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }

        z = z * z + c;
    }

    return None;
}

fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10", ','), None);
    assert_eq!(parse_pair::<i32>(",10", ','), None);
    assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
    assert_eq!(parse_pair::<f64>("0.5x", 'x'), None);
    assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}

fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None
    }
}

#[test]
fn test_parse_complex() {
    assert_eq!(parse_complex("1.25,-0.0625"), Some(Complex{ re: 1.25, im: -0.0625 }));
    assert_eq!(parse_complex(",-0.0625"), None);
}

fn pixel_to_point(bounds: (usize, usize), pixel: (usize, usize), upper_left: Complex<f64>, lower_right: Complex<f64>) -> Complex<f64> {
    let (w, h) = (lower_right.re - upper_left.re, upper_left.im - lower_right.im);

    Complex {
        re: upper_left.re + pixel.0 as f64 * w / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * h / bounds.1 as f64
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(pixel_to_point((100, 200), (25, 175), Complex { re: -1.0, im: 1.0 }, Complex {re: 1.0, im: -1.0}), Complex { re: -0.5, im: -0.75 });
}

fn render(pixels: &mut [u8], bounds: (usize, usize), upper_left: Complex<f64>, lower_right: Complex<f64>) {
    assert_eq!(pixels.len(), bounds.0 * bounds.1);

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);

            pixels[row * bounds.0 + column] = match escape_time(point, 255) {
                None => 0,
                Some(count) => 255 - count as u8
            }
        }
    }
}

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize)) -> Result<(), std::io::Error> {
    // let output = match File::create(filename) {
    //     Ok(f) => f,
    //     Err(e) => {
    //         return Err(e);
    //     }
    // };
    // Is the same as below line of code:

    let output = File::create(filename)?;

    let encoder = PngEncoder::new(output);

    encoder.write_image(&pixels, bounds.0 as u32, bounds.1 as u32, ExtendedColorType::L8).expect("Failed to write image!");

    Ok(())
}


