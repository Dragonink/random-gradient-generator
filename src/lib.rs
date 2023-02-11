//! Library to generate random grandient images using [Perlin noise](https://en.wikipedia.org/wiki/Perlin_noise)
#![warn(
	unused,
	clippy::unused_self,
	unused_crate_dependencies,
	unused_import_braces,
	unreachable_pub,
	noop_method_call,
	clippy::match_wildcard_for_single_variants,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::match_on_vec_items,
	clippy::imprecise_flops,
	clippy::suboptimal_flops,
	clippy::float_cmp,
	clippy::float_cmp_const,
	clippy::mem_forget,
	clippy::filter_map_next,
	clippy::verbose_file_reads,
	clippy::inefficient_to_string,
	clippy::str_to_string,
	clippy::option_option,
	clippy::dbg_macro,
	clippy::print_stdout,
	clippy::print_stderr,
	missing_debug_implementations,
	missing_copy_implementations,
	clippy::missing_const_for_fn,
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::doc_link_with_quotes,
	clippy::doc_markdown,
	clippy::needless_continue,
	clippy::manual_let_else,
	clippy::unnested_or_patterns,
	clippy::semicolon_if_nothing_returned,
	clippy::empty_line_after_outer_attr,
	clippy::empty_structs_with_brackets,
	clippy::enum_glob_use,
	clippy::macro_use_imports,
	clippy::mod_module_files
)]
#![deny(
	keyword_idents,
	non_ascii_idents,
	unused_must_use,
	clippy::lossy_float_literal,
	clippy::exit
)]
#![forbid(unsafe_code, clippy::missing_panics_doc, clippy::missing_errors_doc)]

use bmp::{Image, Pixel};
use std::{
	error::Error,
	fmt::{self, Display, Formatter},
	ops::RangeInclusive,
	str::FromStr,
};

/// Size of the image in pixels
///
/// # [`FromStr` implementation](Self#impl-FromStr-for-Size)
/// The implementation parses strings formatted like `WxH`
/// where `W` is the [`width`](Self#structfield.width) and `H` the [`height`](Self#structfield.height).
///
/// ## Example
/// ```
/// # use random_gradient_generator::Size;
/// let size: Size = "512x256".parse().unwrap();
/// assert_eq!(size, Size {
///     width: 512,
///     height: 256,
/// });
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Size {
	/// Image width
	pub width: u32,
	/// Image height
	pub height: u32,
}
impl FromStr for Size {
	type Err = <u32 as FromStr>::Err;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut dim = s.split('x');

		let width = dim.next().unwrap_or_default().parse()?;
		let height = dim.next().unwrap_or_default().parse()?;

		Ok(Self { width, height })
	}
}
impl Display for Size {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{w}x{h}", w = self.width, h = self.height)
	}
}

#[allow(missing_docs, clippy::missing_docs_in_private_items)]
/// Initial components of the pixel colors
///
/// Each variant represents the component that will be randomized
/// and stores the two others.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelInit {
	/// Randomize `hue`
	Hue { saturation: f32, brightness: f32 },
	/// Randomize `saturation`
	Saturation { hue: f32, brightness: f32 },
	/// Randomize `brightness`
	Brightness { hue: f32, saturation: f32 },
}
impl PixelInit {
	/// Returns the valid range for the randomzed component
	#[inline]
	pub const fn valid_range(&self) -> RangeInclusive<f32> {
		match self {
			Self::Hue { .. } => 0.0..=359.99,
			Self::Saturation { .. } => 0.0..=1.0,
			Self::Brightness { .. } => 0.0..=1.0,
		}
	}
}

/// Parameters to construct the noise with
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct NoiseOptions {
	/// Value that changes the output of the noise
	pub seed: i32,
	/// Number of cycles per unit length that the noise outputs
	pub frequency: f32,
}

/// Generates an image with given `size`, `pixel_init` and `noise_options`
///
/// # Errors
/// This function will return an [`OutOfRangeValue`] error if any of the following conditions is false:
/// - <math><mn>0</mn><mo>≤</mo><mi>hue</mi><mo><</mo><mn>360</mn></math>
/// - <math><mn>0</mn><mo>≤</mo><mi>saturation</mi><mo>≤</mo><mn>1</mn></math>
/// - <math><mn>0</mn><mo>≤</mo><mi>brightness</mi><mo>≤</mo><mn>1</mn></math>
pub fn generate_image(
	size: Size,
	pixel_init: PixelInit,
	noise_options: NoiseOptions,
) -> Result<Image, OutOfRangeValue> {
	use simdnoise::NoiseBuilder;

	let mut settings = NoiseBuilder::gradient_2d(size.width as usize, size.height as usize);
	settings
		.with_freq(noise_options.frequency)
		.with_seed(noise_options.seed);
	let noise_range = pixel_init.valid_range();
	let noise = settings.generate_scaled(*noise_range.start(), *noise_range.end());

	let mut image = Image::new(size.width, size.height);
	for (x, y) in image.coordinates() {
		let noise_value = noise[(size.width * y + x) as usize];
		let px = match pixel_init {
			PixelInit::Hue {
				saturation,
				brightness,
			} => hsv_to_rgb(noise_value, saturation, brightness),
			PixelInit::Saturation { hue, brightness } => hsv_to_rgb(hue, noise_value, brightness),
			PixelInit::Brightness { hue, saturation } => hsv_to_rgb(hue, saturation, noise_value),
		}?;

		image.set_pixel(x, y, px);
	}

	Ok(image)
}

/// Converts HSV to RGB
///
/// <math display="block">
///   <mi>C</mi>
///   <mo>=</mo>
///   <mi>saturation</mi>
///   <mo>×</mo>
///   <mi>brightness</mi>
/// </math>
/// <math display="block">
///   <mi>X</mi>
///   <mo>=</mo>
///   <mi>C</mi>
///   <mo>×</mo>
///   <mrow><mo>(</mo><mrow>
///   <mn>1</mn>
///   <mo>-</mo>
///   <mrow><mo>|</mo><mrow>
///   <mfrac>
///     <mi>hue</mi>
///     <mn>60</mn>
///   </mfrac>
///   <mo>%</mo>
///   <mn>2</mn>
///   <mo>-</mo>
///   <mn>1</mn>
///   </mrow><mo>|</mo></mrow>
///   </mrow><mo>)</mo></mrow>
/// </math>
/// <math display="block">
///   <mi>M</mi>
///   <mo>=</mo>
///   <mi>brightness</mi>
///   <mo>-</mo>
///   <mi>C</mi>
/// </math>
/// <math display="block">
///   <mrow><mo>(</mo><mrow>
///   <mi>red'</mi>
///   <mo>,</mo>
///   <mi>green'</mi>
///   <mo>,</mo>
///   <mi>blue'</mi>
///   </mrow><mo>)</mo></mrow>
///   <mo>=</mo>
///   <mrow><mo>{</mo><mtable>
///     <mtr>
///       <mtd><mo>(</mo><mrow>
///       <mi>C</mi>
///       <mo>,</mo>
///       <mi>X</mi>
///       <mo>,</mo>
///       <mi>0</mi>
///       </mrow><mo>)</mo></mtd>
///       <mtd>
///         <mtext>if</mtext>
///         <mpadded lspace="1em">
///           <mn>0</mn>
///           <mo>≤</mo>
///           <mi>hue</mi>
///           <mo><</mo>
///           <mn>60</mn>
///         </mpadded>
///       </mtd>
///     </mtr>
///     <mtr>
///       <mtd><mo>(</mo><mrow>
///       <mi>X</mi>
///       <mo>,</mo>
///       <mi>C</mi>
///       <mo>,</mo>
///       <mi>0</mi>
///       </mrow><mo>)</mo></mtd>
///       <mtd>
///         <mtext>if</mtext>
///         <mpadded lspace="1em">
///           <mn>60</mn>
///           <mo>≤</mo>
///           <mi>hue</mi>
///           <mo><</mo>
///           <mn>120</mn>
///         </mpadded>
///       </mtd>
///     </mtr>
///     <mtr>
///       <mtd><mo>(</mo><mrow>
///       <mi>0</mi>
///       <mo>,</mo>
///       <mi>C</mi>
///       <mo>,</mo>
///       <mi>X</mi>
///       </mrow><mo>)</mo></mtd>
///       <mtd>
///         <mtext>if</mtext>
///         <mpadded lspace="1em">
///           <mn>120</mn>
///           <mo>≤</mo>
///           <mi>hue</mi>
///           <mo><</mo>
///           <mn>180</mn>
///         </mpadded>
///       </mtd>
///     </mtr>
///     <mtr>
///       <mtd><mo>(</mo><mrow>
///       <mi>0</mi>
///       <mo>,</mo>
///       <mi>X</mi>
///       <mo>,</mo>
///       <mi>C</mi>
///       </mrow><mo>)</mo></mtd>
///       <mtd>
///         <mtext>if</mtext>
///         <mpadded lspace="1em">
///           <mn>180</mn>
///           <mo>≤</mo>
///           <mi>hue</mi>
///           <mo><</mo>
///           <mn>240</mn>
///         </mpadded>
///       </mtd>
///     </mtr>
///     <mtr>
///       <mtd><mo>(</mo><mrow>
///       <mi>X</mi>
///       <mo>,</mo>
///       <mi>0</mi>
///       <mo>,</mo>
///       <mi>C</mi>
///       </mrow><mo>)</mo></mtd>
///       <mtd>
///         <mtext>if</mtext>
///         <mpadded lspace="1em">
///           <mn>240</mn>
///           <mo>≤</mo>
///           <mi>hue</mi>
///           <mo><</mo>
///           <mn>300</mn>
///         </mpadded>
///       </mtd>
///     </mtr>
///     <mtr>
///       <mtd><mo>(</mo><mrow>
///       <mi>C</mi>
///       <mo>,</mo>
///       <mi>0</mi>
///       <mo>,</mo>
///       <mi>X</mi>
///       </mrow><mo>)</mo></mtd>
///       <mtd>
///         <mtext>if</mtext>
///         <mpadded lspace="1em">
///           <mn>300</mn>
///           <mo>≤</mo>
///           <mi>hue</mi>
///           <mo><</mo>
///           <mn>360</mn>
///         </mpadded>
///       </mtd>
///     </mtr>
///   </mtable></mrow>
/// </math>
/// <math display="block">
///   <mrow><mo>(</mo><mrow>
///   <mi>red</mi>
///   <mo>,</mo>
///   <mi>green</mi>
///   <mo>,</mo>
///   <mi>blue</mi>
///   </mrow><mo>)</mo></mrow>
///   <mo>=</mo>
///   <mrow><mo>(</mo><mrow>
///   <mrow><mo>(</mo><mrow>
///   <mi>red'</mi><mo>+</mo><mi>M</mi>
///   </mrow><mo>)</mo></mrow>
///   <mo>×</mo><mn>255</mn>
///   <mo>,</mo>
///   <mrow><mo>(</mo><mrow>
///   <mi>green'</mi><mo>+</mo><mi>M</mi>
///   </mrow><mo>)</mo></mrow>
///   <mo>×</mo><mn>255</mn>
///   <mo>,</mo>
///   <mrow><mo>(</mo><mrow>
///   <mi>blue'</mi><mo>+</mo><mi>M</mi>
///   </mrow><mo>)</mo></mrow>
///   <mo>×</mo><mn>255</mn>
///   </mrow><mo>)</mo></mrow>
/// </math>
///
/// # Errors
/// This function will return an [`OutOfRangeValue`] error if any of the following conditions is false:
/// - <math><mn>0</mn><mo>≤</mo><mi>hue</mi><mo><</mo><mn>360</mn></math>
/// - <math><mn>0</mn><mo>≤</mo><mi>saturation</mi><mo>≤</mo><mn>1</mn></math>
/// - <math><mn>0</mn><mo>≤</mo><mi>brightness</mi><mo>≤</mo><mn>1</mn></math>
pub fn hsv_to_rgb(hue: f32, saturation: f32, brightness: f32) -> Result<Pixel, OutOfRangeValue> {
	if !(0.0..360.0).contains(&hue) {
		return Err(OutOfRangeValue::Hue);
	}
	if !(0.0..=1.0).contains(&saturation) {
		return Err(OutOfRangeValue::Saturation);
	}
	if !(0.0..=1.0).contains(&brightness) {
		return Err(OutOfRangeValue::Brightness);
	}

	let c = saturation * brightness;
	let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
	let m = brightness - c;
	let (r, g, b) = if hue < 60.0 {
		(c, x, 0.0)
	} else if hue < 120.0 {
		(x, c, 0.0)
	} else if hue < 180.0 {
		(0.0, c, x)
	} else if hue < 240.0 {
		(0.0, x, c)
	} else if hue < 300.0 {
		(x, 0.0, c)
	} else {
		(c, 0.0, x)
	};
	Ok(Pixel::new(
		((r + m) * 255.0) as u8,
		((g + m) * 255.0) as u8,
		((b + m) * 255.0) as u8,
	))
}

#[allow(missing_docs, clippy::missing_docs_in_private_items)]
/// Error returned if any given value is not in its expected range
#[derive(Debug, Clone, Copy)]
pub enum OutOfRangeValue {
	Hue,
	Saturation,
	Brightness,
}
impl Display for OutOfRangeValue {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Hue => write!(f, "Hue is out of range: 0 <= hue < 360"),
			Self::Saturation => write!(f, "Saturation is out of range: 0 <= saturation <= 1"),
			Self::Brightness => write!(f, "Brightness is out of range: 0 <= brightness <= 1"),
		}
	}
}
impl Error for OutOfRangeValue {
	#[inline]
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		None
	}
}
