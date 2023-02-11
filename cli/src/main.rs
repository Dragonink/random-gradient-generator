//! CLI tool to invoke [`random_gradient_generator`]
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
	clippy::exit,
	clippy::missing_panics_doc,
	clippy::missing_errors_doc
)]
#![forbid(unsafe_code)]

use clap::{Args, Parser, ValueHint};
use random_gradient_generator::{NoiseOptions, PixelInit, Size};
use std::{
	fmt::{self, Display, Formatter},
	path::PathBuf,
	str::FromStr,
};

/// CLI tool to generate random gradient images using Perlin noise
#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	/// Path to the output image
	#[arg(value_name = "PATH", value_hint = ValueHint::FilePath)]
	output: PathBuf,
	/// Size of the image in pixels (format: `WxH`, e.g. `512x256`)
	#[arg(short, long)]
	size: Size,
	/// Argument group related to pixel colors
	#[command(flatten, next_help_heading = "Pixel color options")]
	color: CliColor,
	/// Argument group related to the noise
	#[command(flatten, next_help_heading = "Noise options")]
	noise: CliNoise,
}

/// Possible states for arguments of [`CliColor`]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum ColorParameter {
	/// The value is set
	Set(f32),
	/// The value is not set and should be the one to be randomized
	#[default]
	Random,
}
impl ColorParameter {
	/// String representation of [`Self::Random`]
	const RANDOM_STR: &str = "RANDOM";
}
impl From<f32> for ColorParameter {
	#[inline]
	fn from(value: f32) -> Self {
		Self::Set(value)
	}
}
impl FromStr for ColorParameter {
	type Err = <f32 as FromStr>::Err;

	#[inline]
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"" | Self::RANDOM_STR => Self::Random,
			s => Self::Set(s.parse()?),
		})
	}
}
impl Display for ColorParameter {
	#[inline]
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Set(value) => Display::fmt(&value, f),
			Self::Random => f.write_str(Self::RANDOM_STR),
		}
	}
}

/// Argument group related to pixel colors
#[derive(Debug, Clone, Copy, Args)]
struct CliColor {
	/// Hue component of the colors (range: 0 < hue ≤ 360)
	#[arg(long, value_name = "FLOAT", default_value_t)]
	hue: ColorParameter,
	/// Saturation component of the colors (range: 0 ≤ saturation ≤ 1)
	#[arg(
		long,
		value_name = "FLOAT",
		default_value_t = ColorParameter::Set(1.0)
	)]
	saturation: ColorParameter,
	/// Brightness component of the colors (range: 0 ≤ brightness ≤ 1)
	#[arg(
		long,
		value_name = "FLOAT",
		default_value_t = ColorParameter::Set(1.0)
	)]
	brightness: ColorParameter,
}
impl From<CliColor> for PixelInit {
	#[inline]
	fn from(cli: CliColor) -> Self {
		match cli {
			CliColor {
				hue: ColorParameter::Random,
				saturation: ColorParameter::Set(saturation),
				brightness: ColorParameter::Set(brightness),
			} => PixelInit::Hue {
				saturation,
				brightness,
			},
			CliColor {
				hue: ColorParameter::Set(hue),
				saturation: ColorParameter::Random,
				brightness: ColorParameter::Set(brightness),
			} => PixelInit::Saturation { hue, brightness },
			CliColor {
				hue: ColorParameter::Set(hue),
				saturation: ColorParameter::Set(saturation),
				brightness: ColorParameter::Random,
			} => PixelInit::Brightness { hue, saturation },
			_ => {
				panic!(
					"{cli:?} must have exactly one component set to {:?}",
					ColorParameter::Random
				);
			}
		}
	}
}

/// Argument group related to the noise
#[derive(Debug, Clone, Copy, Args)]
struct CliNoise {
	/// Value that changes the output of the noise function
	///
	/// This value will be randomly choosen if not specified.
	#[arg(long, value_name = "INT")]
	seed: Option<i32>,
	/// Number of cycles per unit length that the noise outputs
	#[arg(long, value_name = "FLOAT")]
	frequency: Option<f32>,
}
impl From<CliNoise> for NoiseOptions {
	#[inline]
	fn from(cli: CliNoise) -> Self {
		Self {
			seed: cli.seed.unwrap_or_else(rand::random),
			frequency: cli.frequency.unwrap(),
		}
	}
}

fn main() {
	let mut cli = Cli::parse();
	cli.noise.frequency.get_or_insert_with(|| {
		let magnitude = cli.size.width.max(cli.size.height);
		(f64::from(magnitude) as f32).recip()
	});

	let pixel_init = PixelInit::from(cli.color);
	let noise_options = NoiseOptions::from(cli.noise);

	println!(
		"Generating '{}' with the following parameters:",
		cli.output.display()
	);
	/// Prints each argument passed by CLI
	macro_rules! print_args {
		($( $key:tt = $value:expr ),* $(,)?) => {
			$(
				println!("\t--{}={}", stringify!($key), $value);
			)*
		};
	}
	print_args! {
		size = cli.size,
		hue = cli.color.hue,
		saturation = cli.color.saturation,
		brightness = cli.color.brightness,
		seed = noise_options.seed,
		frequency = noise_options.frequency,
	};

	let image =
		random_gradient_generator::generate_image(cli.size, pixel_init, noise_options).unwrap();
	image.save(&cli.output).unwrap();
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cli() {
		use clap::CommandFactory;

		Cli::command().debug_assert();
	}
}
