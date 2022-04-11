use lazy_static::lazy_static;
use std::fs::File;
use std::fs::remove_file;
use std::io::BufRead;
use std::io::BufReader;
use std::panic::catch_unwind;
use std::panic::UnwindSafe;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use test_case::test_case;

// It is unbelievably annoying how Rust's testing system doesn't have setup or
// teardown functionality.

lazy_static! {
    static ref LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

/// Tears down the post-test-execution state.
pub fn teardown() {
    let _ = remove_file("tests/output.cube");

    for index in 0..=10 {
        let _ = remove_file(format!("tests/output_{}.cube", index));
        let _ = remove_file(format!("tests/output_{}m.cube", index));
    }
}

/// Runs the given test in sequence.
/// 
/// This function runs the given tests in sequence and tears down the post-test-
/// execution state afterwards. Because this function catches any panics that
/// might occur in the test, provided that Rust doesn't abort when panicking,
/// the `teardown` function will always run after the test runs, regardless of
/// the result of the test. Tests run by this function will always occur in
/// sequence, even if you forget to run `cargo test` with `--test-threads=1`.
#[allow(clippy::unused_unit)]
pub fn run(test: impl FnOnce() -> () + UnwindSafe) {
    let _lock = LOCK.lock().unwrap_or_else(|error| error.into_inner());

    let result = catch_unwind(test);

    teardown();

    result.unwrap();
}

/// Tests that the given argument configurations execute successfully. But
/// without actually doing any serious computation.
#[allow(clippy::unused_unit)]
#[test_case("--version")]
#[test_case("--help")]
#[test_case("-v")]
#[test_case("-h")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -s 2 -t 1")]
#[test_case("-p AdobeRGB1998 -o tests/output.cube -c 1 2 3 -s 2 -t 1")]
#[test_case("-p aDObErgB1998 -o tests/output.cube -c 1 2 3 -s 2 -t 1" ; "profile_case_insensitive")]
#[test_case("-p Rec709 -o tests/output.cube -c 1 2 3 -s 2 -t 1")]
pub fn test_success(arguments: &str) {
    run(|| {
        let mut process = Command::new("cargo");
        process.args(&["run", "--"]);
        process.args(arguments.split(' '));

        let output = process.output().unwrap();

        assert!(output.status.success());
    });
}

/// Tests that the given invalid arguments are correctly recognized as invalid.
#[allow(clippy::unused_unit)]
#[test_case(""                                                           ; "empty")]
#[test_case("-o tests/output.cube -c 1 2 3"                              ; "profile_missing")]
#[test_case("-p sRGB -c 1 2 3"                                           ; "output_missing")]
#[test_case("-p sRGB -o tests/output.cube"                               ; "primary_missing")]
#[test_case("-p -o tests/output.cube -c 1 2 3"                           ; "profile_missing_argument")]
#[test_case("-p sRGB -o -c 1 2 3"                                        ; "output_missing_argument")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2"                        ; "primary_missing_argument_1")]
#[test_case("-p sRGB -o tests/output.cube -c 1"                          ; "primary_missing_argument_2")]
#[test_case("-p sRGB -o tests/output.cube -c"                            ; "primary_missing_argument_3")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -s"                   ; "size_missing_argument")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -l"                   ; "inklimit_missing_argument")]
#[test_case("-p no_such_profile -o tests/output.cube -c 1 2 3"           ; "profile_not_found")]
#[test_case("-p tests/USWebCoatedSWOP.icc -o tests/output.cube -c 1 2 3" ; "profile_not_rgb")]
#[test_case("-p sRGB -o tests/output.cube -c not_a_number 2 3"           ; "primary_not_number")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -s not_a_number"      ; "size_not_number")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -s 1"                 ; "size_illegal")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -t not_a_number"      ; "target_not_number")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -t 0"                 ; "target_illegal")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -l not_a_number"      ; "inklimit_not_number")]
#[test_case("-p sRGB -o tests/output.cube -c 1 2 3 -l -0.5"              ; "inklimit_illegal")]
pub fn test_bad_arguments(arguments: &str) {
    run(|| {
        let mut process = Command::new("cargo");
        process.args(&["run", "--"]);
        process.args(arguments.split(' '));

        let output = process.output().unwrap();

        assert!(!output.status.success());
    });
}

/// Tests that the program's output (with the given inputs) is identical, within
/// a given tolerance, to a given reference output.
#[allow(clippy::unused_unit)]
#[test_case(
    // Reference generated with 
    // `-p sRGB -o tests/rebeccapurple.cube -c 102 51 153 -s 16` with a
    // generated secondaries goal of 100 000 000.
    "-p sRGB -c 102 51 153 -s 16 -t 10000",
    vec![
        ("tests/rebeccapurple.cube", "tests/output.cube"),
        ("tests/rebeccapurple_0.cube", "tests/output_0.cube"),
        ("tests/rebeccapurple_0m.cube", "tests/output_0m.cube"),
    ],
    5e-4
; "rebeccapurple")]
// #[test_case(
//     // Reference generated with 
//     // `-p sRGB -o tests/cmyk.cube -c 0 174 239 -c 236 0 140 -c 255 242 0 -c 35 31 32 -l 3 -s 16`
//     // with a generated secondaries goal of 100 000 000.
//     "-p sRGB -c 0 174 239 -c 236 0 140 -c 255 242 0 -c 35 31 32 -l 3 -s 16 -t 10000",
//     vec![
//         ("tests/cmyk.cube", "tests/output.cube"),
//         ("tests/cmyk_0.cube", "tests/output_0.cube"),
//         ("tests/cmyk_0m.cube", "tests/output_0m.cube"),
//         ("tests/cmyk_1.cube", "tests/output_1.cube"),
//         ("tests/cmyk_1m.cube", "tests/output_1m.cube"),
//         ("tests/cmyk_2.cube", "tests/output_2.cube"),
//         ("tests/cmyk_2m.cube", "tests/output_2m.cube"),
//         ("tests/cmyk_3.cube", "tests/output_3.cube"),
//         ("tests/cmyk_3m.cube", "tests/output_3m.cube"),
//     ],
//     5e-1
// ; "cmyk")]
#[ignore]
pub fn test_output(arguments: &str, comparisons: Vec<(&str, &str)>, tolerance: f32) {
    run(|| {
        let mut process = Command::new("cargo");
        process.args(&["run", "--", "-o", "tests/output.cube"]);
        process.args(arguments.split(' '));

        assert!(process.output().unwrap().status.success());

        for (path_reference, path_result) in comparisons.iter() {
            /// Parses the given path as a 3D LUT.
            fn parse_lut(path: impl AsRef<Path>) -> Vec<[f32; 3]> {
                eprintln!("{}", path.as_ref().display());
                let read = BufReader::new(File::open(&path).unwrap());

                let mut colors = Vec::with_capacity(16_usize.pow(3));

                for (index, line) in read.lines().map(Result::unwrap).enumerate() {
                    if index >= 3 {
                        let components = line.split(' ');
                        let components = components.map(str::parse);
                        let components = components.map(Result::unwrap);
                        let components = components.collect::<Vec<f32>>();

                        assert_eq!(3, components.len());

                        colors.push([components[0], components[1], components[2]]);
                    }
                }

                colors
            }

            let reference = parse_lut(path_reference);
            let result = parse_lut(path_result);

            assert_eq!(reference.len(), result.len());

            for (index, (color_reference, color_result)) in reference.into_iter().zip(result.into_iter()).enumerate() {
                for (component_reference, component_result) in color_reference.iter().zip(color_result.iter()) {
                    assert!((component_reference - component_result).abs() <= tolerance,
                        "{}:{}: {} !~= {} (+/- {})",
                        path_reference,
                        index + 4,
                        component_reference,
                        component_result,
                        tolerance
                    );
                }
            }
        }
    });
}