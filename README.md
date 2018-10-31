# mls-client

A prototype MLS client. See <https://github.com/wireapp/melissa> and
<https://github.com/wireapp/mls-server>.

## Running

Build the project: `cargo build`.

Run the project: `cargo run`. It will assume that a MLS server is running at
localhost:10100. After that you can use a simple language to perform
commands:

```
send("group_name", blob(index, "content"))

recv("group_name")

exit()
```

The language also supports variables and iteration. See
https://github.com/jonathandturner/rhai#rhai-language-guide for the details.
