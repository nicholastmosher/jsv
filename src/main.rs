use std::fs::File;
use std::io::{Read, BufReader};
use std::collections::HashMap;

use clap::{App, Arg, ArgMatches};
use serde_json::Value;
use jsonschema_valid::Config as SchemaConfig;

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
        .about("JSON-Schema Validator for CSV")
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

    // Produce a SchemaConfig from the JSON schema object.
    let schema_config = SchemaConfig::from_schema(&json_schema, None).unwrap();

    // Use the SchemaConfig for validating values in CSV.
    let validator = CsvValidator::new(schema_config);

    let result = validator.validate(BufReader::new(csv_file));

    match result {
        Ok(num) => println!("Successfully validated {} records", num),
        Err(num) => println!("Validation failed with {} errors", num),
    }

    Ok(())
}

/// Validates CSV fields using the rules from a JSON-Schema validator.
///
/// It is important to note that the given JSON Schema must consist of a single
/// top-level object definition. The keys of this object must correspond to the
/// header names of the CSV data given. Data values in the CSV are parsed and
/// validated as if they were primitive JSON values (i.e. any JSON values
/// except for objects and arrays).
///
/// # Example
///
/// ```
/// use jsonschema_valid::Config as SchemaConfig;
///
/// fn main() {
///     let schema = r#"{
///         "$id": "http://example.com/example.json",
///         "type": "object",
///         "properties": {
///             "id": {
///                 "$id": "#/properties/id",
///                 "type": "integer"
///             }
///             "name": {
///                 "$id": "#/properties/name",
///                 "type": "string"
///             }
///         }
///     }"#;
///
///     let schema_json = serde_json::from_str(schema).unwrap();
///     let schema_config = SchemaConfig::from_schema(&schema_json, None).unwrap();
///     let validator = CsvValidator::new(schema_config);
///
///     let csv = r#"
///     id,name
///     0,Bobby
///     "#;
///
///     validator.validate(Cursor::new(csv));
/// }
/// ```
struct CsvValidator<'a> {
    schema_config: SchemaConfig<'a>,
}

impl CsvValidator<'_> {
    pub fn new(schema_config: SchemaConfig) -> CsvValidator {
        CsvValidator {
            schema_config
        }
    }

    pub fn validate<R: Read>(&self, input: R) -> Result<usize, usize> {
        let mut csv_reader = csv::ReaderBuilder::new()
            .from_reader(input);

        let headers = csv_reader.headers().unwrap().clone();

        let mut success_count: usize = 0;
        let mut error_count: usize = 0;
        for (record_index, result) in csv_reader.records().enumerate() {
            let record = match result {
                Err(e) => {
                    eprintln!("Record error at index {}: {:?}", record_index, e);
                    continue;
                },
                Ok(record) => record,
            };

            let schema = self.schema_config.get_schema().as_object().unwrap();

            let mut record_map: HashMap<&str, Value> = HashMap::new();
            for (field_index, (header, field)) in headers.iter().zip(record.iter()).enumerate() {

                // Manually check whether this field has a "string" type in the schema.
                // If we don't do this, then even though the schema says to treat it
                // like a string, the JSON parser would read a field like 1234 as a number.
                let is_string = schema.get("properties")
                    .and_then(|val| val.as_object())
                    .and_then(|obj| obj.get(header))
                    .and_then(|val| val.as_object())
                    .and_then(|obj| obj.get("type"))
                    .and_then(|val| val.as_str())
                    .map(|typ| typ == "string")
                    .unwrap_or(false);

                // If the schema marks this field as a string, parse it as a string.
                let maybe_field_value = if is_string {
                    serde_json::from_str(&format!("\"{}\"", field))
                }
                // Otherwise, parse it like normal JSON
                else {
                    serde_json::from_str(field)
                        .or_else(|_| serde_json::from_str(&format!("\"{}\"", field)))
                };

                let field_value: Value = match maybe_field_value {
                    Err(e) => {
                        eprintln!("Field error at ({}:{}) for field ({}): {:?}", record_index, field_index, field, e);
                        continue;
                    },
                    Ok(value) => value,
                };

                record_map.insert(header, field_value);
            }
            let record_value: Value = serde_json::to_value(record_map).unwrap();

            let result = self.schema_config.validate(&record_value);
            match result {
                Ok(_) => {
                    success_count += 1;
                },
                Err(e) => {
                    error_count += 1;
                    eprintln!("Validation error on record {}:", record_index + 1);
                    for error in e {
                        eprintln!("{}", error);
                    }
                }
            }
        }

        if error_count == 0 { Ok(success_count) }
        else { Err(error_count) }
    }
}
