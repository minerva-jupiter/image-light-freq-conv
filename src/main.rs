use image::{GenericImageView, DynamicImage, Rgba, Pixel, Luma};
use std::env;
use std::fs::File;
use std::path::Path;

fn load_image(path: &Path) -> Result<Vec<Vec<Rgba<u8>>>, image::ImageError> {
    let img: DynamicImage = image::open(path)?;
    let (width, height) = img.dimensions();

    let mut pixel_array: Vec<Vec<Rgba<u8>>> = Vec::with_capacity(height as usize);
    for y in 0..height {
        let mut row: Vec<Rgba<u8>> = Vec::with_capacity(width as usize);
        for x in 0..width {
            let pixel: Rgba<u8> = img.get_pixel(x, y).to_rgba();
            row.push(pixel);
        }
        pixel_array.push(row);
    }
    Ok(pixel_array)
}

const MIN_WAVELENGTH_NM: f64 = 400.0; // 紫の端
const MAX_WAVELENGTH_NM: f64 = 700.0; // 赤の端
const GREEN_WAVELENGTH_NM: f64 = 550.0; // 緑色の中央付近
                                        //
fn rgba_to_approx_wavelength(pixel: Rgba<u8>) -> f64 {
    let r_u8 = pixel.0[0] as f64;
    let g_u8 = pixel.0[1] as f64;
    let b_u8 = pixel.0[2] as f64;

    let r = r_u8 / 255.0;
    let g = g_u8 / 255.0;
    let b = b_u8 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let chroma = max - min;
    let hue_deg; // 色相 (0.0 から 360.0 度)

    if chroma == 0.0 {
        return GREEN_WAVELENGTH_NM;
    } else {
        let mut h_prime = if max == r {
            (g - b) / chroma
        } else if max == g {
            (b - r) / chroma + 2.0
        } else {
            (r - g) / chroma + 4.0
        };

        if h_prime < 0.0 {
            h_prime += 6.0;
        }

        hue_deg = h_prime * 60.0;
    }
    
    let h_norm = hue_deg / 360.0; 
    
    let wavelength_nm;
    
    if h_norm <= 1.0 / 6.0 { // 赤〜黄 (0°〜60°) : 700nm から 590nm
        let t = h_norm * 6.0; // tは 0.0 から 1.0
        wavelength_nm = MAX_WAVELENGTH_NM - t * (MAX_WAVELENGTH_NM - 590.0);
    } else if h_norm <= 2.0 / 6.0 { // 黄〜緑 (60°〜120°) : 590nm から 550nm
        let t = (h_norm - 1.0 / 6.0) * 6.0; // tは 0.0 から 1.0
        wavelength_nm = 590.0 - t * (590.0 - GREEN_WAVELENGTH_NM);
    } else if h_norm <= 4.0 / 6.0 { // 緑〜青 (120°〜240°) : 550nm から 450nm
        let t = (h_norm - 2.0 / 6.0) * 3.0; // tは 0.0 から 1.0
        wavelength_nm = GREEN_WAVELENGTH_NM - t * (GREEN_WAVELENGTH_NM - 450.0);
    } else { // 青〜赤 (240°〜360°) : 450nm から 700nm

        let t_total = h_norm - 4.0 / 6.0; // 240度 (4/6) から 1.0 までの範囲
        let t_norm = t_total * 3.0;       // t_norm は 0.0 から 1.0

        wavelength_nm = 450.0 - t_norm * (450.0 - MIN_WAVELENGTH_NM);

        return wavelength_nm.max(MIN_WAVELENGTH_NM).min(MAX_WAVELENGTH_NM);
    }

    wavelength_nm.max(MIN_WAVELENGTH_NM).min(MAX_WAVELENGTH_NM)
}

fn wavelength_to_grayscale(wavelength_nm: f64) -> u8 {
    let clamped_wavelength_nm = wavelength_nm.max(MIN_WAVELENGTH_NM).min(MAX_WAVELENGTH_NM);

    let range_length = MAX_WAVELENGTH_NM - MIN_WAVELENGTH_NM;

    let normalized_position = (clamped_wavelength_nm - MIN_WAVELENGTH_NM) / range_length;

    let grayscale_f64 = normalized_position * 255.0;

    grayscale_f64.round() as u8
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 || args.len() > 4{
        println!("args are not collect. use this 'image-light-freq-con <inputFilePath> <outputFilePath>'");
        return Ok(());
    }
    let path = Path::new(&args[1]);
    let pixels = match load_image(path){
        Ok(pixels) => pixels,
        Err(e) => {
            println!("Could not load image. {:?}",e);
            return Ok(());
        }
    };

    let height = pixels.len() as u32;
    let width = pixels[0].len() as u32;

    let mut grascale_img: Vec<Vec<u8>> = Vec::with_capacity(height as usize);

    for row in pixels.iter() {
        let mut grascale_img_row: Vec<u8> = Vec::with_capacity(width as usize);
        for pixel in row.iter() {
            grascale_img_row.push(wavelength_to_grayscale(rgba_to_approx_wavelength(*pixel)));
        }
        grascale_img.push(grascale_img_row);
    }

    let grayscale_data_flat: Vec<u8> = grascale_img.into_iter()
        .flat_map(|row| row.into_iter())
        .collect();

    let output_img = image::ImageBuffer::<Luma<u8>, _>::from_raw(width, height, grayscale_data_flat)
        .ok_or("Failed to create image buffer")?;

    let output_filename = Path::new(&args[2]);
    
    let mut output_file = File::create(output_filename)?;
    output_img.write_to(&mut output_file, image::ImageFormat::Png)?;
    
    Ok(())
}
