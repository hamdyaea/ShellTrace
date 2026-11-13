# ShellTrace Linux packages

ShellTrace ships the **Rust CLI** and **Qt6 Widgets GUI** as two commands:
`shelltrace` and `shelltrace-gui`.

## Debian / Ubuntu — native .deb builds

Source-built, distribution-specific packages are produced by
[GitHub Actions](../.github/workflows/packages.yml). Targeted architectures:
**amd64**, with separate builds for Debian 12/13 and Ubuntu 24.04/26.04.
Qt6's runtime ABI package names differ between distributions, so **do not
install an Ubuntu .deb on Debian**.

Find artifacts under the [package workflow runs](https://github.com/hamdyaea/ShellTrace/actions/workflows/packages.yml)
or public assets under [Releases](https://github.com/hamdyaea/ShellTrace/releases) after a version is published.

Install a package matching your distribution:
```bash
sudo apt install ./shelltrace_0.3.0-1_debian13_amd64.deb
shelltrace --version
shelltrace-gui
```

Debian packaging uses `dpkg-shlibdeps` to resolve real Qt6 runtime
dependencies on each build image, including Ubuntu's t64 library renames.

To build a .deb directly on a supported Debian or Ubuntu host:
```bash
sudo apt install build-essential cmake qt6-base-dev dpkg-dev rustc cargo strace bubblewrap
bash packaging/deb/build-deb.sh
```
Rust must support this project's locked dependencies (Rust 1.85+). Use rustup
where the distro rustc is too old.

## Arch / Manjaro — AUR VCS package

The source package is `shelltrace-git` in [packaging/aur](aur/).
It builds the CLI and GUI from GitHub and installs the application icon.

On Manjaro:
```bash
cd ~/ShellTrace/packaging/aur
makepkg -si
```

To publish to AUR, authenticate with a registered public SSH key at
[AUR account settings](https://aur.archlinux.org/account/) and **manually**
run from the ShellTrace checkout:
```bash
bash packaging/aur/publish-aur.sh
```
The script pushes **only** `PKGBUILD` and generated `.SRCINFO` to
`ssh://aur@aur.archlinux.org/shelltrace-git.git` on `master`.
It does not upload binary packages. Verify uniqueness and build quality
before submitting. Maintainer SSH authorization cannot be performed in CI.

To update the AUR package later, rerun the script after changing the packaging
metadata. VCS packages automatically derive their build version from Git history.
