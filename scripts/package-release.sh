#!/usr/bin/env bash
#
# Turns raw `olympus` binaries into the release artifacts that scripts/install.sh
# and scripts/install.ps1 download. The GitHub Actions `publish` job runs this;
# you can also run it by hand to stage a release locally.
#
# Input binaries are picked up from --bin-dir by name:
#   olympus-linux-x86_64      olympus-macos-x86_64      olympus-windows-x86_64.exe
#   olympus-linux-arm64       olympus-macos-arm64       olympus-windows-arm64.exe
# Any subset is fine: whatever is present gets packaged and the rest is reported
# as missing at the end (pass --strict to refuse a partial set).
#
# Output layout, per the installer contract:
#   dist/olympus-<tag>-<os>-<arch>.tar.gz        os: linux | macos
#     olympus/olympus
#     olympus/data/workouts/*.zwo
#     olympus/LICENSE
#   dist/olympus-<tag>-windows-<arch>.zip
#     olympus.exe
#     data/workouts/*.zwo
#     LICENSE
#   dist/<artifact>.sha256                        "<hash>  <artifact>"
#
# The installers read the sidecar's first field, and `sha256sum -c` verifies the
# whole set from inside dist/.
#
# Usage:
#   scripts/package-release.sh --tag v1.1.0 [--bin-dir DIR] [--out-dir DIR]
#                              [--workouts-dir DIR] [--license FILE] [--strict]
#
# Environment:
#   SOURCE_DATE_EPOCH   unix timestamp used to normalise tar entry mtimes so a
#                       re-run over the same inputs yields a byte-identical
#                       .tar.gz. Unset -> host timestamps. The .zip files are not
#                       byte-reproducible either way, since zip records mtimes.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

say() { printf '%s\n' "$*"; }
die() { say "error: $*" >&2; exit 1; }

# --- Arguments ---------------------------------------------------------------
tag=""
bin_dir="$repo_root/bin"
out_dir="$repo_root/dist"
workouts_dir="$repo_root/data/workouts"
license_file="$repo_root/LICENSE"
strict=0

while [ $# -gt 0 ]; do
  case "$1" in
    --tag)          tag="${2:-}"; shift 2 ;;
    --bin-dir)      bin_dir="${2:-}"; shift 2 ;;
    --out-dir)      out_dir="${2:-}"; shift 2 ;;
    --workouts-dir) workouts_dir="${2:-}"; shift 2 ;;
    --license)      license_file="${2:-}"; shift 2 ;;
    --strict)       strict=1; shift ;;
    -h|--help)      sed -n '3,31p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *)              die "unknown argument: $1 (try --help)" ;;
  esac
done

[ -n "$tag" ] || die "--tag is required (e.g. --tag v1.1.0)"
case "$tag" in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *) die "tag must look like v1.2.3 (got '$tag')" ;;
esac

[ -d "$bin_dir" ]      || die "no --bin-dir at '$bin_dir'"
[ -d "$workouts_dir" ] || die "no --workouts-dir at '$workouts_dir'"

mkdir -p "$out_dir"

# --- Helpers -----------------------------------------------------------------
have() { command -v "$1" >/dev/null 2>&1; }

have_gnu_tar=0
tar --version 2>/dev/null | grep -q 'GNU tar' && have_gnu_tar=1

sha256() { # <file> -> hash on stdout
  if have sha256sum; then sha256sum "$1" | awk '{print $1}'
  elif have shasum; then shasum -a 256 "$1" | awk '{print $1}'
  else die "need sha256sum or shasum to write checksums"
  fi
}

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

# Deterministic on GNU tar when SOURCE_DATE_EPOCH is set. bsdtar (macOS) has no
# --sort/--mtime, so fall back to a plain archive there and drop AppleDouble
# metadata instead.
make_tar() { # <output>
  if [ "$have_gnu_tar" = 1 ] && [ -n "${SOURCE_DATE_EPOCH:-}" ]; then
    tar --sort=name --owner=0 --group=0 --numeric-owner \
        --mtime="@${SOURCE_DATE_EPOCH}" -czf "$1" -C "$stage" olympus
  else
    COPYFILE_DISABLE=1 tar -czf "$1" -C "$stage" olympus
  fi
}

# `zip` when available, else python3's zipfile with equivalent output: files only
# (no directory entries), forward-slash relative names, no extra attributes.
make_zip() { # <output>
  if have zip; then
    ( cd "$stage" && zip -X -9 -q -r "$1" . )
  elif have python3; then
    python3 - "$stage" "$1" <<'PY'
import os, sys, zipfile
stage, out = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for root, dirs, files in os.walk(stage):
        dirs.sort()
        for name in sorted(files):
            path = os.path.join(root, name)
            z.write(path, os.path.relpath(path, stage).replace(os.sep, "/"))
PY
  else
    die "need zip or python3 to build the Windows artifact (neither on PATH)"
  fi
}

# Archive contents with any leading ./ normalised away, so the checks below do
# not depend on how the archiver names entries.
list_archive() { # <archive>
  case "$1" in
    *.zip)
      if have unzip; then unzip -Z1 "$1" | sed 's|^\./||'
      elif have python3; then python3 -c 'import sys,zipfile; print("\n".join(zipfile.ZipFile(sys.argv[1]).namelist()))' "$1"
      else die "need unzip or python3 to inspect .zip artifacts"
      fi
      ;;
    *) tar -tzf "$1" | sed 's|^\./||' ;;
  esac
}

# --- Packaging ---------------------------------------------------------------
packaged=""
missing=""

for os in linux macos windows; do
  for arch in x86_64 arm64; do
    if [ "$os" = windows ]; then
      src="$bin_dir/olympus-$os-$arch.exe"
      artifact="olympus-$tag-$os-$arch.zip"
    else
      src="$bin_dir/olympus-$os-$arch"
      artifact="olympus-$tag-$os-$arch.tar.gz"
    fi

    if [ ! -f "$src" ]; then
      missing="$missing $os-$arch"
      continue
    fi
    [ -s "$src" ] || die "binary '$src' is empty"

    # Clear the staging area left by the previous target.
    rm -rf "${stage:?}/olympus" "${stage:?}/data" "${stage:?}/LICENSE"

    if [ "$os" = windows ]; then
      # Windows layout is flat: olympus.exe + data/workouts. install.ps1 searches
      # recursively, so the depth is cosmetic, but keep it tidy.
      cp "$src" "$stage/olympus.exe"
      mkdir -p "$stage/data/workouts"
      cp "$workouts_dir"/*.zwo "$stage/data/workouts/"
      if [ -f "$license_file" ]; then cp "$license_file" "$stage/LICENSE"; fi
      rm -f "$out_dir/$artifact"
      make_zip "$out_dir/$artifact"
    else
      # Unix layout nests everything under olympus/: install.sh seeds
      # $tmp/olympus/data/workouts and picks the binary by the name `olympus`.
      mkdir -p "$stage/olympus/data/workouts"
      cp "$src" "$stage/olympus/olympus"
      chmod 0755 "$stage/olympus/olympus"
      cp "$workouts_dir"/*.zwo "$stage/olympus/data/workouts/"
      if [ -f "$license_file" ]; then cp "$license_file" "$stage/olympus/LICENSE"; fi
      rm -f "$out_dir/$artifact"
      make_tar "$out_dir/$artifact"
    fi

    [ -f "$out_dir/$artifact" ] || die "failed to write $out_dir/$artifact"

    # Sidecar in sha256sum format: "<hash>  <artifact>". The installers read
    # field 1; `sha256sum -c` verifies the whole set from inside out_dir.
    printf '%s  %s\n' "$(sha256 "$out_dir/$artifact")" "$artifact" > "$out_dir/$artifact.sha256"

    say "packaged  $artifact  ($(du -h "$out_dir/$artifact" | cut -f1))"
    packaged="$packaged $os-$arch"
  done
done

[ -n "$packaged" ] || die "no binaries found in $bin_dir (expected olympus-<os>-<arch>[.exe])"

# --- Verify the artifacts the way an installer will --------------------------
say ""
say "verifying archive contents..."
for archive in "$out_dir"/olympus-"$tag"-*; do
  case "$archive" in *.sha256) continue ;; esac
  base="$(basename "$archive")"
  listing="$(list_archive "$archive")"

  if ! echo "$listing" | grep -qx 'olympus/olympus' &&
     ! echo "$listing" | grep -qx 'olympus.exe'; then
    die "$base does not contain the olympus binary at the expected path"
  fi
  echo "$listing" | grep -q '\.zwo$' || die "$base contains no workouts (.zwo)"

  # The sidecar must match a freshly computed hash.
  expected="$(awk '{print $1; exit}' "$archive.sha256")"
  actual="$(sha256 "$archive")"
  [ "$expected" = "$actual" ] || die "$base.sha256 mismatch ($expected vs $actual)"
  say "  ok  $base  $(printf '%.12s' "$expected")..."
done

# --- Summary -----------------------------------------------------------------
if [ -n "$missing" ]; then
  say ""
  say "missing binaries (not packaged):$missing"
  if [ "$strict" = 1 ]; then
    die "--strict: refusing to publish a partial release set"
  fi
fi

say ""
say "artifacts in $out_dir:"
ls -1 "$out_dir" | sed 's/^/  /'
