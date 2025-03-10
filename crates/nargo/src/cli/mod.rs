pub use build_cmd::build_from_path;
use clap::{App, Arg};
use noirc_driver::Driver;
use noirc_frontend::graph::{CrateName, CrateType};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
extern crate tempdir;
use tempdir::TempDir;

use crate::errors::CliError;

mod build_cmd;
mod compile_cmd;
mod contract_cmd;
mod gates_cmd;
mod new_cmd;
mod prove_cmd;
mod verify_cmd;

const CONTRACT_DIR: &str = "contract";
const PROOFS_DIR: &str = "proofs";
const PROVER_INPUT_FILE: &str = "Prover";
const VERIFIER_INPUT_FILE: &str = "Verifier";
const SRC_DIR: &str = "src";
const PKG_FILE: &str = "Nargo.toml";
const PROOF_EXT: &str = "proof";
const BUILD_DIR: &str = "build";
const ACIR_EXT: &str = "acir";
const WITNESS_EXT: &str = "tr";

pub fn start_cli() {
    let matches = App::new("nargo")
        .about("Noir's package manager")
        .version("0.1")
        .author("Kevaundray Wedderburn <kevtheappdev@gmail.com>")
        .subcommand(App::new("build").about("Builds the constraint system"))
        .subcommand(App::new("contract").about("Creates the smart contract code for circuit"))
        .subcommand(
            App::new("new")
                .about("Create a new binary project")
                .arg(Arg::with_name("package_name").help("Name of the package").required(true))
                .arg(
                    Arg::with_name("path").help("The path to save the new project").required(false),
                ),
        )
        .subcommand(
            App::new("verify")
                .about("Given a proof and a program, verify whether the proof is valid")
                .arg(Arg::with_name("proof").help("The proof to verify").required(true)),
        )
        .subcommand(
            App::new("prove")
                .about("Create proof for this program")
                .arg(Arg::with_name("proof_name").help("The name of the proof").required(true))
                .arg(
                    Arg::with_name("show-ssa")
                        .long("show-ssa")
                        .help("Emit debug information for the intermediate SSA IR"),
                ),
        )
        .subcommand(
            App::new("compile")
                .about("Compile the program and its secret execution trace into ACIR format")
                .arg(
                    Arg::with_name("circuit_name").help("The name of the ACIR file").required(true),
                )
                .arg(
                    Arg::with_name("witness")
                        .long("witness")
                        .help("Solve the witness and write it to file along with the ACIR"),
                ),
        )
        .subcommand(
            App::new("gates").about("Counts the occurences of different gates in circuit").arg(
                Arg::with_name("show-ssa")
                    .long("show-ssa")
                    .help("Emit debug information for the intermediate SSA IR"),
            ),
        )
        .get_matches();

    let result = match matches.subcommand_name() {
        Some("new") => new_cmd::run(matches),
        Some("build") => build_cmd::run(matches),
        Some("contract") => contract_cmd::run(matches),
        Some("prove") => prove_cmd::run(matches),
        Some("compile") => compile_cmd::run(matches),
        Some("verify") => verify_cmd::run(matches),
        Some("gates") => gates_cmd::run(matches),
        None => Err(CliError::Generic("No subcommand was used".to_owned())),
        Some(x) => Err(CliError::Generic(format!("unknown command : {}", x))),
    };
    if let Err(err) = result {
        err.write()
    }
}

fn create_dir<P: AsRef<Path>>(dir_path: P) -> Result<PathBuf, std::io::Error> {
    let mut dir = std::path::PathBuf::new();
    dir.push(dir_path);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn create_named_dir(named_dir: &Path, name: &str) -> PathBuf {
    create_dir(named_dir).unwrap_or_else(|_| panic!("could not create the `{}` directory", name))
}

fn write_to_file(bytes: &[u8], path: &Path) -> String {
    let display = path.display();

    let mut file = match File::create(path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    match file.write_all(bytes) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => display.to_string(),
    }
}

// helper function which tests noir programs by trying to generate a proof and verify it
pub fn prove_and_verify(proof_name: &str, prg_dir: &Path, show_ssa: bool) -> bool {
    let tmp_dir = TempDir::new("p_and_v_tests").unwrap();
    let proof_path =
        match prove_cmd::prove_with_path(proof_name, prg_dir, &tmp_dir.into_path(), show_ssa) {
            Ok(p) => p,
            Err(CliError::Generic(msg)) => {
                println!("Error: {}", msg);
                return false;
            }
            Err(CliError::DestinationAlreadyExists(str)) => {
                println!("Error, destination {} already exists: ", str);
                return false;
            }
        };

    verify_cmd::verify_with_path(prg_dir, &proof_path, show_ssa).unwrap()
}

fn add_std_lib(driver: &mut Driver) {
    let path_to_std_lib_file = path_to_stdlib().join("lib.nr");
    let std_crate = driver.create_non_local_crate(path_to_std_lib_file, CrateType::Library);
    let std_crate_name = "std";
    driver.propagate_dep(std_crate, &CrateName::new(std_crate_name).unwrap());
}

fn path_to_stdlib() -> PathBuf {
    dirs::config_dir().unwrap().join("noir-lang").join("std/src")
}

// FIXME: I not sure that this is the right place for this tests.
#[cfg(test)]
mod tests {
    use noirc_driver::Driver;
    use noirc_frontend::graph::CrateType;

    use std::path::{Path, PathBuf};

    const TEST_DATA_DIR: &str = "tests/compile_tests_data";

    /// Compiles a file and returns true if compilation was successful
    ///
    /// This is used for tests.
    pub fn file_compiles<P: AsRef<Path>>(root_file: P) -> bool {
        let mut driver = Driver::new();
        driver.create_local_crate(&root_file, CrateType::Binary);
        super::add_std_lib(&mut driver);
        driver.file_compiles()
    }

    #[test]
    fn compilation_pass() {
        let mut pass_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        pass_dir.push(&format!("{TEST_DATA_DIR}/pass"));

        let paths = std::fs::read_dir(pass_dir).unwrap();
        for path in paths.flatten() {
            let path = path.path();
            assert!(file_compiles(&path), "path: {}", path.display());
        }
    }

    #[test]
    fn compilation_fail() {
        let mut fail_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fail_dir.push(&format!("{TEST_DATA_DIR}/fail"));

        let paths = std::fs::read_dir(fail_dir).unwrap();
        for path in paths.flatten() {
            let path = path.path();
            assert!(!file_compiles(&path), "path: {}", path.display());
        }
    }
}
