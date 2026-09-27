# Bundled Android binaries

Android 10+ only lets an app run programs that ship inside its APK, so these are
committed here and copied into the Android project by `scripts/prepare-android.cjs`.
Everything else Ingot runs on Android (Java, Pumpkin, playit) is downloaded when first
needed and started through PRoot, inside its Linux sandbox.

Android only unpacks files named `lib*.so`, hence the renames.

| File                   | From (Termux, aarch64)                        | License      |
| ---------------------- | --------------------------------------------- | ------------ |
| `libproot.so`          | `proot` 5.1.107.95: `bin/proot`               | GPL-2.0      |
| `libproot-loader.so`   | `proot` 5.1.107.95: `libexec/proot/loader`    | GPL-2.0      |
| `libproot-loader32.so` | `proot` 5.1.107.95: `libexec/proot/loader32`  | GPL-2.0      |
| `libtalloc.so`         | `libtalloc` 2.4.3: `lib/libtalloc.so.2.4.3`   | LGPL-3.0-or-later (Termux's metadata says GPL-3.0) |
| `libandroid-shmem.so`  | `libandroid-shmem` 0.7: `lib/libandroid-shmem.so` | BSD-3-Clause |

Packages: https://packages.termux.dev/apt/termux-main (`pool/main/p/proot`,
`pool/main/libt/libtalloc`, `pool/main/liba/libandroid-shmem`).
Sources: https://github.com/termux/proot, https://talloc.samba.org,
https://github.com/termux/libandroid-shmem, with Termux's build scripts at
https://github.com/termux/termux-packages.

To update, download the newer `.deb`s from the Termux repository above, extract the
files listed, rename them as shown and replace them here.
