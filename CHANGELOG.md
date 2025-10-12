# Changelog

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
