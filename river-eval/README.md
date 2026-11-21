# river-eval

The S-expression reader for River.

This crate intentionally contains no profile/device/presence types. Those are
written in `.river` and evaluated by `river-run` over the host WIT interfaces.
