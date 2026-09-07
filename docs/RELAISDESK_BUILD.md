# Building the RelaisDesk client fork

This public repository contains the modified RustDesk client, including its
modified protocol submodule. Clone recursively, then select the **immutable
commit recorded for the binary**, not merely the latest branch tip:

```sh
git clone --recursive --branch relaisdesk/authorization https://github.com/JuLeXoGame/relaisdesk.git
cd relaisdesk
git checkout <client-commit-from-the-release>
git submodule update --init --recursive
git rev-parse HEAD
git submodule status --recursive
```

Replace the placeholder with the recorded commit. Do not replace the pinned
`libs/hbb_common` with the upstream version: its protocol differs.

## Flutter builds

The checked-in [build workflow](../.github/workflows/flutter-build.yml)
contains the platform-specific prerequisites, tool versions, dependency
builds and packaging commands. The [nightly entry point](../.github/workflows/flutter-nightly.yml)
selects the current desktop jobs and can be dispatched manually in a fork.
Disable or configure optional signing/publication integrations for your own
account; no RelaisDesk signing secret is required to compile the AGPL client.
CI artifacts are not a promise of byte-for-byte reproducibility or production
approval. Keep the workflow and dependency versions from the same source commit.

## Legacy Windows Sciter build

For the separately maintained Sciter variant, the self-contained
[PowerShell script](../tools/relaisdesk/build-windows-sciter.ps1) includes the
native-dependency manifest and pinned NASM/Sciter inputs. Run it from a Windows
x64 Visual Studio developer shell with Rust, C++ build tools, CMake, Git and
Python available. It does not build the separate commercial launchers or
embed a private key. This variant is not interchangeable with a Flutter
artifact without testing.

```powershell
./tools/relaisdesk/build-windows-sciter.ps1
```

## Distribution and provenance

Publish the exact client and recursive submodule commits, construction scripts,
dependency/tool versions, applicable third-party notices and artifact SHA-256.
Keep access to the corresponding source for already distributed versions.
A later documentation-only commit does **not** change the recorded source
commit of an existing executable. GitHub source ZIPs alone omit submodule
contents; provide the recursive-clone instructions or a complete source archive.

The RustDesk-derived work remains under [AGPLv3](../LICENCE), with the
[modification notice](../RELAISDESK_FORK.md). Dependencies keep their own
licenses. Independently developed service/launcher components require their
own licensing analysis; this page does not grant rights over absent sources.

SignPath/Authenticode is optional for distribution. Accurately describe an
unsigned executable; an Ed25519 download manifest authenticates release
metadata but is not a Windows Authenticode signature. Never publish release
private keys, device proof keys, live tokens or customer information.
