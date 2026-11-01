#!/usr/bin/env bash
# Gnomcode Unix installer. Download and verify before replacing an existing install.
set -euo pipefail

VERSION="0.9.0"
BASE_URL="https://gnomcode.vercel.app/downloads"
INSTALL_DIR="${GNOMCODE_INSTALL_DIR:-${HOME}/.local/bin}"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "${OS}" in linux|darwin) ;; *) echo "Unsupported operating system: ${OS}" >&2; exit 1 ;; esac
case "$(uname -m)" in
  x86_64) ARCH_TAG="x86_64" ;;
  arm64|aarch64) ARCH_TAG="aarch64" ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

command -v curl >/dev/null || { echo 'curl is required.' >&2; exit 1; }
if command -v sha256sum >/dev/null; then
  HASH_TOOL=sha256sum
elif command -v shasum >/dev/null; then
  HASH_TOOL=shasum
else
  echo 'A SHA-256 tool (sha256sum or shasum) is required.' >&2; exit 1
fi
stage="$(mktemp -d)"
replacement=""
trap 'rm -rf "${stage}"; if [ -n "${replacement}" ]; then rm -f "${replacement}"; fi' EXIT
asset="gnomcode-v${VERSION}-${OS}-${ARCH_TAG}.tar.gz"
if ! curl --fail --show-error --silent --location "${BASE_URL}/${asset}" -o "${stage}/${asset}"; then
  echo "No downloadable gnomcode ${VERSION} package for ${OS}-${ARCH_TAG}. Your existing install was preserved." >&2
  exit 1
fi
curl --fail --show-error --silent --location "${BASE_URL}/SHA256SUMS-v${VERSION}.txt" -o "${stage}/SHA256SUMS"
expected="$(awk -v asset="${asset}" '$2 == asset { print $1 }' "${stage}/SHA256SUMS")"
[ "${#expected}" -eq 64 ] || { echo 'Missing or invalid package checksum.' >&2; exit 1; }
if [ "${HASH_TOOL}" = sha256sum ]; then
  actual="$(sha256sum "${stage}/${asset}" | awk '{ print $1 }')"
else
  actual="$(shasum -a 256 "${stage}/${asset}" | awk '{ print $1 }')"
fi
[ "${actual}" = "${expected}" ] || { echo 'Package checksum verification failed.' >&2; exit 1; }
mkdir "${stage}/unpacked"
tar -xzf "${stage}/${asset}" -C "${stage}/unpacked" gnomcode LICENSE THIRD_PARTY_NOTICES.html
chmod 755 "${stage}/unpacked/gnomcode"
[ "$("${stage}/unpacked/gnomcode" --version)" = "gnomcode ${VERSION}" ] || {
  echo 'The downloaded binary cannot run on this system or has the wrong version. Existing install preserved.' >&2; exit 1;
}
mkdir -p "${INSTALL_DIR}" "${INSTALL_DIR}/gnomcode-licenses"
cp "${stage}/unpacked/LICENSE" "${stage}/unpacked/THIRD_PARTY_NOTICES.html" "${INSTALL_DIR}/gnomcode-licenses/"
replacement="$(mktemp "${INSTALL_DIR}/.gnomcode-update.XXXXXX")"
install -m 755 "${stage}/unpacked/gnomcode" "${replacement}"
mv -f "${replacement}" "${INSTALL_DIR}/gnomcode"
replacement=""
printf 'gnomcode %s installed to %s/gnomcode.\n' "${VERSION}" "${INSTALL_DIR}"
printf 'User data and account pairing were preserved. Ensure %s is in PATH.\n' "${INSTALL_DIR}"
