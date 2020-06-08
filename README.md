# jsv

`jsv` is a tool for validating CSV data using rules written with [JSON Schema].

[JSON Schema]: https://json-schema.org/understanding-json-schema/index.html

# Usage

To use `jsv`, you'll need a schema file and a data file.

```
jsv
JSON-Schema Validator for CSV
USAGE:
    jsv [OPTIONS] <csv-file>
FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
OPTIONS:
    -s, --schema <schema>     [default: ./schema.json]
ARGS:
    <csv-file>
```

Suppose you have a `schema.json` file like this

```
{
    "$schema": "http://json-schema.org/draft-07/schema",
    "$id": "http://example.com/example.json",
    "type": "object",
    "properties": {
        "id": {
            "$id": "#/properties/id",
            "type": "integer",
            "title": "The user ID",
            "description": "A unique identifier for a user"
        },
        "name": {
            "$id": "#/properties/name",
            "type": "string",
            "title": "The user's name",
            "description": "The user's first and last name"
        }
    }
}
```

and you've got a data file `users.csv` like this

```
id,name
0,Adam Appleseed
1,Bobby Tables
Two,Chaotic Cassandra
```

When we use `jsv`, we should expect it to point out that the
`id` for the third record is not a valid integer! And indeed it does:

```
$ jsv ./users.csv --schema ./schema.json
Validation error on record 3:
Invalid type.
At instance path /id:
  "Two"

At schema path /properties/id/type:
  {
    "$id": "#/properties/id",
    "description": "A unique identifier for a user",
    "title": "The user ID",
    "type": "integer"
  }

Documentation for this node:
  A unique identifier for a user
  
Validation failed with 1 errors
```

Now that `jsv` pointed us to the dirty data, we can go ahead and
clean it up

```
id,name
0,Adam Appleseed
1,Bobby Tables
2,Chaotic Cassandra
```

And when we run `jsv`, we should see that the validation completes
successfully

```
$ jsv ./users.csv --schema ./schema.json
Successfully validated 3 records
```

# Installation

To use `jsv`, you'll need to build it from source using Rust. You
can install Rust over at [rustup.rs](https://rustup.rs/).

Once you install Rust, you should have access to the `cargo`
command on your command line. This is Rust's build tool, which we'll
use to compile `jsv`.

```
$ git clone https://github.com/nicholastmosher/jsv && cd jsv
$ cargo install --path .
```

This `cargo install` command will compile `jsv` in release mode
(high performance) and put it into your `~/.cargo/bin/` folder, which
should have been added to your PATH when you installed Rust. At this
point you should be able to run `jsv`.
