### `iced_wry` a simple wrapper for embedding wry webviews in your iced application.

`iced` used the MVU architecture to render it's UI, while `wry` is a retained mode API. This crate provides a bridge for the two, with some compromises. The main ones being the webview's visibility may lag behind the view state and keyboard focus in need of further work.

> Check out `examples/simple.rs` for a reference.