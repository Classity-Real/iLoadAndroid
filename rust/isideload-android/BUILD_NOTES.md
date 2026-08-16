# Build notes — isideload dependency quirks

Things learned the hard way while getting `isideload` to compile for
Android. Update this file whenever a similar surprise turns up.

## `idevice` is not optional in practice

`isideload`'s Cargo.toml lists `idevice` as an optional dependency, which
looks like it can be dropped with `default-features = false`. It can't —
`isideload`'s root `lib.rs` references `idevice::IdeviceError`
unconditionally, so disabling it breaks the build with `E0432`/`E0433`
on `idevice::*` paths.

**Fix:** add `idevice` explicitly, matching isideload's own documented
usage:

```toml
isideload = "0.3"
idevice = { version = "0.1.61", default-features = false, features = ["usbmuxd", "ring"] }
```

Restricting to the `usbmuxd` feature keeps it socket-protocol-only (no
libusb), which is what makes this safe to link on Android without
pulling in a raw-USB dependency. It's a compile-time requirement only —
nothing in the auth-only crate ever opens a device connection at
runtime.

## `tokio-tungstenite` is required for real Apple ID login

Same shape of problem: `remote_v3/websocket.rs` in isideload uses
`tokio_tungstenite::*` unconditionally. This is the anisette
provisioning websocket, which the actual login flow depends on — so
leave isideload on its default features rather than trying to strip
this one out too.

## `keyring` — unconfirmed, watch for this

Also listed as optional. We have NOT hit a build error from disabling
it yet, but we also haven't tried disabling it since the `idevice`
mistake above. If a future build fails with errors from
`security-framework`, `windows`, or similar desktop-keychain-backend
crates, it's the same pattern: `keyring` is more deeply wired into
isideload than the "optional" label suggests, and needs to stay on
isideload's default feature set rather than being turned off directly.

## General lesson

Don't infer isideload's real feature-gating from the `optional = true`
marker in its dependency list or from external "features" pages —
those describe what's fetchable, not what's actually reachable through
`#[cfg(feature = ...)]` in the source. When in doubt, pull the real
`Cargo.toml` and the relevant source files
(https://github.com/nab138/isideload) directly before guessing at a
minimal feature set.
