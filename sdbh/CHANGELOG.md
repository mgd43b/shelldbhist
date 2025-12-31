# Changelog

## [0.12.0](https://github.com/mgd43b/shelldbhist/compare/v0.11.0...v0.12.0) (2025-12-31)


### Features

* complete Phase 3 UI/UX polish and performance optimizations ([5f0e45f](https://github.com/mgd43b/shelldbhist/commit/5f0e45f81732e4718e07d2ace54801148a8780ad))
* implement enhanced preview features for fzf integration ([d886aca](https://github.com/mgd43b/shelldbhist/commit/d886aca2b8eceee7d42cfbb4d557b4b7afe13771))
* Major test coverage expansion - 54.60% → 65.39% (+10.79%) ([f409864](https://github.com/mgd43b/shelldbhist/commit/f4098644f7f8f8604ba1cad5dd2853eee103704f))

## [0.11.0](https://github.com/mgd43b/shelldbhist/compare/v0.10.0...v0.11.0) (2025-12-31)


### Features

* add enhanced preview system with context-aware command analysis ([af7c758](https://github.com/mgd43b/shelldbhist/commit/af7c758367996674a79710ba6ea0a90d8f6e1362))
* add fzf support to stats commands ([793cfe4](https://github.com/mgd43b/shelldbhist/commit/793cfe46ecac15f73835fb7faf5bff4195e629a5))


### Bug Fixes

* prefix unused parameters with underscores to fix clippy warnings ([980c57b](https://github.com/mgd43b/shelldbhist/commit/980c57b02e003bc5df2b8859cb245c207465d293))
* update tests to match enhanced preview implementation ([0e7513e](https://github.com/mgd43b/shelldbhist/commit/0e7513e5c3b43f6b2ec8e1146ef7a4beea3d09b8))

## [0.10.0](https://github.com/mgd43b/shelldbhist/compare/v0.9.0...v0.10.0) (2025-12-31)


### Features

* add fzf preview pane with command details ([91b7821](https://github.com/mgd43b/shelldbhist/commit/91b7821bc8c08ec8568c16b99c4d5f8886fb3f8a))
* add multi-select support for fzf commands ([0d8f6f9](https://github.com/mgd43b/shelldbhist/commit/0d8f6f9c0f874699715d592367abb348dba18f6b))
* complete Ctrl+R history integration documentation ([3bb4cb4](https://github.com/mgd43b/shelldbhist/commit/3bb4cb497ef127a43f1a077730a17592a4c82944))
* comprehensive test coverage expansion and documentation ([3676be2](https://github.com/mgd43b/shelldbhist/commit/3676be2ae4d8e10cd3fee041b447c72bd14e9b90))
* implement custom fzf configuration system ([4dd7df8](https://github.com/mgd43b/shelldbhist/commit/4dd7df8a0317c09f044f52693ce8fe221fe8c2c3))

## [0.9.0](https://github.com/mgd43b/shelldbhist/compare/v0.8.0...v0.9.0) (2025-12-31)


### Features

* add fzf integration for interactive command selection ([15f4870](https://github.com/mgd43b/shelldbhist/commit/15f48709aa321b0093a472ac1051eda11ac4eec3))

## [0.8.0](https://github.com/mgd43b/shelldbhist/compare/v0.7.0...v0.8.0) (2025-12-24)


### Features

* add database schema inspection command ([0c7ac90](https://github.com/mgd43b/shelldbhist/commit/0c7ac90556e8430b704c7f4006d400991555fad4))
* add database schema inspection command ([bfc1294](https://github.com/mgd43b/shelldbhist/commit/bfc1294de1b7b1bd9f8bd7c866e32a4846f46b46))
* improve sdbh list command ([#13](https://github.com/mgd43b/shelldbhist/issues/13)) ([6070502](https://github.com/mgd43b/shelldbhist/commit/607050279c0a4386b6bc588c05009b2f5478ae1b))
* initial sdbh rust implementation ([69640a4](https://github.com/mgd43b/shelldbhist/commit/69640a4513836e42645c4e46faf0fd9076c0b474))
* **search:** add time filtering via --since-epoch/--days ([b003bec](https://github.com/mgd43b/shelldbhist/commit/b003bec0982cf623902b897a9007c23cdc149cbd))


### Bug Fixes

* bash history parsing with extra spaces ([ac78f3a](https://github.com/mgd43b/shelldbhist/commit/ac78f3adf8a8bc39439e8b72dbe6bec1fc359604))
* **cli:** clarify search supports time filtering ([058b589](https://github.com/mgd43b/shelldbhist/commit/058b589840c255fa501f7e307e796dea1307df0a))
* import skips corrupted rows ([ac083bc](https://github.com/mgd43b/shelldbhist/commit/ac083bcbc856be6828b3ae39ba31cb913c396596))

## [0.7.0](https://github.com/mgd43b/shelldbhist/compare/v0.6.0...v0.7.0) (2025-12-24)


### Features

* add database schema inspection command ([0c7ac90](https://github.com/mgd43b/shelldbhist/commit/0c7ac90556e8430b704c7f4006d400991555fad4))

## [0.6.0](https://github.com/mgd43b/shelldbhist/compare/v0.5.0...v0.6.0) (2025-12-24)


### Features

* add database schema inspection command ([bfc1294](https://github.com/mgd43b/shelldbhist/commit/bfc1294de1b7b1bd9f8bd7c866e32a4846f46b46))

## [0.5.0](https://github.com/mgd43b/shelldbhist/compare/v0.4.0...v0.5.0) (2025-12-24)


### Features

* improve sdbh list command ([#13](https://github.com/mgd43b/shelldbhist/issues/13)) ([6070502](https://github.com/mgd43b/shelldbhist/commit/607050279c0a4386b6bc588c05009b2f5478ae1b))
* initial sdbh rust implementation ([69640a4](https://github.com/mgd43b/shelldbhist/commit/69640a4513836e42645c4e46faf0fd9076c0b474))
* **search:** add time filtering via --since-epoch/--days ([b003bec](https://github.com/mgd43b/shelldbhist/commit/b003bec0982cf623902b897a9007c23cdc149cbd))


### Bug Fixes

* bash history parsing with extra spaces ([ac78f3a](https://github.com/mgd43b/shelldbhist/commit/ac78f3adf8a8bc39439e8b72dbe6bec1fc359604))
* **cli:** clarify search supports time filtering ([058b589](https://github.com/mgd43b/shelldbhist/commit/058b589840c255fa501f7e307e796dea1307df0a))
* import skips corrupted rows ([ac083bc](https://github.com/mgd43b/shelldbhist/commit/ac083bcbc856be6828b3ae39ba31cb913c396596))

## [0.4.0](https://github.com/mgd43b/shelldbhist/compare/v0.3.0...v0.4.0) (2025-12-24)


### Features

* improve sdbh list command ([#13](https://github.com/mgd43b/shelldbhist/issues/13)) ([6070502](https://github.com/mgd43b/shelldbhist/commit/607050279c0a4386b6bc588c05009b2f5478ae1b))

## [0.3.0](https://github.com/mgd43b/shelldbhist/compare/v0.2.1...v0.3.0) (2025-12-21)


### Features

* initial sdbh rust implementation ([69640a4](https://github.com/mgd43b/shelldbhist/commit/69640a4513836e42645c4e46faf0fd9076c0b474))
* **search:** add time filtering via --since-epoch/--days ([b003bec](https://github.com/mgd43b/shelldbhist/commit/b003bec0982cf623902b897a9007c23cdc149cbd))


### Bug Fixes

* bash history parsing with extra spaces ([ac78f3a](https://github.com/mgd43b/shelldbhist/commit/ac78f3adf8a8bc39439e8b72dbe6bec1fc359604))
* **cli:** clarify search supports time filtering ([058b589](https://github.com/mgd43b/shelldbhist/commit/058b589840c255fa501f7e307e796dea1307df0a))
* import skips corrupted rows ([ac083bc](https://github.com/mgd43b/shelldbhist/commit/ac083bcbc856be6828b3ae39ba31cb913c396596))

## [0.2.1](https://github.com/mgd43b/shelldbhist/compare/v0.2.0...v0.2.1) (2025-12-21)


### Bug Fixes

* **cli:** clarify search supports time filtering ([058b589](https://github.com/mgd43b/shelldbhist/commit/058b589840c255fa501f7e307e796dea1307df0a))

## [0.2.0](https://github.com/mgd43b/shelldbhist/compare/v0.1.5...v0.2.0) (2025-12-21)


### Features

* **search:** add time filtering via --since-epoch/--days ([b003bec](https://github.com/mgd43b/shelldbhist/commit/b003bec0982cf623902b897a9007c23cdc149cbd))

## [0.1.6](https://github.com/mgd43b/shelldbhist/compare/sdbh-v0.1.5...sdbh-v0.1.6) (2025-12-21)


### Features

* initial sdbh rust implementation ([69640a4](https://github.com/mgd43b/shelldbhist/commit/69640a4513836e42645c4e46faf0fd9076c0b474))


### Bug Fixes

* bash history parsing with extra spaces ([ac78f3a](https://github.com/mgd43b/shelldbhist/commit/ac78f3adf8a8bc39439e8b72dbe6bec1fc359604))
* import skips corrupted rows ([ac083bc](https://github.com/mgd43b/shelldbhist/commit/ac083bcbc856be6828b3ae39ba31cb913c396596))
