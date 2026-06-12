#!/bin/sh
# Build the gunmetal wasm bundle for the docs site (spec §5.3).
#
# cargo (workspace release profile: opt-level=z, lto, panic=abort)
#   → wasm-bindgen --target web (CLI version MUST match the workspace
#     lockfile's wasm-bindgen — currently 0.2.108)
#   → optional wasm-opt -Os when binaryen is installed
#   → emit into web/src/lib/wasm/ and enforce the 3 MB size budget.
set -e

# wasm-bindgen may live in a user-local cargo bin not on PATH.
PATH="$HOME/.cargo/bin:$PATH"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="$REPO_ROOT/web/src/lib/wasm"
BUDGET_BYTES=3145728

LOCKED_WB_VERSION=$(grep -A1 'name = "wasm-bindgen"' "$REPO_ROOT/Cargo.lock" | grep version | head -1 | sed 's/[^0-9.]*//g')
CLI_WB_VERSION=$(wasm-bindgen --version | sed 's/[^0-9.]*//g')
if [ "$LOCKED_WB_VERSION" != "$CLI_WB_VERSION" ]; then
  echo "ERROR: wasm-bindgen CLI ($CLI_WB_VERSION) != Cargo.lock ($LOCKED_WB_VERSION)" >&2
  echo "Install the matching CLI: cargo install wasm-bindgen-cli --version $LOCKED_WB_VERSION" >&2
  exit 1
fi

echo "building gunmetal for wasm32 (release)..."
cargo build --manifest-path "$REPO_ROOT/Cargo.toml" \
  --target wasm32-unknown-unknown -p gunmetal --features wasm --release

mkdir -p "$OUT_DIR"
wasm-bindgen --target web --out-dir "$OUT_DIR" --out-name gunmetal \
  "$REPO_ROOT/target/wasm32-unknown-unknown/release/gunmetal.wasm"

if command -v wasm-opt >/dev/null 2>&1; then
  echo "running wasm-opt -Os..."
  wasm-opt -Os -o "$OUT_DIR/gunmetal_bg.wasm" "$OUT_DIR/gunmetal_bg.wasm"
fi

SIZE=$(wc -c < "$OUT_DIR/gunmetal_bg.wasm" | tr -d ' ')
echo "gunmetal_bg.wasm: $SIZE bytes (budget $BUDGET_BYTES)"
if [ "$SIZE" -gt "$BUDGET_BYTES" ]; then
  echo "ERROR: wasm bundle exceeds the 3 MB budget" >&2
  exit 1
fi
echo "wasm bundle ready in web/src/lib/wasm/"

# Vendor the real GUN.js (submodule) for the gunjs-interop demo.
GUN_JS="$REPO_ROOT/crates/gunmetal/sources/gun/gun.js"
if [ -f "$GUN_JS" ]; then
  mkdir -p "$REPO_ROOT/web/static/vendor"
  cp "$GUN_JS" "$REPO_ROOT/web/static/vendor/gun.js"
  echo "vendored gun.js for the interop demo"
else
  echo "WARNING: gun submodule not checked out — gunjs-interop demo will degrade" >&2
fi
