use image::{png::PNGEncoder, ColorType};
use num::Complex;
use std::{fs::File, str::FromStr};

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!("Usage: {} FILE PIXELS UPPERLEFT LOWERRIGHT", args[0]);
        eprintln!(
            "Example: {} mandel.png 1000x750 -1.20,0.35 -1,0.20",
            args[0]
        );
        std::process::exit(1);
    }

    let bounds = parse_pair(&args[2], 'x').expect("error parsing image dimensions");
    let ul = parse_complex(&args[3]).expect("error parsing upper left corner point");
    let lr = parse_complex(&args[4]).expect("error parsing lower right corner point");

    let mut pixels = vec![0; bounds.0 * bounds.1];

    let threads = num_cpus::get();
    let rows_per_band = bounds.1 / threads + 1;
    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();

        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_ul = pixel_to_point(bounds, (0, top), ul, lr);
                let band_lr = pixel_to_point(bounds, (bounds.0, height + top), ul, lr);
                spawner.spawn(move || render(band, band_bounds, band_ul, band_lr));
            }
        });
    }
    write_image(&args[1], &pixels, bounds).expect("error writing PNG file");
}

fn escape_time(c: Complex<f64>, limit: usize) -> Option<usize> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
        z = z * z + c;
    }
    None
}

fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
            (Ok(l), Ok(r)) => Some((l, r)),
            _ => None,
        },
    }
}

fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None,
    }
}

fn pixel_to_point(
    bounds: (usize, usize),
    pixel: (usize, usize),
    ul: Complex<f64>,
    lr: Complex<f64>,
) -> Complex<f64> {
    let (width, height) = (lr.re - ul.re, lr.im - ul.im);
    Complex {
        re: ul.re + pixel.0 as f64 / bounds.0 as f64 * width,
        im: ul.im + pixel.1 as f64 / bounds.1 as f64 * height,
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(
        pixel_to_point(
            (1000, 500),
            (200, 250),
            Complex { re: 1.0, im: 0.0 },
            Complex { re: 6.0, im: 2.0 }
        ),
        Complex { re: 2.0, im: 1.0 }
    )
}

fn render(pixels: &mut [u8], bounds: (usize, usize), ul: Complex<f64>, lr: Complex<f64>) {
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0..bounds.1 {
        for col in 0..bounds.0 {
            pixels[row * bounds.0 + col] =
                match escape_time(pixel_to_point(bounds, (col, row), ul, lr), 255) {
                    Some(i) => 255 - i as u8,
                    None => 0,
                }
        }
    }
}

fn write_image(fname: &str, pixels: &[u8], bounds: (usize, usize)) -> Result<(), std::io::Error> {
    let output = File::create(fname)?;
    let encoder = PNGEncoder::new(output);
    encoder.encode(
        &pixels,
        bounds.0 as u32,
        bounds.1 as u32,
        ColorType::Gray(8),
    )?;
    Ok(())
}
