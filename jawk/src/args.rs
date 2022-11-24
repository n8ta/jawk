use std::path::PathBuf;
use crate::PrintableError;

// TODO: Find a small library to do this

#[derive(Debug, PartialEq)]
pub struct AwkArgs {
    pub debug: bool,
    pub program: String,
    pub files: Vec<String>,
    pub save_executable: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
pub enum ProgramType {
    CLI(String),
    File(String),
}

impl ProgramType {
    pub fn load(self) -> Result<String, PrintableError> {
        match self {
            ProgramType::CLI(s) => Ok(s),
            ProgramType::File(s) =>
                {
                    match std::fs::read_to_string(&s) {
                        Ok(s) => Ok(s),
                        Err(e) => Err(PrintableError::new(format!("Unable to load source program '{}'\nGot error: {}", s, e)))
                    }
                },
        }
    }
}

fn print_help() {
    eprintln!(
        "\
Usage: llawk [--debug] [--save path] -f progfile file ...
Usage: llawk [--debug] [--save path] 'program' file ...
--dump: Dump the AST after parsing
--save file_path: Save the executable to the given path"
    );
}

impl AwkArgs {
    pub fn new(args: Vec<String>) -> Result<Self, PrintableError> {
        let mut debug = false;
        let mut program: Option<ProgramType> = None;
        let mut files: Vec<String> = vec![];
        let mut save_executable: Option<PathBuf> = None;

        let mut i = 1;
        while i < args.len() {
            match &args[i][..] {
                "--debug" => {
                    debug = true;
                    i += 1;
                }
                "--save" => {
                    if let Some(next) = args.get(i + 1) {
                        save_executable = Some(PathBuf::from(next));
                    } else {
                        return Err(PrintableError::new("Expected path after --save"))
                    }
                    i += 2;
                }
                "-f" => {
                    if program != None {
                        print_help();
                        return Err(PrintableError::new("Cannot specify multiple programs!"))
                    }
                    let next = match args.get(i + 1) {
                        None => {
                            print_help();
                            return Err(PrintableError::new("-f must be followed by a file name"));
                        }
                        Some(path) => path,
                    };
                    program = Some(ProgramType::File(next.to_string()));
                    i += 2;
                }
                _ => {
                    if program == None {
                        program = Some(ProgramType::CLI(args[i].clone()));
                    } else {
                        files.push(args[i].clone());
                    }
                    i += 1;
                }
            }
        }
        let program = match program {
            None => {
                print_help();
                return Err(PrintableError::new("You must specify a program either with -f file.awk or as an arg '$1 == 0 {{ print $1 }}'"));
            }
            Some(prog) => prog,
        };
        let program = program.load()?;
        Ok(AwkArgs {
            debug,
            program,
            files,
            save_executable,
        })
    }
}
