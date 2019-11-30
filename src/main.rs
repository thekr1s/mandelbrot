use num::Complex;
use rayon::prelude::*;

/// Try to determine if `c` is in the Mandelbrot set, using at most `limit`
/// iterations to decide.
///
/// If `c` is not a member, return `Some(i)`, where `i` is the number of
/// iterations it took for `c` to leave the circle of radius two centered on the
/// origin. If `c` seems to be a member (more precisely, if we reached the
/// iteration limit without being able to prove that `c` is not a member),
/// return `None`.
fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        z = z * z + c;
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
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
        re: upper_left.re + pixel.0 as f64 * width  / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64
        // Why subtraction here? pixel.1 increases as we go down,
        // but the imaginary component increases as we go up.
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(pixel_to_point((100, 100), (25, 75),
                              Complex { re: -1.0, im:  1.0 },
                              Complex { re:  1.0, im: -1.0 }),
               Complex { re: -0.5, im: -0.5 });
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
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0 .. bounds.1 {
        for column in 0 .. bounds.0 {
            let point = pixel_to_point(bounds, (column, row),
                                       upper_left, lower_right);
            pixels[row * bounds.0 + column] =
                match escape_time(point, 255) {
                    None => 0,
                    Some(count) => (count % (128*2)) as u8  //255 - count as u8
                };
        }
    }
}

use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;

/// Write the buffer `pixels`, whose dimensions are given by `bounds`, to the
/// file named `filename`.
fn write_image(filename: String, pixels: &[u8], bounds: (usize, usize))
    -> Result<(), std::io::Error>
{
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels,
                   bounds.0 as u32, bounds.1 as u32,
                   ColorType::Gray(8))?;

    Ok(())
}


fn generate_field(size: (usize, usize), pixels: &mut[u8], upper_left: Complex<f64>, lower_right: Complex<f64>) {

    // Scope of slicing up `pixels` into horizontal bands.
    {
        let bands: Vec<(usize, &mut [u8])> = pixels
            .chunks_mut(size.0)
            .enumerate()
            .collect();
        bands.into_par_iter()
            .weight_max()
            .for_each(|(i, band)| {
                let top = i;
                let band_bounds = (size.0, 1);
                let band_upper_left = pixel_to_point(size, (0, top),
                                                     upper_left, lower_right);
                let band_lower_right = pixel_to_point(size, (size.0, top + 1),
                                                      upper_left, lower_right);
                render(band, band_bounds, band_upper_left, band_lower_right);
            });
    }
    
}

fn main() {

    let field_size = (6400, 6400);
    let mf_square_fields:usize = 10;

    let mf_upper_left: Complex<f64> = Complex {re: -1.16, im: 0.29};
    let mf_lower_right: Complex<f64> = Complex {re: -1.14, im: 0.275};
    let mf_size = Complex{re: mf_upper_left.re - mf_lower_right.re, im: mf_upper_left.im - mf_lower_right.im};
    let field_size_complex = Complex{re: mf_size.re / mf_square_fields as f64, im: mf_size.im / mf_square_fields as f64};

    let mut pixels = vec![0; field_size.0 * field_size.1];


    for row in 0..mf_square_fields {
        for col in 0..mf_square_fields {
            let re_offset = mf_size.re / mf_square_fields as f64 * col as f64;
            let im_offset = mf_size.im / mf_square_fields as f64 * row as f64;
            let field_upper_left = mf_upper_left - Complex{re: re_offset, im: im_offset};
            let field_lower_right = field_upper_left - field_size_complex;

            println!("generate {}_{} {:?} {:?}", row, col, field_upper_left, field_lower_right);

            generate_field(field_size, &mut pixels, field_upper_left, field_lower_right);
            
            let filename = format!("field_{:03}_{:03}_0.png", row, col);
            println!("Go write {}", filename);
            write_image(filename, &pixels, field_size).expect("error writing PNG file");
            println!("Write done");
        }
    }
}
