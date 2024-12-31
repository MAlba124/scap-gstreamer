# Scap GStreamer

This is a [GStreamer](https://gstreamer.freedesktop.org/) plugin for [scap](https://github.com/CapSoftware/scap). It provides a scapsrc element, which is a video source that captures screencasts.

## Building the plugin

To build the plugin, ensure you have [Rust](https://www.rust-lang.org/) and [Cargo](https://doc.rust-lang.org/cargo/) installed.

Build the plugin in debug mode:

```console
$ cargo build
```

or release mode:

```console
$ cargo build --release
```

## Configuring GStreamer

Tell GStreamer where the plugin can be found (change `/target/debug` to `/target/release` if you built in release mode):

```console
$ export GST_PLUGIN_PATH=`pwd`/target/debug
```

## Running the plugin

You can easily create a pipeline from the command line with the following command:

```console
$ GST_DEBUG=scapsrc:5 gst-launch-1.0 scapsrc ! videoconvert ! autovideosink
```

Discard `GST_DEBUG=scapsrc:5` if debug logs are not needed.

Examples showing how to use the plugin programatically will come soon.

## License

MIT license ([LICENSE-MIT](./LICENSE-MIT)) or Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE))
