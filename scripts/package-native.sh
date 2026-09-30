#!/usr/bin/env bash
#
# Builds the native Linux packages (Debian/Ubuntu, Arch, Fedora/RHEL) from the
# release binaries produced by the `build` matrix. The GitHub Actions `publish`
# job runs this right after scripts/package-release.sh, so every release ships
# both the installer-wizard archives and the native packages.
#
# Input binaries are picked up from --bin-dir by name, exactly as
# scripts/package-release.sh does:
#   olympus-linux-x86_64      olympus-linux-arm64
# Any subset is fine: whatever is present gets packaged and the rest is reported
# as missing at the end (pass --strict to refuse a partial set).
#
# Output, for tag v1.1.0 and version 1.1.0:
#   dist/olympus_1.1.0_amd64.deb              deb     (arch: amd64 / arm64)
#   dist/olympus-1.1.0.x86_64.rpm              rpm     (arch: x86_64 / aarch64)
#   dist/olympus-1.1.0-x86_64.pkg.tar.zst      arch    (arch: x86_64 / aarch64)
#   dist/<package>.sha256                      "<hash>  <package>"
#
# Payload, identical in all six packages:
#   /usr/bin/olympus                      launcher, resolves the data home
#   /usr/bin/olympus-core                 the release binary
#   /usr/share/olympus/workouts/*.zwo     bundled workout library
#   /usr/share/olympus/LICENSE
#
# Why this script exists instead of a plain `nfpm package` call:
#
#   * nfpm does not translate architecture names between packagers, and
#     `overrides.<packager>.arch` is not a valid key, so OLYMPUS_ARCH is set per
#     invocation here.
#   * nfpm resolves `contents[].src` against the working directory and does not
#     expand environment variables in it, so each nfpm call runs from inside its
#     own staging directory.
#   * nfpm applies `file_info.mode` to directories as well as files, and rejects
#     a `type: dir` that duplicates a tree entry, so permissions are set on the
#     staged files with chmod and no file_info is used. 0755 for directories and
#     executables, 0644 for data -- see packaging/nfpm.yaml.
#
# Alpine is deliberately not a target: the binary is glibc-linked (btleplug via
# libdbus, plus the bundled SQLite), so an apk would not run on musl.
#
# Usage:
#   scripts/package-native.sh --tag v1.1.0 [--bin-dir DIR] [--out-dir DIR]
#                             [--workouts-dir DIR] [--license FILE]
#                             [--launcher FILE] [--config FILE] [--nfpm PATH]
#                             [--strict]
#
# Environment:
#   SOURCE_DATE_EPOCH   unix timestamp applied to the staged file mtimes. Partial
#                       only: nfpm still stamps a build time into the package
#                       metadata itself, so unlike the .tar.gz archives these
#                       packages are not byte-reproducible.

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
launcher_file="$repo_root/packaging/launcher/olympus"
config_file="$repo_root/packaging/nfpm.yaml"
nfpm_bin="nfpm"
strict=0

while [ $# -gt 0 ]; do
  case "$1" in
    --tag)          tag="${2:-}"; shift 2 ;;
    --bin-dir)      bin_dir="${2:-}"; shift 2 ;;
    --out-dir)      out_dir="${2:-}"; shift 2 ;;
    --workouts-dir) workouts_dir="${2:-}"; shift 2 ;;
    --license)      license_file="${2:-}"; shift 2 ;;
    --launcher)     launcher_file="${2:-}"; shift 2 ;;
    --config)       config_file="${2:-}"; shift 2 ;;
    --nfpm)         nfpm_bin="${2:-}"; shift 2 ;;
    --strict)       strict=1; shift ;;
    -h|--help)      sed -n '3,60p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *)              die "unknown argument: $1 (try --help)" ;;
  esac
done

[ -n "$tag" ] || die "--tag is required (e.g. --tag v1.1.0)"
case "$tag" in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *) die "tag must look like v1.2.3 (got '$tag')" ;;
esac

# Package managers want a bare version; the leading v is a git tag convention.
version="${tag#v}"

[ -d "$bin_dir" ]      || die "no --bin-dir at '$bin_dir'"
[ -d "$workouts_dir" ] || die "no --workouts-dir at '$workouts_dir'"
[ -f "$launcher_file" ] || die "no launcher at '$launcher_file' (see packaging/launcher/olympus)"
[ -f "$config_file" ]  || die "no nfpm config at '$config_file' (see packaging/nfpm.yaml)"
command -v "$nfpm_bin" >/dev/null 2>&1 || die "nfpm not found on PATH (pass --nfpm PATH); see packaging/nfpm.yaml"

mkdir -p "$out_dir"
# nfpm runs with the working directory set to the staging area, so both the
# config path and the output path have to be absolute.
out_dir="$(cd "$out_dir" && pwd)"
config_file="$(cd "$(dirname "$config_file")" && pwd)/$(basename "$config_file")"

# --- Helpers -----------------------------------------------------------------
have() { command -v "$1" >/dev/null 2>&1; }

sha256() { # <file> -> hash on stdout
  if have sha256sum; then sha256sum "$1" | awk '{print $1}'
  elif have shasum; then shasum -a 256 "$1" | awk '{print $1}'
  else die "need sha256sum or shasum to write checksums"
  fi
}

# Members inside a built package as "<permissions> <path>", one per line, or exit
# 1 when this host has no tool able to read that format. Permissions are kept
# because they are half the contract here: the launcher and the binary have to
# arrive executable, and the workouts readable but not executable.
list_package() { # <file>
  case "$1" in
    *.deb)
      have dpkg-deb || return 1
      # drwxr-xr-x root/root  0 2026-09-30 10:00 ./usr/bin/
      # Strip the leading ./ in awk, anchored, rather than with an unanchored
      # sed that would also rewrite a './' appearing inside a filename.
      dpkg-deb -c "$1" | awk '{ p = $NF; sub(/^\.\//, "", p); print $1, p }'
      ;;
    *.rpm)
      have rpm || return 1
      # -rwxr-xr-x  1 root root 1234 Jan  1 1970 /usr/bin/olympus-core
      rpm -qlvp "$1" 2>/dev/null | awk '{print $1, $NF}'
      ;;
    *.pkg.tar.zst)
      have zstd && have tar || return 1
      # -rwxr-xr-x root/root 1234 1970-01-01 00:00 usr/bin/olympus-core
      zstd -dc "$1" | tar -tvf - | awk '{print $1, $NF}'
      ;;
    *) return 1 ;;
  esac
}

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

# --- Package assertions ------------------------------------------------------
# `artifact` is set by the verification loop and read by these, so a failure
# names the package that is wrong rather than just the invariant.
artifact=""
listing=""

check_member() { # <path>
  awk -v want="$1" '$2 == want { found = 1 } END { exit !found }' "$listing" \
    || die "$artifact does not contain /$1"
}

# Asserts the owner-execute bit. nfpm preserves the staged modes, so this is the
# check that proves the chmod pass in the staging step actually landed in the
# package -- a launcher that is not executable fails here instead of at runtime.
check_executable() { # <path> <yes|no>
  local perm want="$1"
  perm="$(awk -v want="$want" '$2 == want { print $1; exit }' "$listing")"
  # A permission field is <type><owner rwx><group rwx><other rwx>, so index 3 is
  # the owner-execute bit for every entry type ('-' files, 'd' directories, ...).
  local is_exec=no
  [ "${perm:3:1}" = "x" ] && is_exec=yes
  [ "$is_exec" = "$2" ] \
    || die "$artifact: /$want has mode '$perm', expected owner-executable=$2"
}

# --- Packaging ---------------------------------------------------------------
packaged=""
missing=""

for arch in x86_64 arm64; do
  src="$bin_dir/olympus-linux-$arch"
  if [ ! -f "$src" ]; then
    missing="$missing linux-$arch"
    continue
  fi
  [ -s "$src" ] || die "binary '$src' is empty"

  # Per-architecture staging root. nfpm is invoked from inside it so that the
  # fixed relative paths in packaging/nfpm.yaml (payload/bin, payload/share/...)
  # resolve to this architecture's tree.
  work="$stage/$arch"
  rm -rf "${work:?}"
  payload="$work/payload"
  mkdir -p "$payload/bin" "$payload/share/olympus/workouts"

  cp "$src" "$payload/bin/olympus-core"
  cp "$launcher_file" "$payload/bin/olympus"
  cp "$workouts_dir"/*.zwo "$payload/share/olympus/workouts/"
  if [ -f "$license_file" ]; then cp "$license_file" "$payload/share/olympus/LICENSE"; fi

  # Modes are the contract here: nfpm preserves the staged modes, and 0644 on a
  # directory would make the workouts unreachable at runtime. Set directories and
  # executables to 0755 and data to 0644, top-down.
  find "$payload" -type d -exec chmod 0755 {} +
  chmod 0755 "$payload/bin/olympus" "$payload/bin/olympus-core"
  find "$payload/share" -type f -exec chmod 0644 {} +

  if [ -n "${SOURCE_DATE_EPOCH:-}" ]; then
    find "$payload" -type f -exec touch -d "@${SOURCE_DATE_EPOCH}" {} +
  fi

  # Architecture and version differ per packager, so each nfpm invocation gets
  # its own environment. Debian uses amd64/arm64; RPM and Arch use the Linux
  # kernel spellings x86_64/aarch64.
  case "$arch" in
    x86_64) deb_arch=amd64;   rpm_arch=x86_64 ;;
    arm64)  deb_arch=arm64;   rpm_arch=aarch64 ;;
  esac

  # Arch package versions may not contain '-', and nfpm silently DROPS the
  # prerelease suffix rather than translating it: 1.1.0-rc1 would come out as
  # pkgver 1.1.0-1, colliding with the final release. Strip the dash instead, so
  # it becomes 1.1.0rc1. (deb and rpm are left alone: nfpm turns the dash into
  # '~' for those, which is the correct sort-before-the-release convention.)
  archlinux_version="${version//-/}"

  # packager|arch|version|output suffix
  for spec in \
    "deb|$deb_arch|$version|$deb_arch.deb" \
    "rpm|$rpm_arch|$version|.$rpm_arch.rpm" \
    "archlinux|$rpm_arch|$archlinux_version|-$rpm_arch.pkg.tar.zst"; do
    IFS='|' read -r packager packager_arch packager_version suffix <<< "$spec"

    case "$packager" in
      deb)       artifact="olympus_${version}_${suffix}" ;;
      rpm)       artifact="olympus-${version}${suffix}" ;;
      archlinux) artifact="olympus-${version}${suffix}" ;;
    esac

    target="$out_dir/$artifact"
    rm -f "$target"
    (
      cd "$work"
      OLYMPUS_ARCH="$packager_arch" OLYMPUS_VERSION="$packager_version" \
        "$nfpm_bin" package -f "$config_file" -p "$packager" -t "$target"
    ) || die "nfpm failed for $artifact"

    [ -f "$target" ] || die "nfpm reported success but $target is missing"
    [ -s "$target" ] || die "$artifact is empty"

    # Sidecar in sha256sum format, matching scripts/package-release.sh.
    printf '%s  %s\n' "$(sha256 "$target")" "$artifact" > "$target.sha256"

    say "packaged  $artifact  ($(du -h "$target" | cut -f1))"
    packaged="$packaged $artifact"
  done

  say "  ($arch: core $(du -h "$payload/bin/olympus-core" | cut -f1), $(find "$payload/share/olympus/workouts" -name '*.zwo' | wc -l | tr -d ' ') workouts)"
done

[ -n "$packaged" ] || die "no binaries found in $bin_dir (expected olympus-linux-x86_64 and/or olympus-linux-arm64)"

# --- Verify the packages -----------------------------------------------------
# Contents are what matter: a package that builds but ships no workouts, or a
# launcher that is not executable, is worse than a failed release. Each reader
# is optional so the script still runs on a bare host, but --strict refuses to
# publish a set it could not actually open.
say ""
say "verifying package contents..."

inspected=0
for artifact in $packaged; do
  target="$out_dir/$artifact"

  # The sidecar must match a freshly computed hash.
  expected="$(awk '{print $1; exit}' "$target.sha256")"
  actual="$(sha256 "$target")"
  [ "$expected" = "$actual" ] || die "$artifact.sha256 mismatch ($expected vs $actual)"

  listing="$stage/listing"
  if list_package "$target" > "$listing" 2>/dev/null && [ -s "$listing" ]; then
    inspected=$((inspected + 1))

    check_member usr/bin/olympus-core
    check_member usr/bin/olympus

    # Every bundled workout has to be there, not just one of them.
    for zwo in "$workouts_dir"/*.zwo; do
      [ -e "$zwo" ] || continue
      check_member "usr/share/olympus/workouts/$(basename "$zwo")"
    done
    if [ -f "$license_file" ]; then
      check_member usr/share/olympus/LICENSE
    fi

    # Modes: the two executables executable, the data files not.
    check_executable usr/bin/olympus-core yes
    check_executable usr/bin/olympus yes
    for zwo in "$workouts_dir"/*.zwo; do
      [ -e "$zwo" ] || continue
      check_executable "usr/share/olympus/workouts/$(basename "$zwo")" no
    done
    if [ -f "$license_file" ]; then
      check_executable usr/share/olympus/LICENSE no
    fi

    say "  ok  $artifact  $(printf '%.12s' "$expected")..."
  else
    say "  ??  $artifact  (no reader for this format on this host; checksum verified only)"
  fi
done

if [ "$inspected" = 0 ]; then
  if [ "$strict" = 1 ]; then
    die "--strict: no package reader (dpkg-deb, rpm, zstd) available to verify contents"
  fi
  say "warning: installed no package reader (dpkg-deb / rpm / zstd), so contents went unverified"
fi

# --- Summary -----------------------------------------------------------------
if [ -n "$missing" ]; then
  say ""
  say "missing binaries (not packaged):$missing"
  if [ "$strict" = 1 ]; then
    die "--strict: refusing to publish a partial native package set"
  fi
fi

say ""
say "native packages in $out_dir:"
ls -1 "$out_dir" | grep -E '\.(deb|rpm|pkg\.tar\.zst)$' | sed 's/^/  /'
