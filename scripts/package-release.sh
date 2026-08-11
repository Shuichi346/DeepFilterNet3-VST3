#!/bin/zsh

set -euo pipefail

readonly SCRIPT_DIR="${0:A:h}"
readonly REPO_ROOT="${SCRIPT_DIR:h}"
readonly SOURCE_VST3="${REPO_ROOT}/target/bundled/deepfilter-vst.vst3"
readonly SOURCE_CLAP="${REPO_ROOT}/target/bundled/deepfilter-vst.clap"
readonly OUTPUT_DIR="${REPO_ROOT}/dist"

fail() {
    print -u2 -- "error: $*"
    exit 1
}

usage() {
    print -- "Usage: ./scripts/package-release.sh [version]"
    print -- ""
    print -- "Packages the existing ad-hoc-signed Apple Silicon VST3 and CLAP bundles."
    print -- "When version is omitted, it is read from plugin/Cargo.toml."
}

if (( $# > 1 )); then
    usage
    exit 2
fi

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

default_version="$({
    /usr/bin/awk '
        /^\[package\]$/ { in_package = 1; next }
        /^\[/ && in_package { exit }
        in_package && /^version[[:space:]]*=/ {
            value = $0
            sub(/^[^=]*=[[:space:]]*"/, "", value)
            sub(/"[[:space:]]*$/, "", value)
            print value
            exit
        }
    ' "${REPO_ROOT}/plugin/Cargo.toml"
} || true)"

version="${1:-${default_version}}"
[[ -n "${version}" ]] || fail "could not determine the package version"
[[ "${version}" =~ ^[0-9A-Za-z][0-9A-Za-z._-]*$ ]] || fail "invalid version: ${version}"

readonly PACKAGE_NAME="DeepFilterNR-v${version}-macos-arm64"
readonly ARCHIVE_NAME="${PACKAGE_NAME}.zip"
readonly ARCHIVE_PATH="${OUTPUT_DIR}/${ARCHIVE_NAME}"
readonly CHECKSUM_PATH="${ARCHIVE_PATH}.sha256"

require_file() {
    local path="$1"
    [[ -f "${path}" ]] || fail "required file is missing: ${path}"
}

require_bundle() {
    local path="$1"
    [[ -d "${path}" ]] || fail "required bundle is missing: ${path}"
}

validate_bundle() {
    local bundle="$1"
    local binary="${bundle}/Contents/MacOS/deepfilter-vst"
    local architectures
    local signing_info

    [[ -x "${binary}" ]] || fail "bundle executable is missing: ${binary}"

    architectures="$(/usr/bin/lipo -archs "${binary}")"
    [[ "${architectures}" == "arm64" ]] || \
        fail "expected a thin arm64 binary, found: ${architectures} (${bundle})"

    /usr/bin/codesign --verify --deep --strict "${bundle}"
    signing_info="$(/usr/bin/codesign -dvv "${bundle}" 2>&1)"
    [[ "${signing_info}" == *"Signature=adhoc"* ]] || \
        fail "bundle is not ad-hoc signed: ${bundle}"
}

require_bundle "${SOURCE_VST3}"
require_bundle "${SOURCE_CLAP}"
require_file "${REPO_ROOT}/LICENSE"
require_file "${REPO_ROOT}/third-party-licenses/Apache-2.0.txt"
require_file "${REPO_ROOT}/THIRD_PARTY_NOTICES.md"

validate_bundle "${SOURCE_VST3}"
validate_bundle "${SOURCE_CLAP}"

/bin/mkdir -p "${OUTPUT_DIR}"
[[ ! -e "${ARCHIVE_PATH}" ]] || fail "archive already exists: ${ARCHIVE_PATH}"
[[ ! -e "${CHECKSUM_PATH}" ]] || fail "checksum already exists: ${CHECKSUM_PATH}"

stage_root="$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/deepfilter-release.XXXXXX")"
readonly stage_root
readonly payload_dir="${stage_root}/${PACKAGE_NAME}"

cleanup() {
    if [[ -n "${stage_root:-}" && -d "${stage_root}" ]]; then
        case "${stage_root}" in
            "${TMPDIR:-/tmp}"/deepfilter-release.*)
                /bin/rm -rf -- "${stage_root}"
                ;;
            *)
                print -u2 -- "warning: refusing to remove unexpected temporary path: ${stage_root}"
                ;;
        esac
    fi
}
trap cleanup EXIT HUP INT TERM

/bin/mkdir -p \
    "${payload_dir}/Plugins" \
    "${payload_dir}/Third-Party-Licenses"
/usr/bin/ditto "${SOURCE_VST3}" "${payload_dir}/Plugins/deepfilter-vst.vst3"
/usr/bin/ditto "${SOURCE_CLAP}" "${payload_dir}/Plugins/deepfilter-vst.clap"
/usr/bin/ditto "${REPO_ROOT}/LICENSE" "${payload_dir}/LICENSE"
/usr/bin/ditto "${REPO_ROOT}/third-party-licenses/Apache-2.0.txt" \
    "${payload_dir}/Third-Party-Licenses/Apache-2.0.txt"
/usr/bin/ditto "${REPO_ROOT}/THIRD_PARTY_NOTICES.md" \
    "${payload_dir}/THIRD_PARTY_NOTICES.md"

/bin/cat > "${payload_dir}/README.md" <<'README_EOF'
# DeepFilter Noise Reduction

Noise reduction plug-in for Apple Silicon Macs. This package includes both
VST3 and CLAP formats; install only the format your audio host supports.

## Requirements

- Apple Silicon Mac
- macOS 26.x or later (validated configuration)
- A VST3- or CLAP-compatible audio host

## Install

Extract the ZIP and run the commands below from the extracted package folder.

VST3:

```bash
mkdir -p "$HOME/Library/Audio/Plug-Ins/VST3"
ditto "Plugins/deepfilter-vst.vst3" \
  "$HOME/Library/Audio/Plug-Ins/VST3/deepfilter-vst.vst3"
```

CLAP:

```bash
mkdir -p "$HOME/Library/Audio/Plug-Ins/CLAP"
ditto "Plugins/deepfilter-vst.clap" \
  "$HOME/Library/Audio/Plug-Ins/CLAP/deepfilter-vst.clap"
```

Restart or rescan the audio host after installation.

## macOS security

These plug-ins use ad-hoc code signatures. They are not Developer ID signed or
Apple notarized. macOS may quarantine a bundle downloaded from the internet.

Only if you trust the downloaded GitHub Release and have verified its SHA-256
sidecar, remove quarantine from the exact installed bundle you intend to use:

```bash
xattr -dr com.apple.quarantine \
  "$HOME/Library/Audio/Plug-Ins/VST3/deepfilter-vst.vst3"
```

For CLAP, use the exact CLAP path instead:

```bash
xattr -dr com.apple.quarantine \
  "$HOME/Library/Audio/Plug-Ins/CLAP/deepfilter-vst.clap"
```

No `sudo` command is needed for a user-only installation.

## Use

1. Add **DeepFilter Noise Reduction** to a mono or stereo audio track.
2. Leave **Mix** at 100% for the fully processed signal, or reduce it to blend
   in the latency-aligned dry signal.
3. Set **Attenuation Limit** between 0 and 100 dB to limit noise attenuation.
4. If the plug-in is not listed, restart the host or trigger a plug-in rescan.

Supported enhanced sample rates are 44.1, 48, 88.2, 96, 176.4, and 192 kHz.
Unsupported host configurations use unchanged direct bypass instead of silence.

## Integrity and licenses

The GitHub Release should provide a `.zip.sha256` sidecar. Verify it before
installation:

```bash
shasum -a 256 -c DeepFilterNR-v*-macos-arm64.zip.sha256
```

The project code is licensed under MIT; see `LICENSE`. Third-party components
retain their own terms; see `THIRD_PARTY_NOTICES.md` and
`Third-Party-Licenses/`.

The embedded low-latency model comes from DeepFilterNet v0.5.6. At package
creation time, its repository dual-licenses its code but provides no separate
license file for the pretrained model archive.

VST is a registered trademark of Steinberg Media Technologies GmbH.
README_EOF

{
    print -- "Package: ${PACKAGE_NAME}"
    print -- "Project version: ${version}"
    print -- "Platform: macOS"
    print -- "Architecture: arm64 (Apple Silicon)"
    print -- "Plug-in formats: VST3 and CLAP"
    print -- "Code signature: ad hoc"
    print -- "Apple notarization: none"
    print -- "Embedded model: DeepFilterNet v0.5.6 DeepFilterNet3 low-latency model"
    print -- "Project license: MIT"
    print -- "Source: https://github.com/Shuichi346/DeepFilterNet3-VST3"
} > "${payload_dir}/RELEASE_INFO.txt"

(
    cd "${payload_dir}"
    /usr/bin/shasum -a 256 \
        "Plugins/deepfilter-vst.vst3/Contents/MacOS/deepfilter-vst" \
        "Plugins/deepfilter-vst.clap/Contents/MacOS/deepfilter-vst"
) > "${payload_dir}/BINARY_SHA256SUMS.txt"

validate_bundle "${payload_dir}/Plugins/deepfilter-vst.vst3"
validate_bundle "${payload_dir}/Plugins/deepfilter-vst.clap"

/usr/bin/ditto -c -k --norsrc --keepParent \
    "${payload_dir}" "${ARCHIVE_PATH}"
/usr/bin/unzip -tq "${ARCHIVE_PATH}"

(
    cd "${OUTPUT_DIR}"
    /usr/bin/shasum -a 256 "${ARCHIVE_NAME}"
) > "${CHECKSUM_PATH}"

print -- "Created: ${ARCHIVE_PATH}"
print -- "Created: ${CHECKSUM_PATH}"
