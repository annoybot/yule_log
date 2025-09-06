#[cfg(test)]
mod tests {
    use std::env;
    use std::error::Error;
    use std::fs;
    use std::fs::File;
    use std::io::{self, BufReader, Read, Write};
    use std::path::{Path, PathBuf};
    use crate::builder::ULogParserBuilder;
    use crate::encode::Encode;

    #[derive(Debug)]
    struct TestResult {
        input_file: PathBuf,
        result: Result<(), String>,
    }

    #[test]
    fn test_roundtrip() {
        let current_dir = env::current_dir().expect("Failed to get current directory");
        let input_dir = current_dir.join("test_data/input");
        let mut results = Vec::new();

        for entry in fs::read_dir(input_dir.clone()).expect("Failed to read input dir") {
            let entry = entry.expect("Failed to read ulg input file");
            let input_path = entry.path();

            let reader = BufReader::new(File::open(&input_path).expect("Failed to open input file"));

            let parser = ULogParserBuilder::new(reader)
                .include_header(true)
                .include_timestamp(true)
                .include_padding(true)
                .build()
                .expect("Failed to build parser");

            // Create output filename
            let basename = input_path
                .file_stem()
                .expect("Failed to get basename")
                .to_str()
                .expect("Failed to convert basename to string");
            let output_filename = basename.to_owned() + "_export";
            let output_filename = format!("test_{}", output_filename);
            
            let output_path = Path::new(&env::temp_dir())
                .join(output_filename)
                .with_extension("ulg");

            let mut output_file = File::create(&output_path).expect("Failed to create output file");

            for result in parser {
                let ulog_message = result.expect("Failed to parse message");

                // Use the new Encode trait's encode method directly
                ulog_message.encode(&mut output_file).expect("Encoding failed");
            }

            // Compare input and output files
            let result = if output_path.exists() {
                match compare_files(&input_path, &output_path) {
                    Ok(true) => {
                        // Clean up emitted file
                        fs::remove_file(&output_path).expect("Failed to remove emitted file");
                        Ok(())
                    }
                    Ok(false) => Err(format!("Emitted ULOG file does not match input for {:?}", input_path)),
                    Err(e) => Err(format!("File comparison failed: {}", e)),
                }
            } else {
                Err(format!("Emitted file not found for {:?}", output_path))
            };

            results.push(TestResult {
                input_file: input_path,
                result,
            });
        }

        let mut all_passed = true;

        println!("\nTest results:\n");
        for result in results {
            let status = match &result.result {
                Ok(()) => "✅",
                Err(err) => {
                    all_passed = false;
                    &format!("❌ {}", err)
                }
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
}