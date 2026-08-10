# Examples for rust-oracledb

Set the following environment variables to provide credentials for the examples.

- `RSO_EXAMPLES_MAIN_USER`: provides the name of the schema user which will be
  used for running the samples. The user should be capable of creating tables
  since some of the samples require this capability.

- `RSO_EXAMPLES_MAIN_PASSWORD` provides the password of the schema user which
  will be used for running the samples.

- `RSO_EXAMPLES_CONNECT_STRING` provides the connection string that points to
  the database that wil lbe used for running the samples.

To run the examples (with the release/production build),

```
cargo run --example <file_name_without_rs_extension> --release
```

To run the examples that use the Apache Arrow framework, use

```
cargo run --example <file_name_without_rs_extension> --features arrow --release
```

To run in `debug` build, remove the `--release` switch.
