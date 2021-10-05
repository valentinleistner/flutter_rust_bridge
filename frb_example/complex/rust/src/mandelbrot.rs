///! NOTE: This file is **unrelated** to the main topic of our example.
///! Only for generating beautiful image.
///! Copied and modified from https://github.com/ProgrammingRust/mandelbrot/blob/task-queue/src/main.rs

#![warn(rust_2018_idioms)]
#![allow(elided_lifetimes_in_paths)]

use num::Complex;

/// Try to determine if `c` is in the Mandelbrot set, using at most `limit`
/// iterations to decide.
///
/// If `c` is not a member, return `Some(i)`, where `i` is the number of
/// iterations it took for `c` to leave the circle of radius two centered on the
/// origin. If `c` seems to be a member (more precisely, if we reached the
/// iteration limit without being able to prove that `c` is not a member),
/// return `None`.
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

/// Given the row and column of a pixel in the output image, return the
/// corresponding point on the complex plane.
///
/// `bounds` is a pair giving the width and height of the image in pixels.
/// `pixel` is a (column, row) pair indicating a particular pixel in that image.
/// The `upper_left` and `lower_right` parameters are points on the complex
/// plane designating the area our image covers.
fn pixel_to_point(bounds: (usize, usize),
                  pixel: (usize, usize),
                  upper_left: Complex<f64>,
                  lower_right: Complex<f64>)
                  -> Complex<f64>
{
    let (width, height) = (lower_right.re - upper_left.re,
                           upper_left.im - lower_right.im);
    Complex {
        re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64,
        // Why subtraction here? pixel.1 increases as we go down,
        // but the imaginary component increases as we go up.
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(pixel_to_point((100, 200), (25, 175),
                              Complex { re: -1.0, im: 1.0 },
                              Complex { re: 1.0, im: -1.0 }),
               Complex { re: -0.5, im: -0.75 });
}

/// Render a rectangle of the Mandelbrot set into a buffer of pixels.
///
/// The `bounds` argument gives the width and height of the buffer `pixels`,
/// which holds one grayscale pixel per byte. The `upper_left` and `lower_right`
/// arguments specify points on the complex plane corresponding to the upper-
/// left and lower-right corners of the pixel buffer.
fn render(pixels: &mut [u8],
          bounds: (usize, usize),
          upper_left: Complex<f64>,
          lower_right: Complex<f64>)
{
    assert_eq!(pixels.len(), bounds.0 * bounds.1);

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds, (column, row),
                                       upper_left, lower_right);
            pixels[row * bounds.0 + column] =
                match escape_time(point, 255) {
                    None => 0,
                    Some(count) => 255 - count as u8
                };
        }
    }
}

use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;

/// Write the buffer `pixels`, whose dimensions are given by `bounds`, to the
/// file named `filename`.
fn write_image(pixels: &[u8], bounds: (usize, usize)) -> Result<Vec<u8>, std::io::Error> {
    let mut buf = Vec::new();

    let encoder = PNGEncoder::new(&mut buf);
    encoder.encode(&pixels,
                   bounds.0 as u32, bounds.1 as u32,
                   ColorType::Gray(8))?;

    Ok(buf)
}

use std::sync::Mutex;
use std::env;
use std::io::Error;

pub fn draw_mandelbrot(image_width: usize, image_height: usize, left: f64, top: f64, right: f64, bottom: f64, threads: i32) -> Result<Vec<u8>, Error> {
    let bounds = (image_width, image_height);
    let upper_left = Complex::new(left, top);
    let lower_right = Complex::new(right, bottom);

    let mut pixels = vec![0; bounds.0 * bounds.1];

    let band_rows = bounds.1 / threads + 1;

    {
        let bands = Mutex::new(pixels.chunks_mut(band_rows * bounds.0).enumerate());
        crossbeam::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|_| {
                    loop {
                        match {
                            let mut guard = bands.lock().unwrap();
                            guard.next()
                        }
                        {
                            None => { return; }
                            Some((i, band)) => {
                                let top = band_rows * i;
                                let height = band.len() / bounds.0;
                                let band_bounds = (bounds.0, height);
                                let band_upper_left = pixel_to_point(bounds, (0, top),
                                                                     upper_left, lower_right);
                                let band_lower_right = pixel_to_point(bounds, (bounds.0, top + height),
                                                                      upper_left, lower_right);
                                render(band, band_bounds, band_upper_left, band_lower_right);
                            }
                        }
                    }
                });
            }
        }).unwrap();
    }

    write_image(&pixels, bounds)
}
