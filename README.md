# rust-matrix-appservice-webhooks

This is a rewrite of https://github.com/turt2live/matrix-appservice-webhooks in Rust
using [rust-matrix-sdk](https://github.com/matrix-org/matrix-rust-sdk). It should be essentially
a drop-in replacement, albeit without some features. It has the ability to use the sqlite database and config file of the original
without any migration. The differences are documented below.

## Building

`cargo build`, or `cargo build --release`. May require a few dependencies (`cmake`, a C compiler and linker).
Will output the binary to `target/(debug|release)/rust-matrix-appservice-webhooks`.

## Usage

First, edit the config template (`config.yaml`) to match the specifics of your homeserver.
At the very least, the homeserver url, domain, and the webhook url base should have to change.

Then, run `rust-matrix-appservice-webhooks -f appservice.yaml --config config.yaml -r --url <THE URL YOU WILL BE RUNNING THE APPSERVICE ON>`
to generate the `appservice.yaml` file needed to register the appservice with your homeserver.

Configure your homeserver to use the generated file. For Dendrite at least, this involves adding the file path
to `app_service_api.config_files` in the config file.

Finally, start the appservice by running `rust-matrix-appservice-webhooks -f appservice.yaml --config config.yaml --port <PORT> --database-path webhooks.db`
where `<PORT>` matches the url in `appservice.yaml`.

## Similarities

- Uses a subset of the database schema of the node version. Should be able to use
    database files created by the node version without issue.
- Supports almost the exact same command-line flags, with one addition.
- Supports the same config file format, such that no changes are needed to use config files
    for the old version.
- Supports a subset of the same webhook syntax, including all `message_type`s, emoji in message bodies, and some other fields.
    If the output differs meaningfully from the node version in any way, or if there is a feature I haven't added, please feel free
    to create an issue.

## Differences

- Requires a flag (`-d`/`--database-path`) to describe the location of the sqlite database, instead of
    the implicit default of the node version.
    Bonus: for testing, you can set this to `sqlite::memory:` to use a temporary in-memory data store.
- Ignores the `logging:` section of the config file. `stdout` or bust! You can set the logging level using
    `RUST_LOG`.
- No provisioning API, and so it ignores the `provisioning:` section of the config file.
- At least some of the webhook syntax is missing (like attachments), or produces different output.
- Probably other features, and bugs.

## Improvements

- Uses a much more modern SDK, which is likely to get native E2EE support at some point.
- Supports Dendrite (the nodejs version fails due to limits on the charset of webhook userids)
- Properly follows redirects when downloading avatar URLs. The nodejs version fails in this case.
- Persistent webhook userids, instead of adding a new webhook user every time the display name changes.
- Easier deployment, since it's a single binary.

## TODOs

- Testing with Synapse. I only run Dendrite myself, so I don't know if it works with Synapse yet.
- Add a Dockerfile, and maybe publish the image to Dockerhub for ease of use.
- Possibly reach feature parity with the original.

## End-to-end encryption

Not supported, since the SDK doesn't have support yet. See https://github.com/matrix-org/matrix-rust-sdk/issues/228.