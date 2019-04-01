# Development Notes

CI uses "whatever the stable rustfmt is"
and you'll experience a lot of friction
if you drift from that.

## Releasing

[] Remove `-dev` from the version in `gotham/Cargo.toml`
[] Remove `-dev` from the version in `gotham_derive/Cargo.toml`
[] Update the `#![doc(html_root_url)]` to point to the new version.
[] Make a commit of the release with a message like `Releasing v0.x`
[] Tag the commit as `gotham_derive-0.x`
[] Tag the commit as `gotham-0.x`
[] Create a branch for `0.x-maint`
[] `mv Cargo.toml Cargo.toml.suspend` - the workspace has patches that interfere with releasing.
[] `pushd gotham_derive; cargo publish; popd`
[] `pushd gotham; cargo publish; popd`
[] Change the version in `gotham/Cargo.toml` to `0.<x+1>-dev`
[] Change the version in `gotham_derive/Cargo.toml` to `0.<x+1>-dev`
[] Commit with a message like `Starting v0.<x+1>-dev`
[] Push the commits and tags
[] Change to the `maint` branch and push that.
[] Start announcing the release.
