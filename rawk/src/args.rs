use crate::{lex, PrintableError, Symbolizer};
use std::path::PathBuf;
use std::ptr::eq;
use mawk_regex::Regex;
use crate::awk_str::AwkStr;
use crate::compiler::validate_program;
use crate::lexer::escaped_string_reader;

// TODO: Find a small library to do this


const ASSIGNMENT_REGEX: &'static str = "[_a-Z][_a-Z0-9]=";

#[derive(Debug, PartialEq)]
pub struct AwkArgs {
    pub debug: bool,
    pub program: String,
    pub files: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum ProgramType {
    CLI(String),
    File(Vec<String>),
}

impl ProgramType {
    // pub fn load(self) -> Result<String, PrintableError> {
    //     match self {
    //         ProgramType::CLI(s) => Ok(s),
    //         ProgramType::File(s) => match std::fs::read_to_string(&s) {
    //             Ok(s) => Ok(s),
    //             Err(e) => Err(PrintableError::new(format!("Unable to load source program '{}'\nGot error: {}", s, e))),
    //         },
    //     }
    // }
}

fn print_help() {
    eprintln!(
        "\
Usage: rawk [--debug] [-F sepstring] [-v assignment] ... program [argument...]
Usage: rawk [--debug] [-F sepstring] -f progfile [-f progfile] ... [-v assignment] ... [argument...]
--debug: Dump the AST, bytecode/metadata, and more.
-F       : Set the field separator
-v       : Set a variable eg. -v X=4
-f       : Specify an program file eg. -f prog.awk
argument : Either a file or an assignment. Eg. input_file.data or X=4
program  : Body of the awk program
"
    );
}

struct AwkArgBuilder {
    argv: Vec<String>,
    fieldsep: Option<String>,
    assignments: Vec<(String, AwkStr)>,
    program: Option<ProgramType>,
    assignment_regex: Regex,
}

impl AwkArgBuilder {
    pub fn new() -> Self {
        Self {
            argv: vec![],
            fieldsep: None,
            assignments: vec![],
            program: None,
            assignment_regex: Regex::new(ASSIGNMENT_REGEX.as_bytes()),
        }
    }
    pub fn drop(self) -> Result<AwkArgs, PrintableError> {
        todo!()
    }
    pub fn add_inline_program(&mut self, program: String) -> Result<(), PrintableError> {
        if self.program.is_none() {
            self.program = Some(ProgramType::CLI(program));
            Ok(())
        } else {
            Err(PrintableError::new("Awk does not allow mixing inline programs and -f programs loaded from files"))
        }
    }
    pub fn add_program_source_file(&mut self, file: String) -> Result<(), PrintableError> {
        match &mut self.program {
            None => {
                self.program = Some(ProgramType::File(vec![file]));
            }
            Some(existing) => {
                match existing {
                    ProgramType::CLI(_) => {
                        return Err(PrintableError::new("Awk does not allow mixing inline programs and -f programs loaded from files"));
                    }
                    ProgramType::File(files) => {
                        files.push(file)
                    }
                }
            }
        }
        Ok(())
    }
    pub fn add_fieldsep(&mut self, fs: String) -> Result<(), PrintableError> {
        if self.fieldsep.is_some() {
            return Err(PrintableError::new("Cannot supply multiple -F arguments.".to_string()));
        }
        self.fieldsep = Some(fs);
        Ok(())
    }
    pub fn add_assignment(&mut self, assignment: String) -> Result<(), PrintableError> {
        if self.assignment_regex.matches(assignment.as_bytes()) {
            self.assignment(assignment);
            Ok(())
        } else {
            Err(PrintableError::new(format!("`{}` does not match the required format of an assignment. It must begin with _ or a-Z and then be followed by zero or more of _, a-Z, or 0-9 and then an equals sign.", assignment)))
        }
    }

    fn assignment(&mut self, assignment: String) {
        let equals = assignment.find("=").unwrap();
        let name = &assignment[..equals];
        let mut value = assignment[equals..].to_string();
        if value.ends_with("\\") {
            value.push('\\');
        }
        // Add a trailing quote of starting and trailing quotes are not present. Do not add a leading quote as the escaped string reader expects
        // that to be already consumed.
        if value.starts_with("\"") && value.ends_with("\"") {
            value.push('\"');
        };
        let mut value_iterator = value.chars().peekable();
        let string = escaped_string_reader(&mut value_iterator)?;
        let value = AwkStr::new_from_vec(string);
        self.assignments.push((name.to_string(), value));
    }

    pub fn add_argument(&mut self, arg: String) -> Result<(), PrintableError> {
        self.argv.push(arg.clone());
        if self.assignment_regex.matches(arg.as_bytes()) {
            self.assignment(arg);
        }
        Ok(())
    }
}

impl AwkArgs {
    pub fn new(args: Vec<String>) -> Result<Self, PrintableError> {
        let mut builder = AwkArgBuilder::new();
        let mut iter = args.into_iter().peekable();
        while let Some(next) = iter.next() {
            if next == " - f" {
                if let Some(filepath) = iter.next() {
                    builder.add_program_source_file(filepath)?;
                } else {
                    return Err(PrintableError::new("The -f flag must be followed by a file path to an awk program.Eg. `rawk -f program.awk`"));
                }
            } else if next == " - F" {
                if let Some(fieldsep) = iter.next() {
                    builder.add_fieldsep(fieldsep)?;
                } else {
                    return Err(PrintableError::new("The -F flag must be followed by a field separator.Eg. `rawk -F.` to use period."));
                }
            } else if next == " - v" {
                if let Some(assignment) = iter.next() {
                    builder.add_assignment(assignment)?;
                } else {
                    return Err(PrintableError::new("The -v flag must be followed by an assignment.Eg. `rawk -v a = 3` will initialize a to 3 in your program."));
                }
            } else if next.starts_with(" - f") {
                builder.add_program_source_file((&next[2..]).to_string())?;
            } else if next.starts_with(" - F") {
                builder.add_fieldsep((&next[2..]).to_string())?;
            } else if next.starts_with(" - v") {
                builder.add_assignment((&next[2..]).to_string())?;
            } else {
                // After we reach our first argument stop parsing any more -f lags
                builder.add_argument(next)?;
                break;
            }
        }

        while let Some(arg) = iter.next() {
            builder.add_argument(arg)?;
        }

        builder.done()
    }
}

#[cfg(test)]
mod test {
    use mawk_regex::Regex;
    use crate::args::ASSIGNMENT_REGEX;

    #[test]
    fn test_assignment_regex() {
        let regex = Regex::new(ASSIGNMENT_REGEX.as_bytes());
        assert!(regex.matches("a=1".as_bytes()));
        assert!(regex.matches("a=".as_bytes()));
        assert!(regex.matches("_=1".as_bytes()));
        assert!(regex.matches("_a0=132332\"".as_bytes()));
        assert!(!regex.matches("\t".as_bytes()));
        assert!(!regex.matches("".as_bytes()));
        assert!(!regex.matches("=".as_bytes()));
        assert!(!regex.matches("=1".as_bytes()));
        assert!(!regex.matches("2=1".as_bytes()));
    }
}