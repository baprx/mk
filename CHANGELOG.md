# Changelog

## [0.7.2](https://github.com/baprx/mk/compare/v0.7.1...v0.7.2) (2025-10-31)


### Bug Fixes

* **deps:** update rust crate etcetera to 0.11.0 ([#42](https://github.com/baprx/mk/issues/42)) ([1354e09](https://github.com/baprx/mk/commit/1354e09aa5c0c49e1752d0e7dccc011f04bcd116))

## [0.7.1](https://github.com/baprx/mk/compare/v0.7.0...v0.7.1) (2025-10-20)


### Bug Fixes

* **completions:** update count_args function name ([b8b1527](https://github.com/baprx/mk/commit/b8b152789a736efc3aee83f5d2670e5685cb0190))

## [0.7.0](https://github.com/baprx/mk/compare/v0.6.1...v0.7.0) (2025-10-19)


### Features

* **completions:** add support for fish ([1e4cdbf](https://github.com/baprx/mk/commit/1e4cdbfc8576039b4da9f91b4863683d37717970))
* **completions:** improve fish completions by disabling the default file/directory for env/output keys ([235e807](https://github.com/baprx/mk/commit/235e807798968dc395f502f6c82607490045910c))

## [0.6.1](https://github.com/baprx/mk/compare/v0.6.0...v0.6.1) (2025-10-18)


### Bug Fixes

* **completion:** include ignored environments and outputs in shell completions ([22c30a7](https://github.com/baprx/mk/commit/22c30a717aa2c18ba8120bfd127e38bdcefbfef1))

## [0.6.0](https://github.com/baprx/mk/compare/v0.5.11...v0.6.0) (2025-10-16)


### Features

* **bump:** add support for OCI registries ([d108083](https://github.com/baprx/mk/commit/d1080834b5dee122716f23be81c62dcb378ae2f1))
* **bump:** bump .appVersion and .version if they were set correctly ([2706515](https://github.com/baprx/mk/commit/27065152fa0f85cb05291b19bbe8c198d0548146))
* improve error handling and reporting for dependency fetch failures ([914ad23](https://github.com/baprx/mk/commit/914ad23d8c57d3fc2960207625cb39844f3c67e6))


### Bug Fixes

* **bump:** normalize docker.io to registry-1.docker.io ([34b8431](https://github.com/baprx/mk/commit/34b843151193a4cdeceee305977d0c139fb8cec6))
* **bump:** simplify oci repository path logic ([67d9c5e](https://github.com/baprx/mk/commit/67d9c5ef0c145d5f9355b03a29af8401049dd630))

## [0.5.11](https://github.com/baprx/mk/compare/v0.5.10...v0.5.11) (2025-10-14)


### Bug Fixes

* **completions:** rewrite custom env/output completion logic which made bash completion broken ([f3e2a2a](https://github.com/baprx/mk/commit/f3e2a2a2a7f6534d4f2282f2c0dceeb9bc14f154))

## [0.5.10](https://github.com/baprx/mk/compare/v0.5.9...v0.5.10) (2025-10-13)


### Bug Fixes

* **ci:** simplify cargo cache keys and enable cross for x86_64 linux ([0b0facd](https://github.com/baprx/mk/commit/0b0facdcc74fe1b284816ef113512a3faa8d6d53))

## [0.5.9](https://github.com/baprx/mk/compare/v0.5.8...v0.5.9) (2025-10-13)


### Bug Fixes

* **ci:** use cross for all targets ([4269428](https://github.com/baprx/mk/commit/4269428f2c20538a38493acb7715d031c82d732c))

## [0.5.8](https://github.com/baprx/mk/compare/v0.5.7...v0.5.8) (2025-10-13)


### Bug Fixes

* **kustomize:** remove project path from generated commands since we use the working_dir during execution ([6a90546](https://github.com/baprx/mk/commit/6a905464bc91a9e55c6c2cf8e1b1cae916f979b3))

## [0.5.7](https://github.com/baprx/mk/compare/v0.5.6...v0.5.7) (2025-10-12)


### Bug Fixes

* **ci:** oh. oh. ([9148e23](https://github.com/baprx/mk/commit/9148e237725a384a0679ebb1999b49c271f36b5b))

## [0.5.6](https://github.com/baprx/mk/compare/v0.5.5...v0.5.6) (2025-10-12)


### Bug Fixes

* **ci:** oh. ([8173d20](https://github.com/baprx/mk/commit/8173d205f4fa47f604362a1fb35fed33db72a266))

## [0.5.5](https://github.com/baprx/mk/compare/v0.5.4...v0.5.5) (2025-10-12)


### Bug Fixes

* **ci:** use cross-rs/cross for cross-compilation ([#12](https://github.com/baprx/mk/issues/12)) ([36c8561](https://github.com/baprx/mk/commit/36c8561f54f2f726490abf1ff2ec7077f78a65c2))

## [0.5.4](https://github.com/baprx/mk/compare/v0.5.3...v0.5.4) (2025-10-12)


### Bug Fixes

* **build:** add linker configuration for aarch64 target ([2afc2af](https://github.com/baprx/mk/commit/2afc2af962a6e48e5f2d07fa62672f5cf602ac8f))

## [0.5.3](https://github.com/baprx/mk/compare/v0.5.2...v0.5.3) (2025-10-12)


### Bug Fixes

* **ci:** install required tools to run the test suites ([368a104](https://github.com/baprx/mk/commit/368a1049453ae2ebfc294774da72e1fd1dbdcd8f))

## [0.5.2](https://github.com/baprx/mk/compare/v0.5.1...v0.5.2) (2025-10-12)


### Bug Fixes

* install gcc-aarch64-linux-gnu for cross-compilation ([483ea9c](https://github.com/baprx/mk/commit/483ea9c31f6662ce343ff88e96c30f7cdc33943a))

## [0.5.1](https://github.com/baprx/mk/compare/v0.5.0...v0.5.1) (2025-10-12)


### Bug Fixes

* enabled vendored openssl feature to fix build on aarch64-unknown-linux-gnu target ([4eac435](https://github.com/baprx/mk/commit/4eac43511a18436a7f45d34d075b0dd6efd369c5))
* streamline test execution by removing redundant parsing step ([052baed](https://github.com/baprx/mk/commit/052baede3d216a677fc49e8772f08cfec8e4b701))

## [0.5.0](https://github.com/baprx/scripts/compare/mk-v0.4.0...mk-v0.5.0) (2025-10-05)


### Features

* add configuration management and technology detection enhancements ([6fd6a63](https://github.com/baprx/scripts/commit/6fd6a630f1a460ef7afa1af9271afbd645904938))
* add dynamic environment completion for shell commands and enhance technology detection ([a8c0b07](https://github.com/baprx/scripts/commit/a8c0b07820afa5ad526a9bfb69f6b396ab5dd908))
* **bump:** add pre-selection for single dependency updates in multi-select prompt ([111639d](https://github.com/baprx/scripts/commit/111639d90f1336888766a68dce492b4393dcdae9))
* **bump:** add recursive scanning and configuration for dependency updates ([66d7db8](https://github.com/baprx/scripts/commit/66d7db8e358c90a62b9611941e76aa766070e6ed))
* **bump:** add support for including pre-release versions in bump commands ([81b76b2](https://github.com/baprx/scripts/commit/81b76b2e3d7225e83a43c9dd25a01b5d8c444a67))
* enhance multitechno detection by mapping actions to technologies ([2d9caf9](https://github.com/baprx/scripts/commit/2d9caf97fd7e0291f3db2a41daddfce33934d31f))
* enhance technology detection to support action-based filtering and selection ([5764621](https://github.com/baprx/scripts/commit/5764621ce20e635e9306eab47d76e9927a38693a))
* implement dependency bump command with interactive updates for Terraform and Helm projects ([4c1d5c0](https://github.com/baprx/scripts/commit/4c1d5c0f861d61e21a3f34218eb960b6adecabd4))
* **techno:** add logging for technology detection process ([02c3e8c](https://github.com/baprx/scripts/commit/02c3e8ccc3ff7dd9db1d0ecb88bc14b5c308b9b7))
* **terraform:** support sub-modules for the bump action ([22a18f2](https://github.com/baprx/scripts/commit/22a18f2da5b122c31986cce8dd04587a1e8b79d8))
* **tests:** add unit tests for version extraction and bump command functionality ([413b878](https://github.com/baprx/scripts/commit/413b878dba1f1ab535152f4f535076dcdce78389))
* update technology detection to return actual path alongside technology ([6b9a52c](https://github.com/baprx/scripts/commit/6b9a52c8bfb10b2278cee03b23da433f907a2a29))


### Bug Fixes

* **bump:** pass full path for terraform tech ([a087b83](https://github.com/baprx/scripts/commit/a087b830c98eeacc9b679ed37de0e60f14052940))

## [0.4.0](https://github.com/baprx/scripts/compare/mk-v0.3.0...mk-v0.4.0) (2025-10-04)


### Features

* **ci:** enhance testing workflow with nextest integration and result parsing ([9205b8f](https://github.com/baprx/scripts/commit/9205b8f39652bca6d5e0a7dff4978af5ffeecde9))

## [0.3.0](https://github.com/baprx/scripts/compare/mk-v0.2.0...mk-v0.3.0) (2025-10-04)


### Features

* **tests:** add integration tests for duplicate command in Terraform and Helm ([6bc7bb8](https://github.com/baprx/scripts/commit/6bc7bb899b635194bf7bb7fd2037da9934b93b75))
* **tests:** enhance integration tests for CLI commands and add dependency management tests ([d20b500](https://github.com/baprx/scripts/commit/d20b500d51b93cb9ed5f9733e44e3b004b1c327e))

## [0.2.0](https://github.com/baprx/scripts/compare/mk-v0.1.0...mk-v0.2.0) (2025-10-04)


### Features

* **helm:** add force option to helm_deps_update for dependency updates ([43a5294](https://github.com/baprx/scripts/commit/43a5294a7d6d399aa6d036fb604bd494afe431d3))
* **mk:** init the rust version ([5a55dc9](https://github.com/baprx/scripts/commit/5a55dc9832fd35c2df109adfa5a6b27e951947ce))
* **mk:** init workflow ([a58683c](https://github.com/baprx/scripts/commit/a58683c243acf59e026b937e69656c40d07f926f))
* update dependencies and refactor commands for improved functionality ([92b9905](https://github.com/baprx/scripts/commit/92b990591ace587a37eb9358a34d3b8f772183cc))
