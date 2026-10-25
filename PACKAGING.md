# Packaging strategy

## Alpha 0.1 status

Packaging remains **not implemented or verified** after Alpha 0.1 implementation. No installer, release archive, CI workflow, signing, or publication is claimed to exist. The immediate deliverable is the working application and verified development/build commands where the environment permits.

Build separately on native Windows and Arch Linux. A successful build on one target does not establish compatibility with the other. Do not use Wine as native Windows build/runtime evidence.

## Artifact formats

| Platform | Initial distribution format | Build environment |
| --- | --- | --- |
| Windows | Portable ZIP containing `typerr.exe` and required notices/runtime files | Native Windows, MSVC Rust toolchain, Microsoft C++ build tools and Windows SDK |
| Arch Linux | Portable tar archive containing `typerr` and required notices | Native Arch Linux with linker/build tooling and verified Iced runtime dependencies |

Use `typerr-0.1.0-windows-x86_64.zip` and `typerr-0.1.0-arch-linux-x86_64.tar.gz` as proposed names only after the actual build architecture has been verified. Do not label an artifact with an architecture or platform it was not built and checked for.

Both fonts (Google Sans Code and Jomhuria), SVG icons, and the English word list are embedded in the executable. The Dracula palette is compiled from `themes/dracula.json`; a packaged executable does not need an external theme file. Include the JSON and `build.rs` in source distributions. The program must launch from an unrelated working directory without accessing repository assets. Include third-party license notices even when the assets themselves are embedded.

## Required inputs and tools

- The pinned Rust toolchain and locked dependency graph described in `BUILD.md`.
- Native build/linker tools and actual renderer/windowing runtime dependencies for the target platform.
- Platform archive tools for ZIP or tar/gzip creation, to be selected and verified when packaging is implemented.
- The Google Sans Code and Jomhuria `OFL.txt` files and copyright notices. Preserve the font's license when distributing it inside the executable.
- Provenance for the project's originally curated English word list; do not copy another project's list without checking and including its applicable license.
- A record of relevant dependency notices and verified runtime requirements for each artifact.

Determine Linux runtime dependencies from the built executable and selected backend, and Windows runtime requirements from the native build. Do not assume that a portable archive means a fully static executable or that every system library should be bundled.

## Planned packaging sequence

1. Complete the formatting, lint, test, and native release-build checks in `BUILD.md`.
2. Verify the generated executable's platform and architecture, then launch it outside the checkout.
3. Stage the executable, notices, and any demonstrably required runtime files in a clean distribution directory.
4. Create the platform archive and inspect its contents; extract it into a new directory and repeat a launch check.
5. Record exact successful commands, tool versions, artifact filenames/sizes, and the environment used.

The native Arch release build command `CARGO_BUILD_JOBS=2 cargo build --locked --release` succeeded. `file target/release/typerr` confirmed an x86_64 ELF, `ldd` resolved its direct shared libraries, and the release binary launched from `/tmp` with embedded assets. These are executable checks, not packaged-archive verification. Windows release building remains unverified. No archive creation commands are documented as working. Add concrete packaging commands here only when they have been executed and checked, or label examples explicitly unverified.

If introduced later, `packaging/` should contain reproducible source-controlled scripts/manifests; `target/`, staging directories, and distribution archives are generated output and should not be committed. Do not create packaging scaffolding solely to imply packaging is implemented.

## Explicitly unimplemented

- Windows installer, code signing, and installer testing.
- Arch PKGBUILD/AUR package and dependency declarations.
- Automated native build/release pipelines and artifact checksums.
- Public release creation or upload.
- Windows and Arch packaged-artifact runtime verification.
- Compatibility claims for additional architectures or Linux distributions.

Record implemented packaging steps and evidence here as they become available. Do not publish or upload artifacts without a separate request.
