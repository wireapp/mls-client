# mls-client

A prototype MLS client. See <https://github.com/wireapp/melissa> and
<https://github.com/wireapp/mls-server>.

## Running

Build the project: `cargo build`.

Run the project: `cargo run`. It will assume that a MLS server is running at
localhost:10100. After that you can use a simple Rust-like language to
perform commands.

The language also supports variables and iteration. See
https://github.com/jonathandturner/rhai#rhai-language-guide for the details.

## Commands

See `src/repl.rs` for the list of commands.

## Sample scenario

The following commands should be run from two terminals.

    $ cargo run                         $ cargo run

    Created new user 'foo'              Created new user 'bar'

Export users' public keys into the current directory:

    > export()                          > export()
    Wrote foo.pub and foo.init          Wrote bar.pub and bar.init

Create a group and add a user (user's key will be read from the current
directory, and the invitation will also be written into the current
directory):

    > create("travel")

    > add("travel", "bar")
    Wrote travel_bar.welcome

Accept the invitation and do an update:

                                        > join("travel")

                                        > update()

The first user does an update as well:

    > update()

The second user treacherously removes the first user:

                                        > remove("travel", "foo")
