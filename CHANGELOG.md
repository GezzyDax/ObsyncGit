# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.2](https://github.com/GezzyDax/ObsyncGit/compare/v0.4.1...v0.4.2) (2025-10-01)


### Bug Fixes

* **git:** clear stale index lock ([a5804a4](https://github.com/GezzyDax/ObsyncGit/commit/a5804a401db9518c8a9d5e3c3cf39446e5b201af))
* **git:** clear stale index lock ([cbfbd5d](https://github.com/GezzyDax/ObsyncGit/commit/cbfbd5de464254541b68f23c995c3a093829adb5))
* **git:** handle stale mtime without io::Error ([f853655](https://github.com/GezzyDax/ObsyncGit/commit/f85365561d4b2425cd45871445ccb10d99ca72f1))

## [0.4.1](https://github.com/GezzyDax/ObsyncGit/compare/v0.4.0...v0.4.1) (2025-09-26)


### Bug Fixes

* remove transitional libgobject apt dependency from installer ([d829969](https://github.com/GezzyDax/ObsyncGit/commit/d829969ada0a60df0535c04c0b53cb1e5377a9ca))
* skip unavailable apt packages ([8f1c4aa](https://github.com/GezzyDax/ObsyncGit/commit/8f1c4aac104e2153851848e3dbe8a78dbfb36bb0))

## [0.4.0](https://github.com/GezzyDax/ObsyncGit/compare/v0.3.0...v0.4.0) (2025-09-26)


### Features

* add desktop control centre and distro autostart hooks ([5c6f12c](https://github.com/GezzyDax/ObsyncGit/commit/5c6f12cb8a303c4fd7b335db60c95ccd1f79d6f6))
* add desktop control centre and distro autostart hooks ([29b7843](https://github.com/GezzyDax/ObsyncGit/commit/29b784389bfedd77ee205f7204be1a01e0c278fd))
* add desktop control centre and distro autostart hooks ([bea5dde](https://github.com/GezzyDax/ObsyncGit/commit/bea5ddea59a7d73a7725e2595f0655cdfbea209e))
* **gui:** add desktop autostart controls ([936a2b7](https://github.com/GezzyDax/ObsyncGit/commit/936a2b7d3a8d41599cfdcca16b465b5be93d856f))


### Bug Fixes

* enable tray icon dependency on supported targets ([821ee45](https://github.com/GezzyDax/ObsyncGit/commit/821ee452fe86a8a030c3279da2e85b5f811fe834))
* enable tray icon through gui feature ([b741489](https://github.com/GezzyDax/ObsyncGit/commit/b74148923daedc89d79ba39ed50df384939006cf))
* gate tray icon to supported platforms ([c50751e](https://github.com/GezzyDax/ObsyncGit/commit/c50751e812066962f52e828241ba8a58f088cab9))
* gate tray icon to supported platforms ([9926a0b](https://github.com/GezzyDax/ObsyncGit/commit/9926a0b4bb6900277249e36d594f9c007ee5a4be))
* resolve Cargo.lock merge conflict by regenerating lockfile ([7894451](https://github.com/GezzyDax/ObsyncGit/commit/789445196b8fb0a499673a2e03fafc14810614bc))

## [0.3.0](https://github.com/GezzyDax/ObsyncGit/compare/v0.2.1...v0.3.0) (2025-09-25)


### Features

* add linux arm release and simplify artifact names ([5a28463](https://github.com/GezzyDax/ObsyncGit/commit/5a28463b3b9dc953250493c9245feb06b3ff6949))
* add windows arm release artifact ([aded8b7](https://github.com/GezzyDax/ObsyncGit/commit/aded8b786674a928552b1d01e81bc237e616bcb3))

## [0.2.1](https://github.com/GezzyDax/ObsyncGit/compare/v0.2.0...v0.2.1) (2025-09-25)


### Bug Fixes

* autostash before rebasing on remote ([5f23f18](https://github.com/GezzyDax/ObsyncGit/commit/5f23f1872c6f6d730bd6f9347f01b8e058b88322))
* drop pipefail for POSIX sh compatibility ([2b9614c](https://github.com/GezzyDax/ObsyncGit/commit/2b9614c54cf02f8825d0ba50050ff49b1c720bfe))
* harden installer for shell portability ([9abeff1](https://github.com/GezzyDax/ObsyncGit/commit/9abeff142db275f05d7f2157fcd39abc45712c90))

## [0.2.0] - UNRELEASED
- Initial tracked release for release-please bootstrap.
