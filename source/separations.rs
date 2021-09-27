mod vector;

use lazy_static::lazy_static;
use lcms2::ColorSpaceSignature;
use lcms2::Intent;
use lcms2::PixelFormat;
use lcms2::Profile;
use lcms2::Transform;
use maplit::hashmap;
use rstar::primitives::GeomWithData;
use rstar::RTree;
use std::collections::HashMap;
use std::env::args;
use std::fs::File;
use std::io;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use std::sync::Mutex;
use threadpool::ThreadPool;
use unicase::UniCase;

use vector::Vector3;

/// The default output 3D LUT size. A value of 64 is typical in professional
/// settings.
const SIZE_DEFAULT: usize = 64;

/// The default target number. A value of 100 000 000 provides a nice balance
/// between the accuracy of the results and the program finishing within a
/// reasonable amount of time.
const TARGET_DEFAULT: usize = 100_000_000;

/// The default ink limit. A value of infinity results in the program applying
/// no ink limit restrictions on the generated secondary colors.
const INKLIMIT_DEFAULT: f32 = f32::INFINITY;

lazy_static! {
    /// The available named color profiles.
    static ref DATA_PROFILES: HashMap<UniCase<&'static str>, &'static [u8]> = hashmap! {
        // UniCase::new("sRGB") => sRGB will be handled as a special case.
        UniCase::new("AdobeRGB1998") => include_bytes!("profiles/AdobeRGB1998.icc").as_ref(),
        UniCase::new("Rec709") => include_bytes!("profiles/Rec709.icc").as_ref(),
        // TODO Rec2020
        // TODO DCIP3
        // TODO ProPhotoRGB
    };
}

fn main() {
    /// Errors out of the program, printing the given message as an error
    /// message.
    fn errorout(message: impl AsRef<str>) -> ! {
        eprintln!("\x1B[41m Error \x1B[0m {} See \x1B[93m--help\x1B[0m for usage documentation.", message.as_ref());

        exit(1);
    }

    let mut profile = None;
    let mut path_output = None;
    let mut primaries = Vec::with_capacity(4);
    let mut size = SIZE_DEFAULT;
    let mut target = TARGET_DEFAULT;
    let mut inklimit = INKLIMIT_DEFAULT;

    // Parse command line arguments. I probably could have saved myself a lot of
    // effort by using some preexisting argument parsing library, but this
    // application has some kinda weird requirements regarding its arguments,
    // and Clap is a real big bastard of a library, so no choice but to reinven
    // the wheel.
    let mut arguments = args();
    arguments.next();
    while let Some(argument) = arguments.next() {
        // Obtains the next command line argument and returns it, erroring out
        // if no next argument exists.
        let mut argument_next = || -> String {
            arguments.next().unwrap_or_else(||
                errorout(format!("Missing argument for \x1B[93m{}\x1B[0m.", argument))
            )
        };

        match argument.to_ascii_lowercase().as_str() {
            // Version
            "-v" | "--version" => {
                println!("Color Separations {}", env!("CARGO_PKG_VERSION"));
                return;
            },
            // Help
            "-h" | "--help" | "-?" => {
                println!("\n{}\n", include_str!("help.txt"));
                return;
            },

            // Profile
            "-p" | "--profile" => {
                profile = Some(match argument_next() {
                    // sRGB
                    identifier if identifier.eq_ignore_ascii_case("sRGB") =>
                        Profile::new_srgb(),

                    // Named profile (besides sRGB)
                    identifier if DATA_PROFILES.contains_key(&UniCase::new(identifier.as_str())) => {
                        let data_profile = DATA_PROFILES[&UniCase::new(identifier.as_str())];

                        Profile::new_icc(data_profile).unwrap()
                    },
                    // identifier if let Some(data_profile) = DATA_PROFILES.get(&UniCase::new(identifier.clone().as_str())) =>
                    //     Profile::new_icc(data_profile).unwrap(),

                    // From the filesystem
                    path => {
                        let profile = Profile::new_file(&path).unwrap_or_else(|error|
                            errorout(format!("Could not read ICC profile file \x1B[96m{}\x1B[0m: {}.", path, error))
                        );
            
                        if profile.color_space() != ColorSpaceSignature::RgbData {
                            errorout("Only RGB ICC profiles are supported.");
                        }
            
                        profile
                    },
                });
            },
            // Output location
            "-o" | "--output" => {
                path_output = Some(PathBuf::from(argument_next()));
            },
            // Primary color
            "-c" | "--color" => {
                // Parses the given component.
                let parse_component = |component: String| {
                    component.parse::<f32>().unwrap_or_else(|_| {
                        errorout("Primary color component must be a number.")
                    })
                };

                primaries.push(Vector3([
                    parse_component(argument_next()),
                    parse_component(argument_next()),
                    parse_component(argument_next())
                ]));
            },

            // 3D LUT size
            "-s" | "--size" => {
                size = argument_next().parse::<usize>().unwrap_or_else(|_| {
                    errorout("3D LUT size must be an integer greater than or equal to 2.")
                });

                if size < 2 {
                    errorout("3D LUT size must be an integer greater than or equal to 2.");
                }
            },
            // Target secondaries
            "-t" | "--target" => {
                target = argument_next().parse::<usize>().unwrap_or_else(|_| {
                    errorout("Target number must be a positive integer.")
                });

                if target < 1 {
                    errorout("Target number must be a positive integer.");
                }
            },
            // Ink limit
            "-l" | "--limit" => {
                inklimit = argument_next().parse::<f32>().unwrap_or_else(|_| {
                    errorout("Ink limit must be non-negative number.")
                });

                if inklimit < 0.0 {
                    errorout("Ink limit must be non-negative number.");
                }
            },
            
            // Unknown option
            option => {
                errorout(format!("Unknown option \x1B[93m{}\x1B[0m.", option));
            },
        }
    }

    // Verify all of the mandatory arguments have been specified.
    let profile = profile.unwrap_or_else(||
        errorout("No ICC profile was specified. Use \x1B[93m--profile\x1B[0m to specify an ICC profile.")
    );
    let path_output = path_output.unwrap_or_else(||
        errorout("No output file was specified. Use \x1B[93m--output\x1B[0m to specify an output file.")
    );
    if primaries.is_empty() {
        errorout("No primary colors were specified. Use \x1B[93m--color\x1B[0m to specify a primary color.");
    }
    
    // TODO what should resolution be?
    let count_colors_lut = size.pow(3);
    let resolution = (target as f64).powf(1.0 / primaries.len() as f64).ceil() as usize;
    let count_secondaries = resolution.pow(primaries.len() as u32);

    // Prepare profile transformations.
    let profile_xyz = Profile::new_xyz();
    let transformation_reverse = Transform::new(
        &profile,
        PixelFormat::RGB_FLT,
        &profile_xyz,
        PixelFormat::XYZ_FLT,
        Intent::AbsoluteColorimetric
    ).unwrap();
    let transformation_forward = Transform::new(
        &profile_xyz,
        PixelFormat::XYZ_FLT,
        &profile,
        PixelFormat::RGB_FLT,
        Intent::AbsoluteColorimetric
    ).unwrap();
    
    // Returns a new output record.
    let new_output = |path: &PathBuf| -> (File, Vec<Vector3>) {
        let file = File::create(path).unwrap_or_else(|error|
            errorout(format!("Could not create output 3D LUT file \x1B[96m{}\x1B[0m: {}.", path.display(), error))
        );

        (file, Vec::with_capacity(count_colors_lut))
    };

    let mut outputs = Vec::with_capacity(1 + 2 * primaries.len());

    let extension = path_output.extension().unwrap_or_default();
    let stem = path_output.file_stem().unwrap();

    outputs.push(new_output(&path_output));

    // Prepare a primary and mask 3D LUT for each primary color.
    for index_component in 0..primaries.len() {
        let mut stem_component = stem.to_os_string();
        stem_component.push("_");
        stem_component.push(index_component.to_string());

        let mut path_component_main = path_output.with_file_name(&stem_component);
        path_component_main.set_extension(&extension);
        
        stem_component.push("m");

        let mut path_component_mask = path_output.with_file_name(&stem_component);
        path_component_mask.set_extension(&extension);

        outputs.push(new_output(&path_component_main));
        outputs.push(new_output(&path_component_mask));
    }

    // TODO get component type, is 255 really it? Maybe someone wants to specify
    // colors from a range of 0-1?
    for primary in primaries.iter_mut() {
        *primary /= 255.0;
    }
    
    // Generate the origin 3D LUT colors in their correct order.
    let mut colors_lut = Vec::with_capacity(count_colors_lut);
    for index_blue in 0..size {
        let component_blue = (index_blue % size) as f32 / (size - 1) as f32;

        for index_green in 0..size {
            let component_green = (index_green % size) as f32 / (size - 1) as f32;

            for index_red in 0..size {
                let component_red = (index_red % size) as f32 / (size - 1) as f32;

                colors_lut.push(Vector3([component_red, component_green, component_blue]));
            }
        }
    }

    let mut white = vec![Vector3([1.0, 1.0, 1.0])];

    // Move all of the colors into XYZ space.
    // TODO probably not worth multithreading this but maybe?
    transformation_reverse.transform_in_place(&mut primaries);
    transformation_reverse.transform_in_place(&mut white);
    transformation_reverse.transform_in_place(&mut colors_lut);
    let white = white[0];

    // Mix the primary colors together, applying subtractive color mixing.
    // There's probably an algorithm superior to the one used below, one that
    // can optimize for small ink limits. It is almost certainly not worth
    // trying to find it. This area of code is not likely to benefit a lot from
    // multithreading, so I'm not gonna bother.
    let mut secondaries = Vec::with_capacity(count_secondaries);
    'secondaries: for mut number in 0..count_secondaries {
        let mut secondary = white;
        let mut components = Vec::with_capacity(primaries.len());
        let mut total = 0.0;

        for primary in primaries.iter() {
            let fraction = (number % resolution) as f32 / (resolution - 1) as f32;

            total += fraction;
            
            // Current secondary color violates the ink limit. Immediately
            // abandon this particular mixture of primaries.
            if total > inklimit {
                continue 'secondaries;
            }

            secondary *= (fraction * *primary + (1.0 - fraction) * white) / white;

            components.push(fraction);

            number /= resolution;
        }

        secondaries.push(GeomWithData::new(secondary, (secondary, components)));
    }

    // Populate the RTree.
    let rtree = RTree::bulk_load(secondaries);

    let count_threads = num_cpus::get();
    let threadpool = ThreadPool::new(count_threads);

    let arc_results = Arc::new(Mutex::from(vec![Vec::new(); count_threads])); // TODO pointless initialized memory
    let arc_primaries = Arc::new(primaries);
    let arc_colors_lut = Arc::new(colors_lut);
    let arc_rtree = Arc::new(rtree);

    for index_thread in 0..count_threads {
        let results = arc_results.clone();
        let primaries = arc_primaries.clone();
        let colors_lut = arc_colors_lut.clone();
        let rtree = arc_rtree.clone();

        threadpool.execute(move || {
            let start = index_thread * colors_lut.len() / count_threads;
            let end = (index_thread + 1) * colors_lut.len() / count_threads;

            let mut result = vec![Vec::with_capacity(end - start); 1 + 2 * primaries.len()];

            // Generate 3D LUTs for this thread's designated allocation.
            for index in start..end {
                let color_lut = colors_lut[index];

                let data_secondary = rtree.nearest_neighbor(&color_lut).unwrap();

                let (secondary, components) = &data_secondary.data;

                result[0].push(*secondary);

                for index_primary in 0..primaries.len() {
                    let primary = primaries[index_primary];
                    let fraction = components[index_primary];

                    let color = fraction * primary + (1.0 - fraction) * white;

                    result[2 * index_primary + 1].push(color);
                    result[2 * index_primary + 2].push(Vector3([fraction, fraction, fraction]));
                }
            }

            let mut results = results.lock().unwrap();
            results[index_thread] = result;
        });
    }

    threadpool.join();

    // Combine individual thread results into complete 3D LUTs.
    for result in Arc::try_unwrap(arc_results).unwrap().into_inner().unwrap() {
        for (index, mut result_output) in result.into_iter().enumerate() {
            outputs[index].1.append(&mut result_output);
        }
    }

    // Apply forward color transformations to the 3D LUTs that require it.
    transformation_forward.transform_in_place(&mut outputs[0].1);
    for index_output in (1..outputs.len()).step_by(2) {
        transformation_forward.transform_in_place(&mut outputs[index_output].1);
    }

    // Write the 3D LUT files.
    for (file_output, colors_output) in outputs {
        let mut output = BufWriter::new(file_output);

        // I mean, it's kinda like a try-catch block, right?
        (|| {
            writeln!(output, "LUT_3D_SIZE {}", size)?;
            writeln!(output, "DOMAIN_MIN 0 0 0")?;
            writeln!(output, "DOMAIN_MAX 1 1 1")?;
    
            for color in colors_output {
                /// Clamps the given value between 0 and 1. This function won't
                /// be necessary once clamp is stabilized. Assuming it ever gets
                /// stabilized. You'd think something as simple as that wouldn't
                /// cause a whole lot of drama, but you'd be wrong.
                fn clamp(value: f32) -> f32 {
                    if value <= 0.0 {
                        0.0
                    } else if value > 1.0 {
                        1.0
                    } else {
                        value
                    }
                }
    
                writeln!(output, "{} {} {}",
                    clamp(color[0]),
                    clamp(color[1]),
                    clamp(color[2])
                )?;
            }

            Ok(())
        })().unwrap_or_else(|error: io::Error|
            errorout(format!("Encountered an IO error: {}.", error))
        );
    }
}