use std::env;
use std::env::set_current_dir;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::process::{Command, Stdio};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use ulog_rs::builder::ULogParserBuilder;

#[derive(Debug)]
struct TestResult {
    input_file: PathBuf,
    result: Result<(), String>,
}

#[test]

fn test_ulog2csv() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let input_dir = current_dir.join("test_data/input");
    let output_dir = current_dir.join("test_data/output");

    let mut results = Vec::new();

    // Run the test for each .ulg file in input_dir
    for entry in fs::read_dir(input_dir).expect("Failed to read input dir") {
        let entry = entry.expect("Failed to read ulg input file");
        let input_path = entry.path();

        if input_path.extension() == Some(std::ffi::OsStr::new("ulg")) {
            let basename = input_path.file_stem().unwrap().to_str().unwrap();
            let output_path = output_dir.join(format!("{}_export.csv", basename));

            // Copy the input file to the temp dir
            let temp_input = std::env::temp_dir().join(format!("{}.ulg", basename));
            fs::copy(&input_path, &temp_input).expect("Failed to copy input file");

            // Run the `ulog2csv` example using `cargo run --example`
            let status = Command::new("cargo")
                .arg("run")
                .arg("--example")
                .arg("ulog2csv")
                .arg(temp_input.clone())
                .stderr(Stdio::inherit())
                .output()
                .expect("Failed to execute ulog2csv example");

            let result = if status.status.success() {
                let output_path_temp = std::env::temp_dir().join(format!("{}_export.csv", basename));
                if output_path_temp.exists() {
                    match compare_files(&output_path, &output_path_temp) {
                        Ok(true) => {
                            // Clean up output files
                            fs::remove_file(output_path_temp).expect("Failed to remove temp output file");
                            Ok(())
                        }
                        Ok(false) => Err(format!("Output CSV does not match expected for {}", basename)),
                        Err(e) => Err(format!("File comparison failed: {}", e)),
                    }
                } else {
                    Err(format!("Temporary output file not found for {}", basename))
                }
            } else {
                Err(format!("Failed to run ulog2csv example for {}", basename))
            };

            // Collect the result for this file
            results.push(TestResult {
                input_file: input_path,
                result,
            });
        }
    }

    let mut all_passed = true;

    // Report results
    println!("\nTest results:\n");
    for result in results {
        let status = match result.result {
            Ok(()) => "✅", // Success
            Err(err) => {
                all_passed = false;
                &format!("❌ {:?}", err)  // Failure
            }
        };

        // Extract the relative path after "test_data"
        let path_str = extract_relative_path(&result.input_file, "test_data");
        println!("test_ulog2csv: {:<50} {}", path_str, status);
    }

    assert!(all_passed);
}

fn ulog_cat(input_path: &Path) -> Result<Box<Path>, Box<dyn Error>> {
    let reader = BufReader::new(File::open(input_path)?);

    let parser = ULogParserBuilder::new(reader)
        .include_header(true)
        .include_timestamp(true)
        .include_padding(true)
        .build()?;

    let basename = input_path
        .file_stem().expect("Failed to get basename")
        .to_str().expect("Failed to convert basename to string");
    let output_filename = basename.to_owned() + "_export";
    
    // Create the output file path in the temp dir.
    let output_path = Path::new(&env::temp_dir()).join(output_filename).with_extension("ulg");
       
    // Open the output file for writing.
    let mut output_file = File::create(output_path.clone())?;

    for result in parser {
        let ulog_message = result?;

        let bytes:Vec<u8> = ulog_message.into();

        // Write the bytes to the output file in ULOG format.
        output_file.write_all(&bytes)?;
    }

    Ok(Box::from(output_path))
}


#[test]
fn test_ulogcat() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let input_dir = current_dir.join("test_data/input");
    let mut results = Vec::new();

    // Run `ulogcat` for each .ulg file in input_dir and assert the output file is identical to the input file.
    for entry in fs::read_dir(input_dir.clone()).expect("Failed to read input dir") {
        let entry = entry.expect("Failed to read ulg input file");
        let input_path = entry.path();

        let output_path = ulog_cat(&input_path).expect("call to ulog_cat() failed");

        let result =
            if output_path.exists() {
                match compare_files(&input_path, &output_path) {
                    Ok(true) => {
                        // Clean up the emitted file
                        fs::remove_file(output_path).expect("Failed to remove emitted file");
                        Ok(())
                    }
                    Ok(false) => Err(format!("Emitted ULOG file does not match input for {:?}", input_path)),
                    Err(e) => Err(format!("File comparison failed: {}", e)),
                }
            } else {
                Err(format!("Emitted file not found for {:?}", output_path))
            };
        
            // Collect the result for this file
            results.push(TestResult {
                input_file: input_path,
                result,
            });
        }

    let mut all_passed = true;
    
    // Report results
    println!("\nTest results:\n");
    for result in results {
        let status = match result.result {
            Ok(()) => "✅", // Success
            Err(err) => {
                all_passed = false;
                &format!("❌ {}", err)
            },  // Failure
        };
        let path_str = extract_relative_path(&result.input_file, "test_data");
        println!("test_ulogcat: {:<50} {}", path_str, status);
    }
    
    assert!(all_passed);
}

// Function to extract the relative path after the given component
fn extract_relative_path(file_path: &PathBuf, component: &str) -> String {
    if let Some(pos) = file_path.to_str().unwrap_or("").find(component) {
        // Add the component back as the prefix
        format!("{}{}", component, &file_path.to_str().unwrap_or("").split_at(pos + component.len()).1)
    } else {
        // If the component is not found, return the full path
        file_path.display().to_string()
    }
}

fn compare_files(file1: &std::path::Path, file2: &std::path::Path) -> io::Result<bool> {
    let mut f1 = fs::File::open(file1)?;
    let mut f2 = fs::File::open(file2)?;

    let mut buffer1 = [0; 4096]; // Compare in 4KB chunks
    let mut buffer2 = [0; 4096];

    loop {
        let bytes_read1 = f1.read(&mut buffer1)?;
        let bytes_read2 = f2.read(&mut buffer2)?;

        if bytes_read1 != bytes_read2 {
            return Ok(false); // Files have different lengths
        }

        if bytes_read1 == 0 {
            break; // Reached the end of both files
        }

        if buffer1[..bytes_read1] != buffer2[..bytes_read1] {
            return Ok(false); // Files differ in content
        }
    }

    Ok(true)
}
