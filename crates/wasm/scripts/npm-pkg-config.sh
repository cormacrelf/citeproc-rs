#!/usr/bin/env bash

set -euo pipefail
shopt -s nullglob
shopt -s extglob
shopt -s globstar

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
DIR="$( cd $DIR/.. && pwd )"

CLEAR='\033[0m'
RED='\033[0;31m'

function usage() {
  if [ -n "$1" ]; then
    echo -e "${RED}👉 $1${CLEAR}\n";
  fi
  echo "Usage: $0"
  echo "  --dest DEST                            Write output to this folder (rm -rf/deletes it first!)"
  echo "                                         The default is './dist'"
  echo "  --canary-sha SHA_COMMIT                Set version to '0.0.0-canary-\$SHA_COMMIT'"
  echo "  --set-version VERSION                  Use this version specifically"
  echo "  --cargo-version                        Use the version from Cargo.toml"
  echo "  --set-name NAME                        Use this package name"
  echo "  --package-only                         Don't rebuild, just rewrite the package.json"
  echo "  --github-packages @scope/repo/pkg-name Configure for publishing to GitHub packages, @scope/pkg-name in repo @scope/repo"
  echo "  --features                             List of cargo features to enable (comma-sep)"
  echo "  --targets                              List of npm targets to build (comma-sep, default all)"
  echo "  --dev                                  Build in --dev mode"
  echo
  exit 1
}

DEST=./dist
SET_VERSION=
USE_CARGO_VERSION=
SET_NAME=
PACKAGE_ONLY=
GITHUB_PACKAGES_DEF=
CANARY_SHA=
FEATURES=
TARGETS=
DEV_OR_RELEASE=--release

# parse params
while [[ "$#" > 0 ]]; do case $1 in
  --dest) DEST="$2"; shift;shift;;
  --set-name) SET_NAME="$2";shift;shift;;
  --canary-sha) CANARY_SHA="$2";shift;shift;;
  --set-version) SET_VERSION="$2";shift;shift;;
  --cargo-version) USE_CARGO_VERSION=true;shift;;
  --package-only) PACKAGE_ONLY=true;shift;;
  --github-packages) GITHUB_PACKAGES_DEF="$2";shift;shift;;
  --features) FEATURES="$2";shift;shift;;
  --targets) TARGETS="$2";shift;shift;;
  --dev) DEV_OR_RELEASE="--dev";shift;;
  *) usage "Unknown parameter passed: $1"; shift; shift;;
esac; done

# make sure it doesn't have a v prefix
SET_VERSION=${SET_VERSION#v}

# default destination location is called 'dist' i.e. distribution
CARGO_VERSION=$(cargo metadata --format-version 1 --no-deps --manifest-path $DIR/Cargo.toml | jq -r '.packages  | .[] | select(.name=="wasm") | .version')

bail() {
  MESSAGE="$@"
  echo -e "${RED}failed ${MESSAGE}${CLEAR}"
  exit 1
}

# What's going on here?
#
# It's a replacement while we wait on https://github.com/rustwasm/wasm-pack/pull/705
#
# Webpack knows what its target is. If you are using it (or similar), which
# entry point in package.json to select will be determined by this target.
# There are fallbacks, so it tries "browser", "module", and then "main".
# Node.js, on the other hand, when executing require('...'), simply looks for
# "main" and loads the file it points to.
#
# For us:
# - "main" points to _cjs, which uses Node.js-specific features to load a raw
#   binary file, and load it as WASM. The JS modules are also in CommonJS
#   (`module.exports = {}`) format.
# - "browser" points to _esm, which uses window and other browser-based
#   things to load wasm. The JS modules are in ES Modules (`export { blah }`)
#   format, which WebPack et al can understand.
#
# Each of the methods/formats is written out for us by wasm-pack. What about
# the "web" (i.e. "can be loaded by a browser with a script tag") format? That
# one is included in the `_web` folder, but it does not need an entry in package.json!
# 
# The PR linked above uses different folder names, but these ones accurately
# describe why we need different builds in different environments.

TARGETS_ALL="$TARGETS"
if [ -z "$TARGETS_ALL" ]; then
  TARGETS_ALL="nodejs,browser,web,no-modules,zotero"
fi

if [ -z "$PACKAGE_ONLY" ]; then
  mkdir -p $DIR/pkg-scratch
  rm -rf $DEST
  mkdir -p $DEST

  ## e.g. build into pkg-scratch/target; move files to $DEST/_target/
  target() {
    TARGET=${1:-}
    OUT=${2:-}
    EXTRA_FEATURES=${3:-}
    SCRATCH="$DIR/pkg-scratch/$OUT"
    wasm-pack build $DEV_OR_RELEASE \
      --out-name citeproc_rs_wasm \
      --scope citeproc-rs \
      --target "$TARGET" \
      --out-dir "$SCRATCH" \
      -- --features "$FEATURES$EXTRA_FEATURES" \
      || bail "building target $TARGET -> $OUT --features \"$FEATURES$EXTRA_FEATURES\""

    SNIPPETS="$DIR/pkg-scratch/$OUT/snippets"
    mkdir -p "$DEST/$OUT" \
      && ([[ -d "$SNIPPETS" ]] && cp -R "$SNIPPETS" "$DEST/$OUT" || true) \
      && cp -R  $DIR/pkg-scratch/$OUT/citeproc_rs_wasm* "$DEST/$OUT/" \
      && cp $DIR/pkg-scratch/$OUT/.gitignore "$DEST/" \
      || bail "building target $TARGET -> $OUT --features \"$FEATURES$EXTRA_FEATURES\""
  }

  echo "$TARGETS_ALL" | sed -n 1'p' | tr ',' '\n' | while read TARGET; do
      echo TARGET: $TARGET
      if [ "$TARGET" == "nodejs" ]; then target nodejs _cjs; fi
      if [ "$TARGET" == "browser" ]; then target browser _esm; fi
      if [ "$TARGET" == "web" ]; then target web _web; fi
      if [ "$TARGET" == "no-modules" ]; then target no-modules _no_modules ,no-modules; fi
      if [ "$TARGET" == "zotero" ]; then target no-modules _zotero ,zotero; fi
      echo TARGET DONE: $TARGET
  done

else
  mkdir -p $DEST/_cjs $DEST/_esm $DEST/_web $DEST/_no_modules $DEST/_zotero
fi

if [[ $TARGETS_ALL =~ "zotero" ]]; then
  ZOTERO_BINDGEN_SRC="$DIR/pkg-scratch/_zotero/citeproc_rs_wasm.js"
  ZOTERO_BINDGEN="$DEST/_zotero/citeproc_rs_wasm.js"
  sed -e 's/CITEPROC_RS_ZOTERO_GLOBAL/Zotero.CiteprocRs/g' \
    < $ZOTERO_BINDGEN_SRC \
    > "$ZOTERO_BINDGEN" \
    || bail "could not replace CITEPROC_RS_ZOTERO_GLOBAL"
  cat <<EOF >> "$ZOTERO_BINDGEN"
module.exports = wasm_bindgen;
if (typeof Zotero !== "undefined" && typeof Zotero.CiteprocRs !== "undefined") {
  Object.assign(Zotero.CiteprocRs, wasm_bindgen)
}
EOF
fi

NODEJS_INCLUDE=/dev/null
NOMOD_INCLUDE=/dev/null
ZOTERO_INCLUDE=/dev/null
if [[ "$TARGETS_ALL" == *"nodejs"* ]]; then NODEJS_INCLUDE=($DEST/_cjs/snippets/**/include.js); fi
if [[ "$TARGETS_ALL" == *"no-modules"* ]]; then NOMOD_INCLUDE="$DEST/_no_modules/citeproc_rs_wasm_include.js"; fi
if [[ "$TARGETS_ALL" == *"zotero"* ]]; then ZOTERO_INCLUDE="$DEST/_zotero/citeproc_rs_wasm_include.js"; fi

sed -e 's/export class/class/' < $DIR/src/js/include.js \
  | tee "$ZOTERO_INCLUDE" \
  | tee "$NODEJS_INCLUDE" \
  > "$NOMOD_INCLUDE" \
  || bail "could not write include.js for no-modules targets"
# We want the commonjs module.exports in both of these
cat $DIR/src/js/commonjs_export.js >> "$NOMOD_INCLUDE" || bail "failed writing $NOMOD_INCLUDE"
cat $DIR/src/js/commonjs_export.js >> "$NODEJS_INCLUDE" || bail "failed writing $NODEJS_INCLUDE"
# zotero is weird
cat $DIR/src/js/zotero.js >> "$ZOTERO_INCLUDE" || bail "failed writing $ZOTERO_INCLUDE"

cp $DIR/README.md $DEST/ || bail "could not copy README"
# cp $DIR/scripts/model-package.json $DEST/package.json

TARGET_VERSION=${SET_VERSION:-$CARGO_VERSION}

JQ_ARGS=""
JQ_FILTERS="  ."
function append() {
  if [ -n "$1" ]; then JQ_ARGS=$(printf "$JQ_ARGS \n  $1"); fi
  if [ -n "$2" ]; then JQ_FILTERS=$(printf "$JQ_FILTERS \n  | $2"); fi
}

if [ -n "$CANARY_SHA" ]; then
  append "--arg version v0.0.0-canary-$CANARY_SHA" '.version = $version'
elif [ -n "$SET_VERSION" ]; then
  append "--arg version $SET_VERSION" '.version = $version'
elif [ -n "$USE_CARGO_VERSION" ]; then
  append "--arg version $CARGO_VERSION" '.version = $version'
fi

if [ -n "$GITHUB_PACKAGES_DEF" ]; then
  NO_AT=${GITHUB_PACKAGES_DEF#@}
  PKG=$(basename $NO_AT)
  REPO=${NO_AT%/$PKG}
  REPO=${REPO#@}
  SCO=${NO_AT%%/*/$PKG}
  NAME="@$SCO/$PKG"

  append '' '.publishConfig = { registry: "https://npm.pkg.github.com/cormacrelf" }'

  append "--arg url ssh://git@github.com/$REPO.git --arg directory packages/$PKG" \
         '.repository = { type: "git", url: $url, directory: $directory }'

  append "--arg name $NAME" '.name = $name'

elif [ -n "$SET_NAME" ]; then
  append "--arg name $SET_NAME" '.name = $name'
fi


printf "Writing package.json with: $JQ_ARGS\n$JQ_FILTERS\n"

jq $JQ_ARGS "$JQ_FILTERS" < $DIR/scripts/model-package.json > $DEST/package.json \
  || bail "writing $DEST/package.json using scripts/model-package.json and jq"

# cat $DEST/package.json | jq

