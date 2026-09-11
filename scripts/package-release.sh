#!/usr/bin/env bash
set -euo pipefail

root_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root_dir/Cargo.toml" | head -1)
target_dir="$root_dir/target/release-package"
package_dir="$target_dir/omarchy-batch-monitor-$version"

cargo build --release --locked --manifest-path "$root_dir/Cargo.toml"
rm -rf -- "$target_dir"
install -Dm755 "$root_dir/target/release/obm" "$package_dir/bin/obm"
install -Dm644 "$root_dir/assets/org.omarchy.obm.desktop" "$package_dir/share/applications/org.omarchy.obm.desktop"
install -Dm644 "$root_dir/assets/org.omarchy.obm.svg" "$package_dir/share/icons/hicolor/scalable/apps/org.omarchy.obm.svg"
install -Dm644 "$root_dir/man/obm.1" "$package_dir/share/man/man1/obm.1"
install -Dm644 "$root_dir/LICENSE" "$package_dir/share/licenses/omarchy-batch-monitor/LICENSE"
install -Dm644 "$root_dir/README.md" "$package_dir/share/doc/omarchy-batch-monitor/README.md"
tar -C "$target_dir" -czf "$target_dir/omarchy-batch-monitor-$version-x86_64-linux-gnu.tar.gz" "omarchy-batch-monitor-$version"
sha256sum "$target_dir/omarchy-batch-monitor-$version-x86_64-linux-gnu.tar.gz" > "$target_dir/omarchy-batch-monitor-$version-x86_64-linux-gnu.tar.gz.sha256"
echo "Created release assets in $target_dir"
