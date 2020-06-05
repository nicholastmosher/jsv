use std::fs::File;
use std::io::{Read, BufReader};
use std::collections::HashMap;

use clap::{App, Arg, ArgMatches};
use serde::Deserialize;
use serde_json::Value;
use serde::export::Formatter;

fn main() {
    let matches = app().get_matches();
    match execute(&matches) {
        Ok(_) => return,
        Err(e) => println!("{}", e),
    }
}

/// Defines the structure of command-line parsing options
fn app<'a, 'b>() -> App<'a, 'b> {
    App::new("jsv")
        .arg(Arg::with_name("schema")
            .short("s")
            .long("--schema")
            .default_value("./schema.json"))
        .arg(Arg::with_name("csv-file")
            .index(1)
            .required(true))
}

/// Executes according to the given command-line arguments
fn execute(args: &ArgMatches) -> Result<(), String> {

    // The schema_path comes from the --schema command-line argument
    let schema_path = args.value_of("schema")
        // Use the unsafe "expect" here because the "--schema"
        // argument has a default value, so this is impossible to fail.
        .expect("schema argument is required");

    let csv_path = args.value_of("csv-file")
        // Use the unsafe "expect" here because the "input"
        // argument is marked as required, so this is impossible to fail.
        .expect("csv-file argument is required");

    // Try to open the schema file using the schema path
    let schema_file = File::open(schema_path)
        .map_err(|e| format!("failed to open schema file ({}): {:?}", schema_path, e))?;

    // Try to open the csv file using the csv path
    let csv_file = File::open(csv_path)
        .map_err(|e| format!("failed to open csv file ({}): {:?}", csv_path, e))?;

    // The schema file is itself JSON, so parse it into a JSON representation
    let json_schema: Value = serde_json::from_reader(schema_file)
        .map_err(|e| format!("failed to parse schema as JSON: {:?}", e))?;

    let schema = CsvSchema::from_json(json_schema)?;

    let validator = CsvValidator::new(schema);

    validator.validate(BufReader::new(csv_file));

    Ok(())
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize)]
enum CsvColumnType {
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "object")]
    Object,
}

impl std::fmt::Display for CsvColumnType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CsvColumnType::Integer => write!(f, "integer"),
            CsvColumnType::String => write!(f, "string"),
            CsvColumnType::Object => write!(f, "object"),
            CsvColumnType::Date => write!(f, "date"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CsvColumnProperty {
    #[serde(rename = "$id")]
    id: String,
    #[serde(rename = "type")]
    type_: CsvColumnType,
    title: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct CsvSchema {
    title: String,
    description: String,
    examples: Vec<Value>,
    required: Vec<String>,
    properties: HashMap<String, CsvColumnProperty>,
}

impl CsvSchema {
    pub fn from_json(json: Value) -> Result<CsvSchema, String> {
        // Deserialize CsvSchema from a JSON value
        let csv_schema: CsvSchema = serde_json::from_value(json)
            .map_err(|e| format!("failed to parse CsvSchema from JSON: {:?}", e))?;

        let errors: Vec<_> = csv_schema.properties.iter()
            .filter(|(_, props)| props.type_ == CsvColumnType::Object
                || props.type_ == CsvColumnType::Date)
            .collect();

        if !errors.is_empty() {
            use std::fmt::Write;
            let mut error = String::new();
            errors.iter().for_each(|(col, props)| {
                writeln!(error, "schema error: field definition for '{}' has illegal type {}. Only 'integer' and 'string' are supported.",
                         col,
                         props.type_);
            });
            return Err(error);
        }

        Ok(csv_schema)
    }
}

struct CsvValidator {
    schema: CsvSchema,
}

impl CsvValidator {
    pub fn new(schema: CsvSchema) -> CsvValidator {
        CsvValidator {
            schema
        }
    }

    pub fn validate<R: Read>(&self, input: R) {
        let mut csv_reader = csv::ReaderBuilder::new()
            .from_reader(input);

        let column_types: HashMap<usize, CsvColumnType> = match csv_reader.headers() {
            Err(e) => {
                eprintln!("failed to read headers from csv file: {:?}", e);
                return;
            },
            Ok(headers) => {
                if headers.len() != self.schema.properties.len() {
                    eprintln!("warning: there are {} columns but {} properties in the schema",
                              headers.len(),
                              self.schema.properties.len())
                }

                let column_types = headers.into_iter()
                    .enumerate()
                    .filter_map(|(i, field)| {
                        self.schema.properties.get(field)
                            .map(|typ| (i, typ.type_))
                    }).collect();

                println!("Column types: {:?}", column_types);
                column_types
            }
        };

        for result in csv_reader.records() {
            match result {
                Err(e) => eprintln!("error with record: {:?}", e),
                Ok(record) => {
                    record.iter()
                        .enumerate()
                        .for_each(|(i, field)| {
                            let maybe_type = column_types.get(&i);
                            print!("(field {}: {:?}) ", field, maybe_type);
                        });
                    println!("Record: {:?}", record);
                }
            }
        }
    }
}
