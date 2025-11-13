# Changelog

## [0.3.0](https://github.com/blakebarnett/locai/compare/locai-core-v0.2.1...locai-core-v0.3.0) (2025-11-13)


### Features

* add automatic memory lifecycle tracking ([7fc6301](https://github.com/blakebarnett/locai/commit/7fc6301338e604b78c1046a4a6a1a89b663f6c43))
* add batch operations API for efficient multi-operation execution ([05335a6](https://github.com/blakebarnett/locai/commit/05335a6dc5fcd14244baa7e56aeb078ff4f2fc0f))
* add dynamic relationship type registry ([3bfaca5](https://github.com/blakebarnett/locai/commit/3bfaca58d456399999635b66957181f4bc64a6e1))
* add enhanced search scoring with multiple signals ([cc3757a](https://github.com/blakebarnett/locai/commit/cc3757a985eb754d514c6f48ec99f345cb936e4a))
* add extensible hook system for memory lifecycle events ([eea052e](https://github.com/blakebarnett/locai/commit/eea052e855e0962a218dc7aa4843c7d8d71d7f0b))
* add temporal search and graph span analysis ([857bae9](https://github.com/blakebarnett/locai/commit/857bae916bd355faf4c5864d24c352d279dc692e))
* **api:** add auto-generation structure for embeddings ([9c1d6d3](https://github.com/blakebarnett/locai/commit/9c1d6d3ad8a42387983f85df12dcd9911cd8a39e))
* **api:** add embedding support with validation and normalization ([4290d94](https://github.com/blakebarnett/locai/commit/4290d94b7519b9d902b76f69ac236d9f1e35a481))
* CLI enhancements and core improvements ([6088407](https://github.com/blakebarnett/locai/commit/60884077b6a61a916cb38a4f806e73cfca384b50))
* integrate new features into API and storage layer ([cd7ea66](https://github.com/blakebarnett/locai/commit/cd7ea669af94b5b02149a7efb8252c81f7c4d0e8))
* **storage:** improve vector search and enforce 1024-dim embeddings ([1e8e9d5](https://github.com/blakebarnett/locai/commit/1e8e9d51aff5637f2f6b689d5a9652ed8a34ae37))


### Bug Fixes

* disable sccache in CI ([c88ac08](https://github.com/blakebarnett/locai/commit/c88ac083fa4d4c20c3c5cd9f7a33229fddca7a2f))
* **docs:** update doctest embedding to use 1024 dimensions ([3459d2b](https://github.com/blakebarnett/locai/commit/3459d2ba589a112deb9a2f5ae6424fa0d775334b))
* resolve all clippy and formatting issues ([a5467ea](https://github.com/blakebarnett/locai/commit/a5467ea65b191fd3109fc6ef0d4eac6bfd5ca286))
* **storage:** resolve BM25 search deserialization issue ([107b763](https://github.com/blakebarnett/locai/commit/107b76371804dd40ec32111b8c885b9f32445bd0))
* support common build architectures ([461d77a](https://github.com/blakebarnett/locai/commit/461d77aaa417e9b422bf1447b970c640817a9d11))
