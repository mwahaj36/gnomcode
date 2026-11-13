# Release update signing

The Rust daemon owns update trust. Set `GNOMCODE_UPDATE_PUBLIC_KEY` to the release Ed25519 public key as 64 hexadecimal characters **before compiling Rust**, including before running the existing Windows package-release.ps1 workflow. `option_env!` embeds the value in the daemon. Cargo tracks changes to this compiler environment input. Confirm the resulting binary, not only the build shell, in the smoke checklist.

An embedded key always wins over the runtime environment. Unsetting, replacing or blanking the runtime variable cannot disable the embedded requirement. An empty or malformed embedded value refuses every installer; the application and existing agents remain usable. With no embedded key, the existing optional runtime-key behavior remains for development compatibility. A release build must embed the reviewed public key. No production key is supplied by this change.

## Key custody

Wahaj supplies an existing Ed25519 signing key through an approved secret manager or encrypted offline vault. Keep the private key outside the source tree, worktrees, artifact directories, cloud-synced folders and chat. Restrict filesystem access to the release account. Keep a separately encrypted recovery backup with an owner-approved recovery procedure. Never commit, upload, print or paste the private key into a command, PR, session log or chat. This runbook does not generate a real key.

For signing, supply exactly one source in the secure release process:

- `GNOMCODE_UPDATE_PRIVATE_KEY_FILE`: absolute path to the owner-provided unencrypted PEM, materialized only for signing with restrictive access; securely remove that temporary material through the secret manager afterward.
- `GNOMCODE_UPDATE_PRIVATE_KEY`: PEM injected directly by the secret manager into the signing subprocess environment. Do not type it into shell history. Prefer the file source when environment inheritance or process inspection is a concern.

Do not pass a private-key variable to the Rust/Electron build. Clear it after the signing subprocess exits. The signer never prints key material and accepts only Ed25519. Its derived public key must match `GNOMCODE_UPDATE_PUBLIC_KEY` from the release build.

## Offline signing procedure

1. Build the reviewed release using the public key only. Complete packaged checks, final OS code signing if approved, notices and SHA-256 generation first. Any later byte change requires new hashes and signatures.
2. Prepare an update manifest with the real stable `version` and `platforms` entries. Each entry names an actual installer using `url`, `sha256` and optionally `file_name`. Use only the verified platform artifacts and approved release origins. Do not publish a manifest with fabricated hashes or placeholder signatures.
3. With the private source injected securely and the matching public variable set, run:

   ```powershell
   node scripts/release/sign-update-manifest.mjs <manifest.json> <assets-directory>
   ```

   The two paths are explicit release inputs. The script reads the corresponding local files, rejects checksum mismatches, writes each asset's 128-character hexadecimal `signature`, then atomically replaces the manifest only after all assets pass. A stale `.signed-part` file prevents overwrite; inspect it before removing it.
4. Independently review the final manifest and verify the signature with the embedded-key candidate. The signature covers the raw installer bytes, matching ADR 0033; the manifest envelope, version, timestamp and notes are not signed. HTTPS origin checks and monotonic version selection remain required. This is update authenticity, not Windows Authenticode, macOS notarization or approval to publish.
5. Claude reviews hashes, signatures, packaged smoke results and caveats. Wahaj signs off before any manifest, installer, tag or GitHub release is uploaded. No command here publishes anything.

## Rotation and recovery

Keep a dated inventory of public-key fingerprints, release versions and owner approval; never include private material. For planned rotation, ship a bridge update signed with the old private key whose new binary embeds the new public key. Keep the old key available securely until the supported old-client upgrade path is resolved. Old clients cannot authenticate a new-key-only artifact automatically. Test both old-to-bridge and bridge-to-new updates before retiring the old signer. A single pinned key does not support simultaneous trusted keys.

If the old private key is lost or compromised, stop publication and obtain an independently trusted manual installer/update procedure from Wahaj. Do not disable signature checking or distribute a replacement key through an unauthenticated manifest. Existing application use continues while automatic installation is blocked.

## Regression commands

`node --test scripts/release/sign-update-manifest.test.mjs` checks that absent/conflicting sources cannot alter the manifest or print a supplied sentinel. A valid owner-key signing roundtrip remains a release-time check; no private test key is generated or committed.

For the Rust embedded-key regression, compile daemon library tests with `GNOMCODE_UPDATE_PUBLIC_KEY=d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a` (RFC 8032 test vector public key only), using `cargo test -p gnomcode-daemon --lib --no-run --message-format=json`. Find the `gnomcode_daemon` test executable in Cargo's compiler-artifact output. Unset the variable **before running that executable**, then run it with `embedded_key_requires_signature_despite_runtime_override --ignored --nocapture`. Repeat with a different runtime public value. Both runs must reject unsigned, invalid-signature and tampered bytes while accepting the RFC signature. No private key is needed. Do not ship that test public key. Rebuild the actual candidate with Wahaj's reviewed public key afterward.
