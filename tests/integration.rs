extern crate assert_cli;
extern crate serde_json;

#[cfg(test)]
mod integration {
    use assert_cli;
    use std::fs::{File};
    use std::path::Path;
    use std::io::prelude::*;
    use std::fs::{self};
    use std::panic;
    use serde_json;
    use serde_json::Value;

    fn setup() {
        fs::create_dir("./tests/tmp").unwrap();
    }

    fn teardown() {
        fs::remove_dir_all("./tests/tmp").unwrap();
    } 
    
    #[test]
    fn run_integration_tests() {
        // see https://medium.com/@ericdreichert/test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab
        setup();

        let result = panic::catch_unwind(|| {
            test_compile_and_witness_dir()
        });

        teardown();

        assert!(result.is_ok())
    }

    fn test_compile_and_witness_dir() {
        let dir = Path::new("./tests/code");
        if dir.is_dir() {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.extension().unwrap() == "witness" {
                    let program_name = Path::new(Path::new(path.file_stem().unwrap()).file_stem().unwrap());
                    let prog = dir.join(program_name).with_extension("code");
                    let flat = dir.join(program_name).with_extension("expected.out.code");
                    let witness = dir.join(program_name).with_extension("expected.witness");
                    let args = dir.join(program_name).with_extension("arguments.json");
                    test_compile_and_witness(program_name.to_str().unwrap(), &prog, &flat, &args, &witness);
                }
            }
        }
    }

    fn test_compile_and_witness(program_name: &str, program_path: &Path, expected_flattened_code_path: &Path, arguments_path: &Path, expected_witness_path: &Path) {
        let tmp_base = Path::new("./tests/tmp/");
        let test_case_path = tmp_base.join(program_name);
    	let flattened_path = tmp_base.join(program_name).join("out");
    	let flattened_code_path = tmp_base.join(program_name).join("out").with_extension("code");
    	let witness_path = tmp_base.join(program_name).join("witness");
        let verification_key_path = tmp_base.join(program_name).join("verification").with_extension("key");
        let proving_key_path = tmp_base.join(program_name).join("proving").with_extension("key");
        let variable_information_path = tmp_base.join(program_name).join("variables").with_extension("inf");
        let verification_contract_path = tmp_base.join(program_name).join("verifier").with_extension("sol");

        // create a tmp folder to store artifacts
        fs::create_dir(test_case_path).unwrap();

        // COMPILE
        assert_cli::Assert::command(&["cargo", "run", "--", "compile",
            "-i", program_path.to_str().unwrap(),
            "-o", flattened_path.to_str().unwrap()])
            .succeeds()
            .unwrap();

        // load the expected result
        let mut expected_flattened_code_file = File::open(&expected_flattened_code_path).unwrap();
        let mut expected_flattened_code = String::new();
        expected_flattened_code_file.read_to_string(&mut expected_flattened_code).unwrap();

        // load the actual result
        let mut flattened_code_file = File::open(&flattened_code_path).unwrap();
        let mut flattened_code = String::new();
        flattened_code_file.read_to_string(&mut flattened_code).unwrap();

        // check equality
        assert_eq!(flattened_code, expected_flattened_code, "Flattening failed for {}\n\nExpected\n\n{}\n\nGot\n\n{}", program_path.to_str().unwrap(), expected_flattened_code.as_str(), flattened_code.as_str());

        // SETUP
        assert_cli::Assert::command(&["cargo", "run", "--", "setup",
            "-i", flattened_path.to_str().unwrap(),
            "-p", proving_key_path.to_str().unwrap(),
            "-v", verification_key_path.to_str().unwrap(),
            "-m", variable_information_path.to_str().unwrap()])
            .succeeds()
            .unwrap();

        // EXPORT-VERIFIER
        assert_cli::Assert::command(&["cargo", "run", "--", "export-verifier",
            "-i", verification_key_path.to_str().unwrap(),
            "-o", verification_contract_path.to_str().unwrap()])
            .succeeds()
            .unwrap();

        // COMPUTE_WITNESS
        let arguments: Value = serde_json::from_reader(File::open(arguments_path).unwrap()).unwrap();

        let arguments_str_list: Vec<String> = arguments.as_array().unwrap().iter().map(|i| match *i {
            Value::Number(ref n) => n.to_string(),
            _ => panic!(format!("Cannot read arguments. Check {}", arguments_path.to_str().unwrap()))
        }).collect();

        let mut compute = vec!["cargo", "run", "--", "compute-witness",
            "-i", flattened_path.to_str().unwrap(),
            "-o", witness_path.to_str().unwrap(),
            "-a"];

        for arg in arguments_str_list.iter() {
            compute.push(arg);
        }
        
        assert_cli::Assert::command(&compute)
            .succeeds()
            .unwrap();

		// load the expected witness
		let mut expected_witness_file = File::open(&expected_witness_path).unwrap();
		let mut expected_witness = String::new();
		expected_witness_file.read_to_string(&mut expected_witness).unwrap();

		// load the actual witness
    	let mut witness_file = File::open(&witness_path).unwrap();
        let mut witness = String::new();
		witness_file.read_to_string(&mut witness).unwrap();

		// check equality
		assert!(witness.contains(
            expected_witness.as_str()),
            "Witness generation failed for {}\n\nExpected\n\n{}\n\nGot\n\n{}",
                program_path.to_str().unwrap(),
                expected_witness.as_str(),
                witness.as_str());

        // GENERATE-PROOF
        assert_cli::Assert::command(&["cargo", "run", "--", "generate-proof",
            "-w", witness_path.to_str().unwrap(),
            "-p", proving_key_path.to_str().unwrap(),
            "-i", variable_information_path.to_str().unwrap()])
            .succeeds()
            .unwrap();
    }
}