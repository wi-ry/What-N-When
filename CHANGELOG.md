## [5.0.0](https://github.com/wi-ry/What-N-When/compare/v4.0.2...v5.0.0) (2026-10-09)

### ⚠ BREAKING CHANGES

* rewrite of rust code and link fix
* Complete renaming of application
* the app now ships as a Tauri (Rust + WebView2) application
instead of Electron. Build tooling, installer format (NSIS/MSI instead of a
single portable .exe), and the settings.json storage path have all changed
accordingly.

### Features

* add auto store publishing ([604a846](https://github.com/wi-ry/What-N-When/commit/604a846624146604fb9d6967ad194479b23d7dfa))
* bring back the portable version .exe ([8ee2d09](https://github.com/wi-ry/What-N-When/commit/8ee2d09522dd79488c588a31237f728cc6aa4979))
* Complete renaming of application ([182fd5a](https://github.com/wi-ry/What-N-When/commit/182fd5a52f590cc9fa510b954ae5b5fe8a0bd73b))
* migrate desktop shell from Electron to Tauri ([abfeb87](https://github.com/wi-ry/What-N-When/commit/abfeb87ac44373f4ef358e51050ff174dc9b0465))
* **options:** add real-time transparency preview and fix settings persistence ([6df23e6](https://github.com/wi-ry/What-N-When/commit/6df23e6f337d1b7e977b8d77e78a3184de46ef41))
* switch from electron to tauri ([f9fffcd](https://github.com/wi-ry/What-N-When/commit/f9fffcd63fd9e23b988e753d98e5583801e91b99))
* update icon ([d3915db](https://github.com/wi-ry/What-N-When/commit/d3915dbe833dff8cff20071b96cfa1df01b74a68))
* **widget:** add desktop toggle modes ([b563776](https://github.com/wi-ry/What-N-When/commit/b56377624485b6d45d8213024105d25278505546))

### Bug Fixes

* add missing -i to command ([0d38336](https://github.com/wi-ry/What-N-When/commit/0d3833675024ae9cd364c593c33fa06f1401403f))
* **ci:** locate makeappx on Windows runners ([44f872e](https://github.com/wi-ry/What-N-When/commit/44f872e9bd499b6740e43cfe37e992eb692ce952))
* correct file path ([e892aef](https://github.com/wi-ry/What-N-When/commit/e892aef8a2d28012c8e6be1ed600e40b398894bc))
* even more name changes missed ([4a2d5c1](https://github.com/wi-ry/What-N-When/commit/4a2d5c1e534a4ebc81e03fda2ba9b0fe96058e5b))
* improved icon ([ae9d32d](https://github.com/wi-ry/What-N-When/commit/ae9d32db449afb134c0b4b6c9e110f73cc6c0211))
* local settings for dev builds ([735ca5a](https://github.com/wi-ry/What-N-When/commit/735ca5a7ac7978833423014f7f59d735d465005d))
* **msix:** remove AppListEntry=none to avoid headless app store validation error ([184a6a5](https://github.com/wi-ry/What-N-When/commit/184a6a5f06176f58623ca890135651d1d05e1bb4))
* **msix:** reorder manifest elements to satisfy runFullTrust schema requirement ([aa41b80](https://github.com/wi-ry/What-N-When/commit/aa41b80df6d80aeb3feff459fa98abfd98cb804a))
* **msix:** resolve manifest schema error and extract manifest to template file ([163acc1](https://github.com/wi-ry/What-N-When/commit/163acc1b3d9b7e093c03f45370e4fa4c860d8da9))
* **msix:** use correct case-sensitive EntryPoint value for full-trust apps ([9bfb888](https://github.com/wi-ry/What-N-When/commit/9bfb888bc1d741fb27173243bd1b37b4f459298f))
* **msix:** use Store-reserved package identity name and publisher ([22a85da](https://github.com/wi-ry/What-N-When/commit/22a85da390a7b5742f038e206b3262ab11f721a3))
* prevent double-clicking icons for affecting the widget ([602cde9](https://github.com/wi-ry/What-N-When/commit/602cde9b13494e1d0a05667b8861db08897f90c1))
* **release:** align changelog preset versions ([222b93b](https://github.com/wi-ry/What-N-When/commit/222b93bec0a393149780e1d01135374659a0e6a5))
* **release:** checkout main HEAD instead of stale workflow_run SHA ([f7e23ca](https://github.com/wi-ry/What-N-When/commit/f7e23caef03c549313a5e4f27f9589b29fa1bbf0))
* **release:** trigger on successful main CI runs ([d1d5a56](https://github.com/wi-ry/What-N-When/commit/d1d5a56e538226d3bdd3e91f550fd581e233457b))
* remove dashes from application id ([ea8a9ad](https://github.com/wi-ry/What-N-When/commit/ea8a9ad8a2ae108b90386dff8002f87c895a0ee0))
* remove debug message on startup ([feb1780](https://github.com/wi-ry/What-N-When/commit/feb1780dac622f25c12d081dbbab346e477461f9))
* rename application to What-N-When ([8762d1b](https://github.com/wi-ry/What-N-When/commit/8762d1b6849a5a5f6970a421e50e8789281de298))
* resolve explorer.exe crash ([9e07f1e](https://github.com/wi-ry/What-N-When/commit/9e07f1e2d93d2338e23b669d49b0aa7bf39d0d98))
* resolve issue with app moving to foreground unexpectedly ([aa5548e](https://github.com/wi-ry/What-N-When/commit/aa5548ef3ec3434e6704c97a9db6c34139b39bc0))
* resolve issue with google login ([413d047](https://github.com/wi-ry/What-N-When/commit/413d047c5f21b870cd41e66770fb0ffea2789b6b))
* resolve issue with icon path ([468e116](https://github.com/wi-ry/What-N-When/commit/468e1168111d2524fed0a5d484e85454e11bcc90))
* resolve issue with windows store publishing ([4922bcd](https://github.com/wi-ry/What-N-When/commit/4922bcded3e4c3a5ff51c77e02054a634b97f6c9))
* rewrite of rust code and link fix ([3608b08](https://github.com/wi-ry/What-N-When/commit/3608b08585c644886fc6a83d233bb0653cdaf678))
* **startup:** remove legacy Electron autostart entry ([57d5755](https://github.com/wi-ry/What-N-When/commit/57d575577c46087ea038e0488bb14bad641adbd6))
* update project instructions and Explorer handling ([9f7fba3](https://github.com/wi-ry/What-N-When/commit/9f7fba30d70239765d50e79b295a7507b5673070))

### Documentation

* add MIT license ([bc23e16](https://github.com/wi-ry/What-N-When/commit/bc23e16b242d0ff4c2e9211d4a3d64730baf469c))

### Maintenance

* add automatic release notes ([05366b8](https://github.com/wi-ry/What-N-When/commit/05366b8b85c6f310d7be0c6b85c8c901a0445b76))
* cleanup 1.x.x startup entry ([692ef90](https://github.com/wi-ry/What-N-When/commit/692ef90f17de892438f5915180482292f4a9f6ca))
* **deps-dev:** bump @semantic-release/github from 12.0.9 to 12.0.10 ([8c9e8cc](https://github.com/wi-ry/What-N-When/commit/8c9e8cc2c1b247d8f05153873d41a18d0f8308d5))
* **deps-dev:** bump @semantic-release/npm from 12.0.2 to 13.1.5 ([56228ea](https://github.com/wi-ry/What-N-When/commit/56228eabde12f386686cf18537c7c48a125f9e3e))
* **deps-dev:** bump conventional-changelog-conventionalcommits ([f7d2d67](https://github.com/wi-ry/What-N-When/commit/f7d2d678ffb05fac93b421b64529ec4d51b5c083))
* **deps-dev:** bump js-yaml from 4.3.1 to 4.3.2 ([bf21236](https://github.com/wi-ry/What-N-When/commit/bf21236790bf0188474c521709c19216974f7b79))
* **deps-dev:** bump semantic-release from 24.2.9 to 25.0.9 ([1c5200f](https://github.com/wi-ry/What-N-When/commit/1c5200f5a0e1156c05cb1c00dace13d9765fed8e))
* **deps:** bump actions/cache from 4 to 6 ([c52feca](https://github.com/wi-ry/What-N-When/commit/c52feca7f05d80cbe48a9d933f74f4d65f29fac2))
* **deps:** bump actions/setup-node from 6 to 7 ([391d3d8](https://github.com/wi-ry/What-N-When/commit/391d3d839d3933aed04ac8a7a9a5b979e58ebdb3))
* **deps:** bump cycjimmy/semantic-release-action from 4 to 6 ([aa3ea97](https://github.com/wi-ry/What-N-When/commit/aa3ea9715f54ee1eb54c8e3585b99df98456d49e))
* **deps:** bump sigstore, semantic-release and @semantic-release/npm ([09c5291](https://github.com/wi-ry/What-N-When/commit/09c52914437b8207a0efa1ca93a0fb869280b442))
* **deps:** bump tauri-plugin-autostart in /src-tauri ([3a11596](https://github.com/wi-ry/What-N-When/commit/3a11596ed9326fc988e1caa992f84fc096b85d7f))
* **deps:** bump winreg from 0.55.0 to 0.56.0 in /src-tauri ([7752276](https://github.com/wi-ry/What-N-When/commit/775227647609542509869a28215131ba17e6c625))
* **deps:** update npm dependencies ([7756b4c](https://github.com/wi-ry/What-N-When/commit/7756b4cc0bbbbc2c4230a931cd1969d530b1a95f))
* fix dependabot workflow ([172356f](https://github.com/wi-ry/What-N-When/commit/172356f962adbb77d6af6cf54aeea27a72f4b3d5))
* fix lock file to match ([f48e110](https://github.com/wi-ry/What-N-When/commit/f48e11025b6dcb294051ad9c0b8a96c6274e517f))
* ignore src-tauri/gen ([b23af8f](https://github.com/wi-ry/What-N-When/commit/b23af8f4597f1a11201090a938cdb9b25669dea7))
* package upgrades ([58df262](https://github.com/wi-ry/What-N-When/commit/58df262257238f43c75233ab2527c409a492242e))
* package upgrades ([1076b04](https://github.com/wi-ry/What-N-When/commit/1076b04c0142cb16c258a8dd7dabdd5d90b5a60c))
* **release:** 1.4.0 [skip ci] ([b7b8e37](https://github.com/wi-ry/What-N-When/commit/b7b8e379395d2f5e21051cbb506baa3e4eade4f7))
* **release:** 1.4.1 [skip ci] ([a363322](https://github.com/wi-ry/What-N-When/commit/a36332221700b5703fc920169e11e1229f998415))
* **release:** 1.4.2 [skip ci] ([484436c](https://github.com/wi-ry/What-N-When/commit/484436c59b26e8af5dcfb0e4bf168076b5b36c84))
* **release:** 1.4.3 [skip ci] ([b9fc3b6](https://github.com/wi-ry/What-N-When/commit/b9fc3b6c1ee9fa3643abf0b04ec34ed4887be73e))
* **release:** 1.5.0 [skip ci] ([6d57ad0](https://github.com/wi-ry/What-N-When/commit/6d57ad046c1fab82643efc6e29fb8d8cfa1e462f))
* **release:** 1.5.1 [skip ci] ([5c7fd5d](https://github.com/wi-ry/What-N-When/commit/5c7fd5d8e66645f53db31d0ce2eac67fed2837ce))
* **release:** 2.0.0 [skip ci] ([a5c033c](https://github.com/wi-ry/What-N-When/commit/a5c033c5f61e26ec52286e9f5b20115700d4a574))
* **release:** 2.0.1 [skip ci] ([49eb962](https://github.com/wi-ry/What-N-When/commit/49eb9622651569461c90e5dec6cd8d7ad6941857))
* **release:** 2.0.2 [skip ci] ([9357ace](https://github.com/wi-ry/What-N-When/commit/9357ace2e8720b1051b7aa668faefe47c423c3f9))
* **release:** 2.0.3 [skip ci] ([e4d9080](https://github.com/wi-ry/What-N-When/commit/e4d9080519ff40e8089af94968fb7f5bdc62bbc8))
* **release:** 2.0.4 [skip ci] ([3788499](https://github.com/wi-ry/What-N-When/commit/3788499efe0189930e25ac0a9f584ecd4706869a))
* **release:** 2.0.5 [skip ci] ([6c22772](https://github.com/wi-ry/What-N-When/commit/6c227721880a535055a32936d98b73c6054ed45d))
* **release:** 2.0.6 [skip ci] ([6086631](https://github.com/wi-ry/What-N-When/commit/608663196f1ba4302db11254212bdf84bf55119e))
* **release:** 2.0.7 [skip ci] ([2aba27f](https://github.com/wi-ry/What-N-When/commit/2aba27f7e09b0f4f160764f3811cf27a0bd0a427))
* **release:** 2.0.8 [skip ci] ([a589f5f](https://github.com/wi-ry/What-N-When/commit/a589f5f2440e0e2fffed2db4d8aa446e86bdb937))
* **release:** 2.0.9 [skip ci] ([05d4bfc](https://github.com/wi-ry/What-N-When/commit/05d4bfc8ec5ee97e47a7fd8cbab5c203f8bd106a))
* **release:** 2.1.0 [skip ci] ([1c34e7f](https://github.com/wi-ry/What-N-When/commit/1c34e7f3723e2d8b599a2d30d16d82e762fea0f1))
* **release:** 2.1.1 [skip ci] ([568b71d](https://github.com/wi-ry/What-N-When/commit/568b71d77c7187ed52f37663ba6c88743301aef3))
* **release:** 2.1.2 [skip ci] ([200ecb0](https://github.com/wi-ry/What-N-When/commit/200ecb075e0c726d69884e9f59c25c55e9faa61e))
* **release:** 2.1.3 [skip ci] ([c8ee3a1](https://github.com/wi-ry/What-N-When/commit/c8ee3a1a051e90127f38b753ebae2cb784f58140))
* **release:** 2.1.4 [skip ci] ([bbe1f1d](https://github.com/wi-ry/What-N-When/commit/bbe1f1d125341c68fac55c86488da12b20286c88))
* **release:** 2.1.5 [skip ci] ([208ca25](https://github.com/wi-ry/What-N-When/commit/208ca2556ab2156ebd602577a2d23e987d8c2c6b))
* **release:** 2.1.6 [skip ci] ([e61dd4d](https://github.com/wi-ry/What-N-When/commit/e61dd4df7176746c83624e4b605b61462ea96724))
* **release:** 2.2.0 [skip ci] ([39f2d95](https://github.com/wi-ry/What-N-When/commit/39f2d9593dfe64a68a3d5b927482231e4eb615f8))
* **release:** 2.2.1 [skip ci] ([323ef33](https://github.com/wi-ry/What-N-When/commit/323ef33b0ce9aa7ea1dd6cd549216cd98c6f54a7))
* **release:** 2.2.2 [skip ci] ([175ed31](https://github.com/wi-ry/What-N-When/commit/175ed31d991c9d47d61a1f56641547a4d7b422a1))
* **release:** 2.2.3 [skip ci] ([6302987](https://github.com/wi-ry/What-N-When/commit/6302987214a9bbeb915caff17eaa2f1a7ffd492e))
* **release:** 3.0.0 [skip ci] ([a7194a2](https://github.com/wi-ry/What-N-When/commit/a7194a2edd46fce2b5a89f745a017316a9263766))
* **release:** 3.0.1 [skip ci] ([6f5d3cf](https://github.com/wi-ry/What-N-When/commit/6f5d3cf91c0880723d05b7727f3c851f20cdf291))
* **release:** 3.1.0 [skip ci] ([78fa6b6](https://github.com/wi-ry/What-N-When/commit/78fa6b63e5a47343f410ef83431dd7d369ae36a9))
* **release:** 3.2.0 [skip ci] ([3e99145](https://github.com/wi-ry/What-N-When/commit/3e99145417610e52a3e7dbe6a71a76801de5caef))
* **release:** 3.2.1 [skip ci] ([529aa24](https://github.com/wi-ry/What-N-When/commit/529aa24920c85f4e52851767b49c00d33b5c6d1a))
* **release:** 3.2.2 [skip ci] ([d392570](https://github.com/wi-ry/What-N-When/commit/d392570ec923b25d34cc3469d5ace6a0d01cee80))
* **release:** 3.2.3 [skip ci] ([b43a1ff](https://github.com/wi-ry/What-N-When/commit/b43a1ffac1456d6a6797628501653710435c9fe4))
* **release:** 3.2.3 [skip ci] ([7f2d369](https://github.com/wi-ry/What-N-When/commit/7f2d369963c34dd74993587ba328195070910ebb))
* **release:** 3.2.4 [skip ci] ([ae62b70](https://github.com/wi-ry/What-N-When/commit/ae62b700eb3cd2de765dfe10f7e8dbfe3abe3bfb))
* **release:** 3.2.5 [skip ci] ([5ee08ed](https://github.com/wi-ry/What-N-When/commit/5ee08edc6c553f96af82b53b1ffef7e8b1391877))
* **release:** 4.0.0 [skip ci] ([30b688f](https://github.com/wi-ry/What-N-When/commit/30b688fec995b6331d36d7417c61187b0c050606))
* **release:** 4.0.1 [skip ci] ([469936c](https://github.com/wi-ry/What-N-When/commit/469936cc19005d8da96446086247038691e336be))
* update .husky\pre-push ([7817d60](https://github.com/wi-ry/What-N-When/commit/7817d60f2332ea0b1caf60898ac881f04675dafa))
* update app descriptions in readme and builds ([652aecd](https://github.com/wi-ry/What-N-When/commit/652aecd69667637cde4a453322b9ddc6c6c347aa))
* update cargo lock ([9340060](https://github.com/wi-ry/What-N-When/commit/93400602699d85e799f8c6b98dbc54fa52b65a00))
* update documentation ([618fa86](https://github.com/wi-ry/What-N-When/commit/618fa867be846318fb5ed8a035fde797399504b3))
* update readme ([51be925](https://github.com/wi-ry/What-N-When/commit/51be9258b1e0aa30663f2dc4fde42e5643d202ab))
* upgrade npm packages ([6ef72fe](https://github.com/wi-ry/What-N-When/commit/6ef72fe48724603a5358e442efcfcd7cd5be3eee))
* upgrade npm packages ([959f4d5](https://github.com/wi-ry/What-N-When/commit/959f4d56350cbe8361ef899933e04af6b6401285))

### Continuous Integration

* add MSIX packaging ([3c84ae0](https://github.com/wi-ry/What-N-When/commit/3c84ae0f8f3d6c3ef3afdc10bd69392a0d0977e0))
* cache installed node dependencies ([567c51f](https://github.com/wi-ry/What-N-When/commit/567c51f47927cdf3d8b8226d550319e228a95d85))
* **release:** set Microsoft Store release notes from semantic-release ([4bd1bad](https://github.com/wi-ry/What-N-When/commit/4bd1bad5db459f871823fae436811d3fdd16e740))
* skip installer bundling on pull requests ([221cb8f](https://github.com/wi-ry/What-N-When/commit/221cb8ff8d1633764edc58f8ee652881641bba82))

## [4.0.2](https://github.com/wi-ry/What-N-When/compare/v4.0.1...v4.0.2) (2026-10-04)

### Maintenance

* upgrade npm packages ([029c473](https://github.com/wi-ry/What-N-When/commit/029c4737cf6a3d965d0c2acd296f63e7cdcbb174))

## [4.0.1](https://github.com/wi-ry/What-N-When/compare/v4.0.0...v4.0.1) (2026-10-04)

### Maintenance

* **deps-dev:** bump @semantic-release/github from 12.0.9 to 12.0.10 ([dbf0d82](https://github.com/wi-ry/What-N-When/commit/dbf0d824c384cd5aaac2ee246b04db0afd3d8928))
* **deps:** bump tauri-plugin-autostart in /src-tauri ([d807558](https://github.com/wi-ry/What-N-When/commit/d8075581e37b79be342a5f4bc55caa7cbd9491a6))

## [4.0.0](https://github.com/wi-ry/What-N-When/compare/v3.2.5...v4.0.0) (2026-10-04)

### ⚠ BREAKING CHANGES

* rewrite of rust code and link fix

### Bug Fixes

* rewrite of rust code and link fix ([897f838](https://github.com/wi-ry/What-N-When/commit/897f8388dc0367ffbcd6d429315e82e22f71cb18))

## [3.2.5](https://github.com/wi-ry/What-N-When/compare/v3.2.4...v3.2.5) (2026-09-26)

### Bug Fixes

* resolve issue with windows store publishing ([4199519](https://github.com/wi-ry/What-N-When/commit/4199519fab9ab258aabd89fb47e2e32ab5465743))

## [3.2.4](https://github.com/wi-ry/What-N-When/compare/v3.2.3...v3.2.4) (2026-09-26)

### Bug Fixes

* local settings for dev builds ([0674562](https://github.com/wi-ry/What-N-When/commit/067456203de5676dc2f04d8784d5eaa99967ab1d))

## [3.2.3](https://github.com/wi-ry/What-N-When/compare/v3.2.2...v3.2.3) (2026-09-25)

### Bug Fixes

* prevent double-clicking icons for affecting the widget ([954b3fc](https://github.com/wi-ry/What-N-When/commit/954b3fc88d4d32aa5671df8ba03f67ee780ce094))
* resolve explorer.exe crash ([e2a549d](https://github.com/wi-ry/What-N-When/commit/e2a549d83809a79372404ccd3fbf2c48c7e0f94c))

### Maintenance

* **release:** 3.2.3 [skip ci] ([a8b334e](https://github.com/wi-ry/What-N-When/commit/a8b334e53cd66517a4424304a5e35e4a38b4297c))

## [3.2.3](https://github.com/wi-ry/What-N-When/compare/v3.2.2...v3.2.3) (2026-09-25)

### Bug Fixes

* prevent double-clicking icons for affecting the widget ([954b3fc](https://github.com/wi-ry/What-N-When/commit/954b3fc88d4d32aa5671df8ba03f67ee780ce094))

## [3.2.2](https://github.com/wi-ry/What-N-When/compare/v3.2.1...v3.2.2) (2026-09-19)

### Bug Fixes

* remove debug message on startup ([460c588](https://github.com/wi-ry/What-N-When/commit/460c58888fa31f97b9bfa1e6bc69133d71d9f136))

## [3.2.1](https://github.com/wi-ry/What-N-When/compare/v3.2.0...v3.2.1) (2026-09-17)

### Bug Fixes

* add missing -i to command ([cf44217](https://github.com/wi-ry/What-N-When/commit/cf442175c2f8aa3163ca2436278b2ff5b957117a))
* correct file path ([249d06f](https://github.com/wi-ry/What-N-When/commit/249d06f09833431338b367ab4b9b4c4e1c5f231d))

## [3.2.0](https://github.com/wi-ry/What-N-When/compare/v3.1.0...v3.2.0) (2026-09-17)

### Features

* add auto store publishing ([f1f69e4](https://github.com/wi-ry/What-N-When/commit/f1f69e44f1e58e2e14b7bce73bce9258da1fce53))

## [3.1.0](https://github.com/wi-ry/What-N-When/compare/v3.0.1...v3.1.0) (2026-09-17)

### Features

* **options:** add real-time transparency preview and fix settings persistence ([41adb6b](https://github.com/wi-ry/What-N-When/commit/41adb6b18d7167da5ffed800def08f4abc0cf9b6))

### Bug Fixes

* remove dashes from application id ([2c31a3c](https://github.com/wi-ry/What-N-When/commit/2c31a3c3174d32cf6bd3fd6396268d1556b045c7))

## [3.0.1](https://github.com/wi-ry/What-N-When/compare/v3.0.0...v3.0.1) (2026-09-15)

### Bug Fixes

* even more name changes missed ([33fd1c1](https://github.com/wi-ry/What-N-When/commit/33fd1c170e4f17ea5787dcc999ce192208d0c5d4))

## [3.0.0](https://github.com/wi-ry/What-N-When/compare/v2.2.3...v3.0.0) (2026-09-15)

### ⚠ BREAKING CHANGES

* Complete renaming of application

### Features

* Complete renaming of application ([0337d29](https://github.com/wi-ry/What-N-When/commit/0337d291653536e53f9ef1ef0e35d273b989d2e8))

### Bug Fixes

* resolve issue with icon path ([94e2b54](https://github.com/wi-ry/What-N-When/commit/94e2b54d05fb3c0a8510bd3a1d68060c87b109b8))

## [2.2.3](https://github.com/wi-ry/What-N-When/compare/v2.2.2...v2.2.3) (2026-09-15)

### Bug Fixes

* rename application to What-N-When ([58692b5](https://github.com/wi-ry/What-N-When/commit/58692b58284daf6a4ca94f02b4508c835dea7ec2))

### Maintenance

* update cargo lock ([47aa6b5](https://github.com/wi-ry/What-N-When/commit/47aa6b5b3b07371aa89aa2933ea7a5ebed639bb2))

## [2.2.2](https://github.com/wi-ry/What-N-When/compare/v2.2.1...v2.2.2) (2026-09-13)

### Bug Fixes

* improved icon ([f4a4d0e](https://github.com/wi-ry/What-N-When/commit/f4a4d0e40fdff9a3baa025052fa367212b5cd816))

## [2.2.1](https://github.com/wi-ry/What-N-When/compare/v2.2.0...v2.2.1) (2026-09-12)

### Bug Fixes

* resolve issue with app moving to foreground unexpectedly ([f597fdc](https://github.com/wi-ry/What-N-When/commit/f597fdc04ceb0e0dcc1fde9ebe5e1357cba83740))

## [2.2.0](https://github.com/wi-ry/What-N-When/compare/v2.1.6...v2.2.0) (2026-09-12)

### Features

* update icon ([7221737](https://github.com/wi-ry/What-N-When/commit/72217373260a92bc276b2a3b4ff5d053f3562b3d))

## [2.1.6](https://github.com/wi-ry/What-N-When/compare/v2.1.5...v2.1.6) (2026-09-10)

### Bug Fixes

* **msix:** use Store-reserved package identity name and publisher ([ccae338](https://github.com/wi-ry/What-N-When/commit/ccae338257a9270496784a0e8d8bc57cfb02fa45))

## [2.1.5](https://github.com/wi-ry/What-N-When/compare/v2.1.4...v2.1.5) (2026-09-10)

### Bug Fixes

* **msix:** remove AppListEntry=none to avoid headless app store validation error ([feb99fe](https://github.com/wi-ry/What-N-When/commit/feb99fe46d1726ae97b1c74c0ae1fd3f43142001))

## [2.1.4](https://github.com/wi-ry/What-N-When/compare/v2.1.3...v2.1.4) (2026-09-10)

### Bug Fixes

* **ci:** locate makeappx on Windows runners ([95e7b4b](https://github.com/wi-ry/What-N-When/commit/95e7b4b07bc89095799d5cc07292eebe84251b18))
* **msix:** reorder manifest elements to satisfy runFullTrust schema requirement ([d2c6bad](https://github.com/wi-ry/What-N-When/commit/d2c6bad1a19ccceed02fb674358425f29f6ac43a))
* **msix:** resolve manifest schema error and extract manifest to template file ([cf95b7b](https://github.com/wi-ry/What-N-When/commit/cf95b7b8f50b70429d089c166e7c12b9c2d3c7be))
* **msix:** use correct case-sensitive EntryPoint value for full-trust apps ([301b22e](https://github.com/wi-ry/What-N-When/commit/301b22eb644c8dd83355584dd03f58b12082ceae))

### Maintenance

* **deps-dev:** bump conventional-changelog-conventionalcommits ([a5a01ec](https://github.com/wi-ry/What-N-When/commit/a5a01ec0de5df8ffa77d58548279059d0550702e))
* **deps-dev:** bump js-yaml from 4.3.1 to 4.3.2 ([eb5fd9a](https://github.com/wi-ry/What-N-When/commit/eb5fd9a33140af61cac94425a4547dac570fc734))
* **deps:** bump actions/cache from 4 to 6 ([7d12215](https://github.com/wi-ry/What-N-When/commit/7d122155900274e3bf81bf4726b197a6f5b5a47c))

### Continuous Integration

* add MSIX packaging ([d5e8418](https://github.com/wi-ry/What-N-When/commit/d5e841826dc4f7a7006884f5b80633675bc37316))

## [2.1.3](https://github.com/wi-ry/What-N-When/compare/v2.1.2...v2.1.3) (2026-08-27)

### Documentation

* add MIT license ([61dbd2d](https://github.com/wi-ry/What-N-When/commit/61dbd2ddcc4590439b9b257cb6908a9136ad7a2c))

## [2.1.2](https://github.com/wi-ry/What-N-When/compare/v2.1.1...v2.1.2) (2026-08-27)

### Bug Fixes

* update project instructions and Explorer handling ([4ee2300](https://github.com/wi-ry/What-N-When/commit/4ee2300aaef9dc70d14f19ac5a82cd7f8a26aaac))

### Maintenance

* **deps:** update npm dependencies ([889dda0](https://github.com/wi-ry/What-N-When/commit/889dda0e528a80e8c5f7b51958ccf3a2dce22801))

## [2.1.1](https://github.com/wi-ry/What-N-When/compare/v2.1.0...v2.1.1) (2026-08-26)

### Bug Fixes

* **release:** trigger on successful main CI runs ([3e4cdd2](https://github.com/wi-ry/What-N-When/commit/3e4cdd20cb9a924aa221bcd11d716c6bd8d30d62))

## [2.1.0](https://github.com/wi-ry/What-N-When/compare/v2.0.9...v2.1.0) (2026-08-26)

### Features

* **widget:** add desktop toggle modes ([855628a](https://github.com/wi-ry/What-N-When/commit/855628a32e6448a53f48cbd266ca401446c4417f))

### Bug Fixes

* **release:** align changelog preset versions ([0044e5a](https://github.com/wi-ry/What-N-When/commit/0044e5a9a1394b5d3dde441f64e4ca552a6a5a3a))

### Continuous Integration

* cache installed node dependencies ([9e9cf87](https://github.com/wi-ry/What-N-When/commit/9e9cf87abff49e57bf1a17368d009a3d1f100e21))
* skip installer bundling on pull requests ([249eabe](https://github.com/wi-ry/What-N-When/commit/249eabe3996f206a816f3877d006f4c4e7e09b74))

## [2.0.9](https://github.com/wi-ry/What-N-When/compare/v2.0.8...v2.0.9) (2026-08-16)

## [2.0.8](https://github.com/wi-ry/What-N-When/compare/v2.0.7...v2.0.8) (2026-08-16)

## [2.0.7](https://github.com/wi-ry/What-N-When/compare/v2.0.6...v2.0.7) (2026-08-14)

## [2.0.6](https://github.com/wi-ry/What-N-When/compare/v2.0.5...v2.0.6) (2026-08-13)

## [2.0.5](https://github.com/wi-ry/What-N-When/compare/v2.0.4...v2.0.5) (2026-08-13)

## [2.0.4](https://github.com/wi-ry/What-N-When/compare/v2.0.3...v2.0.4) (2026-08-13)

## [2.0.3](https://github.com/wi-ry/What-N-When/compare/v2.0.2...v2.0.3) (2026-08-13)

## [2.0.2](https://github.com/wi-ry/What-N-When/compare/v2.0.1...v2.0.2) (2026-08-13)

## [2.0.1](https://github.com/wi-ry/What-N-When/compare/v2.0.0...v2.0.1) (2026-08-13)

## [2.0.0](https://github.com/wi-ry/What-N-When/compare/v1.5.1...v2.0.0) (2026-08-13)

## [1.5.1](https://github.com/wi-ry/What-N-When/compare/v1.5.0...v1.5.1) (2026-08-13)

## [1.5.0](https://github.com/wi-ry/What-N-When/compare/v1.4.3...v1.5.0) (2026-08-13)

## [1.4.3](https://github.com/wi-ry/What-N-When/compare/v1.4.2...v1.4.3) (2026-08-13)

## [1.4.2](https://github.com/wi-ry/What-N-When/compare/v1.4.1...v1.4.2) (2026-08-13)

## [1.4.1](https://github.com/wi-ry/What-N-When/compare/v1.4.0...v1.4.1) (2026-08-13)

## [1.4.0](https://github.com/wi-ry/What-N-When/compare/v1.3.2...v1.4.0) (2026-08-13)

## [1.3.2](https://github.com/wi-ry/What-N-When/compare/v1.3.1...v1.3.2) (2026-07-17)

## [1.3.1](https://github.com/wi-ry/What-N-When/compare/v1.3.0...v1.3.1) (2026-07-16)

## [1.3.0](https://github.com/wi-ry/What-N-When/compare/v1.2.1...v1.3.0) (2026-07-15)

## [1.2.1](https://github.com/wi-ry/What-N-When/compare/v1.2.0...v1.2.1) (2026-07-09)

## 1.2.0 (2026-07-09)

* feat: add custom app icon and update title styling ([08f502a](https://github.com/wi-ry/What-N-When/commit/08f502a))

## <small>1.1.1 (2026-05-27)</small>

* Merge branch 'main' of https://github.com/wi-ry/What-N-When ([e8571f9](https://github.com/wi-ry/What-N-When/commit/e8571f9))
* fix:Update sem ver in .exe filename ([5008148](https://github.com/wi-ry/What-N-When/commit/5008148))

## 1.1.0 (2026-05-27)

* fix:Semantic version fixes ([78e961f](https://github.com/wi-ry/What-N-When/commit/78e961f))
* Merge branch 'main' of https://github.com/wi-ry/What-N-When ([3a94d0d](https://github.com/wi-ry/What-N-When/commit/3a94d0d))
* feat:Add Windows .exe to new releases ([4b13ab3](https://github.com/wi-ry/What-N-When/commit/4b13ab3))

## 1.0.0 (2026-05-27)

* fix:Misc CI/CD fixes ([c8b425c](https://github.com/wi-ry/What-N-When/commit/c8b425c))
* feat:Add CI/CD to repo ([edbd383](https://github.com/wi-ry/What-N-When/commit/edbd383))
* Add options to app ([f7f2f39](https://github.com/wi-ry/What-N-When/commit/f7f2f39))
* Electron version ([b94fb02](https://github.com/wi-ry/What-N-When/commit/b94fb02))
* first commit ([8f9c3d7](https://github.com/wi-ry/What-N-When/commit/8f9c3d7))
* Fixes ([e4417b8](https://github.com/wi-ry/What-N-When/commit/e4417b8))
* Updates ([b751dde](https://github.com/wi-ry/What-N-When/commit/b751dde))
