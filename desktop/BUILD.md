# Build steps

Make sure to start at the root of the app folder.

## macOS preliminary

### Certificates

When building from macOS, it is necessary to have the certificates (these can be found in the drive
currently under `dev/certificates`).

For this you need the following two files:

- `Developer ID Application: Brian Carlsen (63BA6GV3UX)`
- `Developer ID Certification Authority`

Simply drag and drop these files to the `Keychain Access` app in the `login` tab.

### XCode

Make sure XCode is installed, with it comes `altool`, you need to have this in your path!

## Build database

- `cd local/database`

### On macOS and Linux

- `./build.sh`

> If building from m1 to intel: `./build_x86_64.sh`
> If building from intel to m1: `./build_aarch64.sh`

### On Windows

- `./build.bat`

## Add secrets to ENV

If you haven't already, go back to the root path `cd ../../`.

### In bash / zsh

- `set -o allexport && source .github/act/secrets && set +o allexport`

## Build application

- `cd desktop`
- `cargo tauri build`

> If an error occurs, run with the `--verbose` flag.
> If building from m1 to intel: `cargo tauri build --target x86_64-apple-darwin`
> If building from intel to m1: `cargo tauri build --target aarch64-apple-darwin`

## FAQ

### Apple

#### Error running bundle_dmg.sh
Make sure no `syre*` process is running and that Syre is not mounted as a volume.

#### Error failed to bundle project: failed to upload app to Apple's notarization servers.
Check the [apple developer](https://developer.apple.com/) account that no terms of service are pending approval.

### Windows

#### `zmq` build errors
`cargo build` the `src-tauri` crate first.