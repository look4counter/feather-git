# Windows 배포용 제3자 라이선스 고지

이 문서는 `Cargo.lock`의 Windows x86_64 일반 의존성 트리를 기준으로 작성했다. 빌드 도구와 개발용 의존성은 제외했다. 빌드 구성이나 의존성 버전이 바뀌면 다시 확인해야 한다.

## 배포 시 지킬 사항

- 아래 구성 요소의 저작권 고지와 라이선스 조건을 실행 파일과 함께 제공한다. 로컬 패키지에 포함된 원문은 [제3자 라이선스 전문](THIRD_PARTY_LICENSES.txt)에 모았다.
- Apache-2.0 구성 요소의 라이선스 전문과 해당하는 `NOTICE`를 유지한다.
- `uluru`의 MPL-2.0 적용 부분은 수령자가 [해당 버전의 소스 코드](https://crates.io/api/v1/crates/uluru/3.1.0/download)를 얻는 방법을 안내한다.
- `libgit2`는 [GPLv2 연결 예외](https://github.com/libgit2/libgit2/blob/main/COPYING)에 따라 함께 사용한다. `libgit2` 자체를 수정해 배포하면 수정본의 소스 제공 조건을 확인한다.

## 구성 요소 목록

| 패키지 | 버전 | 라이선스 표기 | 로컬 원문 |
|---|---:|---|---|
| [adler2](https://crates.io/crates/adler2/2.0.1) | 2.0.1 | `0BSD OR MIT OR Apache-2.0` | Included |
| [aho-corasick](https://crates.io/crates/aho-corasick/1.1.5) | 1.1.5 | `Unlicense OR MIT` | Included |
| [aligned](https://crates.io/crates/aligned/0.4.3) | 0.4.3 | `MIT OR Apache-2.0` | Included |
| [aligned-vec](https://crates.io/crates/aligned-vec/0.6.4) | 0.6.4 | `MIT` | Included |
| [allocator-api2](https://crates.io/crates/allocator-api2/0.2.21) | 0.2.21 | `MIT OR Apache-2.0` | Included |
| [anyhow](https://crates.io/crates/anyhow/1.0.104) | 1.0.104 | `MIT OR Apache-2.0` | Included |
| [arc-swap](https://crates.io/crates/arc-swap/1.9.2) | 1.9.2 | `MIT OR Apache-2.0` | Included |
| [arg_enum_proc_macro](https://crates.io/crates/arg_enum_proc_macro/0.3.4) | 0.3.4 | `MIT` | Included |
| [arrayref](https://crates.io/crates/arrayref/0.3.9) | 0.3.9 | `BSD-2-Clause` | Included |
| [arrayvec](https://crates.io/crates/arrayvec/0.7.8) | 0.7.8 | `MIT OR Apache-2.0` | Included |
| [as-slice](https://crates.io/crates/as-slice/0.2.1) | 0.2.1 | `MIT OR Apache-2.0` | Included |
| [ash](https://crates.io/crates/ash/0.38.0+1.3.281) | 0.38.0+1.3.281 | `MIT OR Apache-2.0` | Included |
| [ash-window](https://crates.io/crates/ash-window/0.13.0) | 0.13.0 | `MIT OR Apache-2.0` | Included |
| [async-channel](https://crates.io/crates/async-channel/1.9.0) | 1.9.0 | `Apache-2.0 OR MIT` | Included |
| [async-channel](https://crates.io/crates/async-channel/2.5.0) | 2.5.0 | `Apache-2.0 OR MIT` | Included |
| [async-compression](https://crates.io/crates/async-compression/0.4.44) | 0.4.44 | `MIT OR Apache-2.0` | Included |
| [async-executor](https://crates.io/crates/async-executor/1.14.0) | 1.14.0 | `Apache-2.0 OR MIT` | Included |
| [async-fs](https://crates.io/crates/async-fs/2.2.0) | 2.2.0 | `Apache-2.0 OR MIT` | Included |
| [async-global-executor](https://crates.io/crates/async-global-executor/2.4.1) | 2.4.1 | `Apache-2.0 OR MIT` | Included |
| [async-io](https://crates.io/crates/async-io/2.6.0) | 2.6.0 | `Apache-2.0 OR MIT` | Included |
| [async-lock](https://crates.io/crates/async-lock/3.4.2) | 3.4.2 | `Apache-2.0 OR MIT` | Included |
| [async-net](https://crates.io/crates/async-net/2.0.0) | 2.0.0 | `Apache-2.0 OR MIT` | Included |
| [async-process](https://crates.io/crates/async-process/2.5.0) | 2.5.0 | `Apache-2.0 OR MIT` | Included |
| [async-std](https://crates.io/crates/async-std/1.13.2) | 1.13.2 | `Apache-2.0 OR MIT` | Included |
| [async-task](https://crates.io/crates/async-task/4.7.1) | 4.7.1 | `Apache-2.0 OR MIT` | Included |
| [async_zip](https://crates.io/crates/async_zip/0.0.17) | 0.0.17 | `MIT` | Included |
| [atomic](https://crates.io/crates/atomic/0.5.3) | 0.5.3 | `Apache-2.0/MIT` | Included |
| [atomic-waker](https://crates.io/crates/atomic-waker/1.1.2) | 1.1.2 | `Apache-2.0 OR MIT` | Included |
| [av-scenechange](https://crates.io/crates/av-scenechange/0.14.1) | 0.14.1 | `MIT` | Included |
| [av1-grain](https://crates.io/crates/av1-grain/0.2.5) | 0.2.5 | `BSD-2-Clause` | Included |
| [avif-serialize](https://crates.io/crates/avif-serialize/0.8.9) | 0.8.9 | `BSD-3-Clause` | Included |
| [base64](https://crates.io/crates/base64/0.22.1) | 0.22.1 | `MIT OR Apache-2.0` | Included |
| [bit-set](https://crates.io/crates/bit-set/0.8.0) | 0.8.0 | `Apache-2.0 OR MIT` | Included |
| [bit-vec](https://crates.io/crates/bit-vec/0.8.0) | 0.8.0 | `Apache-2.0 OR MIT` | Included |
| [bit_field](https://crates.io/crates/bit_field/0.10.3) | 0.10.3 | `Apache-2.0/MIT` | Included |
| [bitflags](https://crates.io/crates/bitflags/1.3.2) | 1.3.2 | `MIT/Apache-2.0` | Included |
| [bitflags](https://crates.io/crates/bitflags/2.13.2) | 2.13.2 | `MIT OR Apache-2.0` | Included |
| [bitstream-io](https://crates.io/crates/bitstream-io/4.10.0) | 4.10.0 | `MIT/Apache-2.0` | Included |
| [blade-graphics](https://crates.io/crates/blade-graphics/0.7.1) | 0.7.1 | `MIT` | Missing |
| [blade-macros](https://crates.io/crates/blade-macros/0.3.0) | 0.3.0 | `MIT` | Missing |
| [blade-util](https://crates.io/crates/blade-util/0.3.0) | 0.3.0 | `MIT` | Missing |
| [block-buffer](https://crates.io/crates/block-buffer/0.10.4) | 0.10.4 | `MIT OR Apache-2.0` | Included |
| [block-buffer](https://crates.io/crates/block-buffer/0.12.1) | 0.12.1 | `MIT OR Apache-2.0` | Included |
| [blocking](https://crates.io/crates/blocking/1.7.0) | 1.7.0 | `Apache-2.0 OR MIT` | Included |
| [bstr](https://crates.io/crates/bstr/1.13.1) | 1.13.1 | `MIT OR Apache-2.0` | Included |
| [bytemuck](https://crates.io/crates/bytemuck/1.25.2) | 1.25.2 | `Zlib OR Apache-2.0 OR MIT` | Included |
| [bytemuck_derive](https://crates.io/crates/bytemuck_derive/1.12.0) | 1.12.0 | `Zlib OR Apache-2.0 OR MIT` | Included |
| [byteorder](https://crates.io/crates/byteorder/1.5.0) | 1.5.0 | `Unlicense OR MIT` | Included |
| [byteorder-lite](https://crates.io/crates/byteorder-lite/0.1.0) | 0.1.0 | `Unlicense OR MIT` | Included |
| [bytes](https://crates.io/crates/bytes/1.12.1) | 1.12.1 | `MIT` | Included |
| [bytesize](https://crates.io/crates/bytesize/2.7.0) | 2.7.0 | `Apache-2.0` | Included |
| [cfg-if](https://crates.io/crates/cfg-if/1.0.5) | 1.0.5 | `MIT OR Apache-2.0` | Included |
| [clru](https://crates.io/crates/clru/0.6.3) | 0.6.3 | `MIT` | Included |
| [codespan-reporting](https://crates.io/crates/codespan-reporting/0.12.0) | 0.12.0 | `Apache-2.0` | Included |
| [color_quant](https://crates.io/crates/color_quant/1.1.0) | 1.1.0 | `MIT` | Included |
| [compression-codecs](https://crates.io/crates/compression-codecs/0.4.39) | 0.4.39 | `MIT OR Apache-2.0` | Included |
| [compression-core](https://crates.io/crates/compression-core/0.4.33) | 0.4.33 | `MIT OR Apache-2.0` | Included |
| [concurrent-queue](https://crates.io/crates/concurrent-queue/2.5.0) | 2.5.0 | `Apache-2.0 OR MIT` | Included |
| [const-oid](https://crates.io/crates/const-oid/0.10.2) | 0.10.2 | `Apache-2.0 OR MIT` | Included |
| [convert_case](https://crates.io/crates/convert_case/0.4.0) | 0.4.0 | `MIT` | Missing |
| [core_maths](https://crates.io/crates/core_maths/0.1.1) | 0.1.1 | `MIT` | Included |
| [cpufeatures](https://crates.io/crates/cpufeatures/0.2.17) | 0.2.17 | `MIT OR Apache-2.0` | Included |
| [cpufeatures](https://crates.io/crates/cpufeatures/0.3.1) | 0.3.1 | `MIT OR Apache-2.0` | Included |
| [crc32fast](https://crates.io/crates/crc32fast/1.5.1) | 1.5.1 | `MIT OR Apache-2.0` | Included |
| [crossbeam-channel](https://crates.io/crates/crossbeam-channel/0.5.16) | 0.5.16 | `MIT OR Apache-2.0` | Included |
| [crossbeam-deque](https://crates.io/crates/crossbeam-deque/0.8.7) | 0.8.7 | `MIT OR Apache-2.0` | Included |
| [crossbeam-epoch](https://crates.io/crates/crossbeam-epoch/0.9.20) | 0.9.20 | `MIT OR Apache-2.0` | Included |
| [crossbeam-queue](https://crates.io/crates/crossbeam-queue/0.3.13) | 0.3.13 | `MIT OR Apache-2.0` | Included |
| [crossbeam-utils](https://crates.io/crates/crossbeam-utils/0.8.22) | 0.8.22 | `MIT OR Apache-2.0` | Included |
| [crypto-common](https://crates.io/crates/crypto-common/0.1.7) | 0.1.7 | `MIT OR Apache-2.0` | Included |
| [crypto-common](https://crates.io/crates/crypto-common/0.2.2) | 0.2.2 | `MIT OR Apache-2.0` | Included |
| [ctor](https://crates.io/crates/ctor/0.4.3) | 0.4.3 | `Apache-2.0 OR MIT` | Included |
| [ctor-proc-macro](https://crates.io/crates/ctor-proc-macro/0.0.6) | 0.0.6 | `Apache-2.0 OR MIT` | Included |
| [dashmap](https://crates.io/crates/dashmap/6.2.1) | 6.2.1 | `MIT` | Included |
| [data-url](https://crates.io/crates/data-url/0.3.2) | 0.3.2 | `MIT OR Apache-2.0` | Included |
| [deflate64](https://crates.io/crates/deflate64/0.1.12) | 0.1.12 | `MIT` | Included |
| [derive_more](https://crates.io/crates/derive_more/0.99.20) | 0.99.20 | `MIT` | Included |
| [digest](https://crates.io/crates/digest/0.10.7) | 0.10.7 | `MIT OR Apache-2.0` | Included |
| [digest](https://crates.io/crates/digest/0.11.3) | 0.11.3 | `MIT OR Apache-2.0` | Included |
| [dirs](https://crates.io/crates/dirs/4.0.0) | 4.0.0 | `MIT OR Apache-2.0` | Included |
| [dirs-sys](https://crates.io/crates/dirs-sys/0.3.7) | 0.3.7 | `MIT OR Apache-2.0` | Included |
| [displaydoc](https://crates.io/crates/displaydoc/0.2.7) | 0.2.7 | `MIT OR Apache-2.0` | Included |
| [dtor](https://crates.io/crates/dtor/0.0.6) | 0.0.6 | `Apache-2.0 OR MIT` | Missing |
| [dtor-proc-macro](https://crates.io/crates/dtor-proc-macro/0.0.5) | 0.0.5 | `Apache-2.0 OR MIT` | Missing |
| [dunce](https://crates.io/crates/dunce/1.0.5) | 1.0.5 | `CC0-1.0 OR MIT-0 OR Apache-2.0` | Included |
| [dyn-clone](https://crates.io/crates/dyn-clone/1.0.20) | 1.0.20 | `MIT OR Apache-2.0` | Included |
| [either](https://crates.io/crates/either/1.18.0) | 1.18.0 | `MIT OR Apache-2.0` | Included |
| [encoding_rs](https://crates.io/crates/encoding_rs/0.8.35) | 0.8.35 | `(Apache-2.0 OR MIT) AND BSD-3-Clause` | Included |
| [equator](https://crates.io/crates/equator/0.4.2) | 0.4.2 | `MIT` | Included |
| [equator-macro](https://crates.io/crates/equator-macro/0.4.2) | 0.4.2 | `MIT` | Included |
| [equivalent](https://crates.io/crates/equivalent/1.0.2) | 1.0.2 | `Apache-2.0 OR MIT` | Included |
| [erased-serde](https://crates.io/crates/erased-serde/0.4.10) | 0.4.10 | `MIT OR Apache-2.0` | Included |
| [errno](https://crates.io/crates/errno/0.3.14) | 0.3.14 | `MIT OR Apache-2.0` | Included |
| [etagere](https://crates.io/crates/etagere/0.2.15) | 0.2.15 | `MIT/Apache-2.0` | Included |
| [euclid](https://crates.io/crates/euclid/0.22.14) | 0.22.14 | `MIT OR Apache-2.0` | Included |
| [event-listener](https://crates.io/crates/event-listener/2.5.3) | 2.5.3 | `Apache-2.0 OR MIT` | Included |
| [event-listener](https://crates.io/crates/event-listener/5.4.2) | 5.4.2 | `Apache-2.0 OR MIT` | Included |
| [event-listener-strategy](https://crates.io/crates/event-listener-strategy/0.5.4) | 0.5.4 | `Apache-2.0 OR MIT` | Included |
| [exr](https://crates.io/crates/exr/1.74.2) | 1.74.2 | `BSD-3-Clause` | Included |
| [faster-hex](https://crates.io/crates/faster-hex/0.10.0) | 0.10.0 | `MIT` | Included |
| [fastrand](https://crates.io/crates/fastrand/1.9.0) | 1.9.0 | `Apache-2.0 OR MIT` | Included |
| [fastrand](https://crates.io/crates/fastrand/2.5.0) | 2.5.0 | `Apache-2.0 OR MIT` | Included |
| [fax](https://crates.io/crates/fax/0.2.7) | 0.2.7 | `MIT` | Included |
| [fdeflate](https://crates.io/crates/fdeflate/0.3.7) | 0.3.7 | `MIT OR Apache-2.0` | Included |
| [filetime](https://crates.io/crates/filetime/0.2.29) | 0.2.29 | `MIT/Apache-2.0` | Included |
| [flate2](https://crates.io/crates/flate2/1.1.10) | 1.1.10 | `MIT OR Apache-2.0` | Included |
| [float-cmp](https://crates.io/crates/float-cmp/0.9.0) | 0.9.0 | `MIT` | Included |
| [float_next_after](https://crates.io/crates/float_next_after/1.0.0) | 1.0.0 | `MIT` | Included |
| [flume](https://crates.io/crates/flume/0.11.1) | 0.11.1 | `Apache-2.0/MIT` | Included |
| [fnv](https://crates.io/crates/fnv/1.0.7) | 1.0.7 | `Apache-2.0 / MIT` | Included |
| [foldhash](https://crates.io/crates/foldhash/0.1.5) | 0.1.5 | `Zlib` | Included |
| [foldhash](https://crates.io/crates/foldhash/0.2.0) | 0.2.0 | `Zlib` | Included |
| [fontdb](https://crates.io/crates/fontdb/0.23.0) | 0.23.0 | `MIT` | Included |
| [form_urlencoded](https://crates.io/crates/form_urlencoded/1.2.2) | 1.2.2 | `MIT OR Apache-2.0` | Included |
| [futf](https://crates.io/crates/futf/0.1.5) | 0.1.5 | `MIT / Apache-2.0` | Included |
| [futures](https://crates.io/crates/futures/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [futures-channel](https://crates.io/crates/futures-channel/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [futures-core](https://crates.io/crates/futures-core/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [futures-executor](https://crates.io/crates/futures-executor/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [futures-io](https://crates.io/crates/futures-io/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [futures-lite](https://crates.io/crates/futures-lite/1.13.0) | 1.13.0 | `Apache-2.0 OR MIT` | Included |
| [futures-lite](https://crates.io/crates/futures-lite/2.6.1) | 2.6.1 | `Apache-2.0 OR MIT` | Included |
| [futures-macro](https://crates.io/crates/futures-macro/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [futures-sink](https://crates.io/crates/futures-sink/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [futures-task](https://crates.io/crates/futures-task/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [futures-util](https://crates.io/crates/futures-util/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | Included |
| [generic-array](https://crates.io/crates/generic-array/0.14.7) | 0.14.7 | `MIT` | Included |
| [getrandom](https://crates.io/crates/getrandom/0.2.17) | 0.2.17 | `MIT OR Apache-2.0` | Included |
| [getrandom](https://crates.io/crates/getrandom/0.3.4) | 0.3.4 | `MIT OR Apache-2.0` | Included |
| [getrandom](https://crates.io/crates/getrandom/0.4.3) | 0.4.3 | `MIT OR Apache-2.0` | Included |
| [gif](https://crates.io/crates/gif/0.14.2) | 0.14.2 | `MIT OR Apache-2.0` | Included |
| [git2](https://crates.io/crates/git2/0.21.0) | 0.21.0 | `MIT OR Apache-2.0` | Included |
| [gix](https://crates.io/crates/gix/0.87.1) | 0.87.1 | `MIT OR Apache-2.0` | Included |
| [gix-actor](https://crates.io/crates/gix-actor/0.42.0) | 0.42.0 | `MIT OR Apache-2.0` | Included |
| [gix-archive](https://crates.io/crates/gix-archive/0.36.1) | 0.36.1 | `MIT OR Apache-2.0` | Included |
| [gix-attributes](https://crates.io/crates/gix-attributes/0.35.0) | 0.35.0 | `MIT OR Apache-2.0` | Included |
| [gix-bitmap](https://crates.io/crates/gix-bitmap/0.4.0) | 0.4.0 | `MIT OR Apache-2.0` | Included |
| [gix-blame](https://crates.io/crates/gix-blame/0.17.1) | 0.17.1 | `MIT OR Apache-2.0` | Included |
| [gix-chunk](https://crates.io/crates/gix-chunk/0.8.0) | 0.8.0 | `MIT OR Apache-2.0` | Included |
| [gix-command](https://crates.io/crates/gix-command/0.10.1) | 0.10.1 | `MIT OR Apache-2.0` | Included |
| [gix-commitgraph](https://crates.io/crates/gix-commitgraph/0.39.0) | 0.39.0 | `MIT OR Apache-2.0` | Included |
| [gix-config](https://crates.io/crates/gix-config/0.60.0) | 0.60.0 | `MIT OR Apache-2.0` | Included |
| [gix-config-value](https://crates.io/crates/gix-config-value/0.19.1) | 0.19.1 | `MIT OR Apache-2.0` | Included |
| [gix-credentials](https://crates.io/crates/gix-credentials/0.40.0) | 0.40.0 | `MIT OR Apache-2.0` | Included |
| [gix-date](https://crates.io/crates/gix-date/0.16.0) | 0.16.0 | `MIT OR Apache-2.0` | Included |
| [gix-diff](https://crates.io/crates/gix-diff/0.67.1) | 0.67.1 | `MIT OR Apache-2.0` | Included |
| [gix-dir](https://crates.io/crates/gix-dir/0.29.1) | 0.29.1 | `MIT OR Apache-2.0` | Included |
| [gix-discover](https://crates.io/crates/gix-discover/0.55.0) | 0.55.0 | `MIT OR Apache-2.0` | Included |
| [gix-error](https://crates.io/crates/gix-error/0.3.2) | 0.3.2 | `MIT OR Apache-2.0` | Included |
| [gix-features](https://crates.io/crates/gix-features/0.49.1) | 0.49.1 | `MIT OR Apache-2.0` | Included |
| [gix-filter](https://crates.io/crates/gix-filter/0.34.0) | 0.34.0 | `MIT OR Apache-2.0` | Included |
| [gix-fs](https://crates.io/crates/gix-fs/0.22.1) | 0.22.1 | `MIT OR Apache-2.0` | Included |
| [gix-glob](https://crates.io/crates/gix-glob/0.27.1) | 0.27.1 | `MIT OR Apache-2.0` | Included |
| [gix-hash](https://crates.io/crates/gix-hash/0.26.2) | 0.26.2 | `MIT OR Apache-2.0` | Included |
| [gix-hashtable](https://crates.io/crates/gix-hashtable/0.16.0) | 0.16.0 | `MIT OR Apache-2.0` | Included |
| [gix-ignore](https://crates.io/crates/gix-ignore/0.22.1) | 0.22.1 | `MIT OR Apache-2.0` | Included |
| [gix-imara-diff](https://crates.io/crates/gix-imara-diff/0.2.5) | 0.2.5 | `Apache-2.0` | Included |
| [gix-index](https://crates.io/crates/gix-index/0.55.0) | 0.55.0 | `MIT OR Apache-2.0` | Included |
| [gix-lock](https://crates.io/crates/gix-lock/24.0.0) | 24.0.0 | `MIT OR Apache-2.0` | Included |
| [gix-macros](https://crates.io/crates/gix-macros/0.1.6) | 0.1.6 | `MIT OR Apache-2.0` | Included |
| [gix-mailmap](https://crates.io/crates/gix-mailmap/0.34.1) | 0.34.1 | `MIT OR Apache-2.0` | Included |
| [gix-merge](https://crates.io/crates/gix-merge/0.20.1) | 0.20.1 | `MIT OR Apache-2.0` | Included |
| [gix-negotiate](https://crates.io/crates/gix-negotiate/0.35.1) | 0.35.1 | `MIT OR Apache-2.0` | Included |
| [gix-note](https://crates.io/crates/gix-note/0.1.1) | 0.1.1 | `MIT OR Apache-2.0` | Included |
| [gix-object](https://crates.io/crates/gix-object/0.64.1) | 0.64.1 | `MIT OR Apache-2.0` | Included |
| [gix-odb](https://crates.io/crates/gix-odb/0.84.0) | 0.84.0 | `MIT OR Apache-2.0` | Included |
| [gix-pack](https://crates.io/crates/gix-pack/0.74.2) | 0.74.2 | `MIT OR Apache-2.0` | Included |
| [gix-packetline](https://crates.io/crates/gix-packetline/0.22.2) | 0.22.2 | `MIT OR Apache-2.0` | Included |
| [gix-path](https://crates.io/crates/gix-path/0.12.6) | 0.12.6 | `MIT OR Apache-2.0` | Included |
| [gix-pathspec](https://crates.io/crates/gix-pathspec/0.20.0) | 0.20.0 | `MIT OR Apache-2.0` | Included |
| [gix-prompt](https://crates.io/crates/gix-prompt/0.17.0) | 0.17.0 | `MIT OR Apache-2.0` | Included |
| [gix-protocol](https://crates.io/crates/gix-protocol/0.65.1) | 0.65.1 | `MIT OR Apache-2.0` | Included |
| [gix-quote](https://crates.io/crates/gix-quote/0.8.0) | 0.8.0 | `MIT OR Apache-2.0` | Included |
| [gix-ref](https://crates.io/crates/gix-ref/0.67.1) | 0.67.1 | `MIT OR Apache-2.0` | Included |
| [gix-refspec](https://crates.io/crates/gix-refspec/0.45.1) | 0.45.1 | `MIT OR Apache-2.0` | Included |
| [gix-revision](https://crates.io/crates/gix-revision/0.49.1) | 0.49.1 | `MIT OR Apache-2.0` | Included |
| [gix-revwalk](https://crates.io/crates/gix-revwalk/0.35.0) | 0.35.0 | `MIT OR Apache-2.0` | Included |
| [gix-sec](https://crates.io/crates/gix-sec/0.14.2) | 0.14.2 | `MIT OR Apache-2.0` | Included |
| [gix-shallow](https://crates.io/crates/gix-shallow/0.13.0) | 0.13.0 | `MIT OR Apache-2.0` | Included |
| [gix-status](https://crates.io/crates/gix-status/0.34.1) | 0.34.1 | `MIT OR Apache-2.0` | Included |
| [gix-submodule](https://crates.io/crates/gix-submodule/0.34.0) | 0.34.0 | `MIT OR Apache-2.0` | Included |
| [gix-tempfile](https://crates.io/crates/gix-tempfile/24.0.0) | 24.0.0 | `MIT OR Apache-2.0` | Included |
| [gix-trace](https://crates.io/crates/gix-trace/0.1.21) | 0.1.21 | `MIT OR Apache-2.0` | Included |
| [gix-transport](https://crates.io/crates/gix-transport/0.59.2) | 0.59.2 | `MIT OR Apache-2.0` | Included |
| [gix-traverse](https://crates.io/crates/gix-traverse/0.61.0) | 0.61.0 | `MIT OR Apache-2.0` | Included |
| [gix-url](https://crates.io/crates/gix-url/0.38.0) | 0.38.0 | `MIT OR Apache-2.0` | Included |
| [gix-utils](https://crates.io/crates/gix-utils/0.3.6) | 0.3.6 | `MIT OR Apache-2.0` | Included |
| [gix-validate](https://crates.io/crates/gix-validate/0.11.4) | 0.11.4 | `MIT OR Apache-2.0` | Included |
| [gix-worktree](https://crates.io/crates/gix-worktree/0.56.0) | 0.56.0 | `MIT OR Apache-2.0` | Included |
| [gix-worktree-state](https://crates.io/crates/gix-worktree-state/0.34.1) | 0.34.1 | `MIT OR Apache-2.0` | Included |
| [gix-worktree-stream](https://crates.io/crates/gix-worktree-stream/0.36.1) | 0.36.1 | `MIT OR Apache-2.0` | Included |
| [gix-zlib](https://crates.io/crates/gix-zlib/0.1.0) | 0.1.0 | `MIT OR Apache-2.0` | Included |
| [globset](https://crates.io/crates/globset/0.4.20) | 0.4.20 | `Unlicense OR MIT` | Included |
| [gpu-alloc](https://crates.io/crates/gpu-alloc/0.6.2) | 0.6.2 | `MIT OR Apache-2.0` | Missing |
| [gpu-alloc-ash](https://crates.io/crates/gpu-alloc-ash/0.7.1) | 0.7.1 | `MIT OR Apache-2.0` | Missing |
| [gpu-alloc-types](https://crates.io/crates/gpu-alloc-types/0.3.1) | 0.3.1 | `MIT OR Apache-2.0` | Missing |
| [gpui](https://crates.io/crates/gpui/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui-macros](https://crates.io/crates/gpui-macros/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_collections](https://crates.io/crates/gpui_collections/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_derive_refineable](https://crates.io/crates/gpui_derive_refineable/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_http_client](https://crates.io/crates/gpui_http_client/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_perf](https://crates.io/crates/gpui_perf/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_refineable](https://crates.io/crates/gpui_refineable/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_semantic_version](https://crates.io/crates/gpui_semantic_version/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_sum_tree](https://crates.io/crates/gpui_sum_tree/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_util](https://crates.io/crates/gpui_util/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [gpui_util_macros](https://crates.io/crates/gpui_util_macros/0.2.2) | 0.2.2 | `Apache-2.0` | Included |
| [grid](https://crates.io/crates/grid/0.18.0) | 0.18.0 | `MIT` | Included |
| [h2](https://crates.io/crates/h2/0.4.19) | 0.4.19 | `MIT` | Included |
| [half](https://crates.io/crates/half/2.7.1) | 2.7.1 | `MIT OR Apache-2.0` | Included |
| [hash32](https://crates.io/crates/hash32/0.3.1) | 0.3.1 | `MIT OR Apache-2.0` | Included |
| [hashbrown](https://crates.io/crates/hashbrown/0.14.5) | 0.14.5 | `MIT OR Apache-2.0` | Included |
| [hashbrown](https://crates.io/crates/hashbrown/0.15.5) | 0.15.5 | `MIT OR Apache-2.0` | Included |
| [hashbrown](https://crates.io/crates/hashbrown/0.16.1) | 0.16.1 | `MIT OR Apache-2.0` | Included |
| [hashbrown](https://crates.io/crates/hashbrown/0.17.1) | 0.17.1 | `MIT OR Apache-2.0` | Included |
| [heapless](https://crates.io/crates/heapless/0.8.0) | 0.8.0 | `MIT OR Apache-2.0` | Included |
| [heck](https://crates.io/crates/heck/0.5.0) | 0.5.0 | `MIT OR Apache-2.0` | Included |
| [hexf-parse](https://crates.io/crates/hexf-parse/0.2.1) | 0.2.1 | `CC0-1.0` | Missing |
| [hidden-trait](https://crates.io/crates/hidden-trait/0.1.2) | 0.1.2 | `MIT` | Included |
| [home](https://crates.io/crates/home/0.5.12) | 0.5.12 | `MIT OR Apache-2.0` | Included |
| [http](https://crates.io/crates/http/1.5.0) | 1.5.0 | `MIT OR Apache-2.0` | Included |
| [http-body](https://crates.io/crates/http-body/1.1.0) | 1.1.0 | `MIT` | Included |
| [http-body-util](https://crates.io/crates/http-body-util/0.1.5) | 0.1.5 | `MIT` | Included |
| [httparse](https://crates.io/crates/httparse/1.10.1) | 1.10.1 | `MIT OR Apache-2.0` | Included |
| [human_format](https://crates.io/crates/human_format/1.2.1) | 1.2.1 | `MIT` | Included |
| [hybrid-array](https://crates.io/crates/hybrid-array/0.4.14) | 0.4.14 | `MIT OR Apache-2.0` | Included |
| [hyper](https://crates.io/crates/hyper/1.12.0) | 1.12.0 | `MIT` | Included |
| [hyper-rustls](https://crates.io/crates/hyper-rustls/0.27.10) | 0.27.10 | `Apache-2.0 OR ISC OR MIT` | Included |
| [hyper-util](https://crates.io/crates/hyper-util/0.1.21) | 0.1.21 | `MIT` | Included |
| [icu_collections](https://crates.io/crates/icu_collections/2.4.0) | 2.4.0 | `Unicode-3.0` | Included |
| [icu_locale_core](https://crates.io/crates/icu_locale_core/2.4.0) | 2.4.0 | `Unicode-3.0` | Included |
| [icu_normalizer](https://crates.io/crates/icu_normalizer/2.4.0) | 2.4.0 | `Unicode-3.0` | Included |
| [icu_normalizer_data](https://crates.io/crates/icu_normalizer_data/2.4.0) | 2.4.0 | `Unicode-3.0` | Included |
| [icu_properties](https://crates.io/crates/icu_properties/2.4.0) | 2.4.0 | `Unicode-3.0` | Included |
| [icu_properties_data](https://crates.io/crates/icu_properties_data/2.4.0) | 2.4.0 | `Unicode-3.0` | Included |
| [icu_provider](https://crates.io/crates/icu_provider/2.4.0) | 2.4.0 | `Unicode-3.0` | Included |
| [idna](https://crates.io/crates/idna/1.1.0) | 1.1.0 | `MIT OR Apache-2.0` | Included |
| [idna_adapter](https://crates.io/crates/idna_adapter/1.2.2) | 1.2.2 | `Apache-2.0 OR MIT` | Included |
| [image](https://crates.io/crates/image/0.25.10) | 0.25.10 | `MIT OR Apache-2.0` | Included |
| [image-webp](https://crates.io/crates/image-webp/0.2.4) | 0.2.4 | `MIT OR Apache-2.0` | Included |
| [imagesize](https://crates.io/crates/imagesize/0.13.0) | 0.13.0 | `MIT` | Included |
| [imgref](https://crates.io/crates/imgref/1.12.3) | 1.12.3 | `CC0-1.0 OR Apache-2.0` | Included |
| [indexmap](https://crates.io/crates/indexmap/2.14.1) | 2.14.1 | `Apache-2.0 OR MIT` | Included |
| [inventory](https://crates.io/crates/inventory/0.3.24) | 0.3.24 | `MIT OR Apache-2.0` | Included |
| [io-close](https://crates.io/crates/io-close/0.3.7) | 0.3.7 | `MIT OR Apache-2.0` | Included |
| [ipnet](https://crates.io/crates/ipnet/2.12.2) | 2.12.2 | `MIT OR Apache-2.0` | Included |
| [itertools](https://crates.io/crates/itertools/0.14.0) | 0.14.0 | `MIT OR Apache-2.0` | Included |
| [itoa](https://crates.io/crates/itoa/1.0.18) | 1.0.18 | `MIT OR Apache-2.0` | Included |
| [jiff](https://crates.io/crates/jiff/0.2.35) | 0.2.35 | `Unlicense OR MIT` | Included |
| [jiff-core](https://crates.io/crates/jiff-core/0.1.0) | 0.1.0 | `Unlicense OR MIT` | Included |
| [jiff-tzdb](https://crates.io/crates/jiff-tzdb/0.1.8) | 0.1.8 | `Unlicense OR MIT` | Included |
| [jiff-tzdb-platform](https://crates.io/crates/jiff-tzdb-platform/0.1.3) | 0.1.3 | `Unlicense OR MIT` | Included |
| [kurbo](https://crates.io/crates/kurbo/0.11.3) | 0.11.3 | `Apache-2.0 OR MIT` | Included |
| [kv-log-macro](https://crates.io/crates/kv-log-macro/1.0.7) | 1.0.7 | `MIT OR Apache-2.0` | Included |
| [lebe](https://crates.io/crates/lebe/0.5.3) | 0.5.3 | `BSD-3-Clause` | Included |
| [libc](https://crates.io/crates/libc/0.2.191) | 0.2.191 | `MIT OR Apache-2.0` | Included |
| [libgit2-sys](https://crates.io/crates/libgit2-sys/0.18.8+1.9.7) | 0.18.8+1.9.7 | `MIT OR Apache-2.0` | Included |
| [libloading](https://crates.io/crates/libloading/0.8.9) | 0.8.9 | `ISC` | Included |
| [libm](https://crates.io/crates/libm/0.2.16) | 0.2.16 | `MIT` | Included |
| [libz-sys](https://crates.io/crates/libz-sys/1.1.31) | 1.1.31 | `MIT OR Apache-2.0` | Included |
| [litemap](https://crates.io/crates/litemap/0.8.3) | 0.8.3 | `Unicode-3.0` | Included |
| [lock_api](https://crates.io/crates/lock_api/0.4.14) | 0.4.14 | `MIT OR Apache-2.0` | Included |
| [log](https://crates.io/crates/log/0.4.34) | 0.4.34 | `MIT OR Apache-2.0` | Included |
| [loop9](https://crates.io/crates/loop9/0.1.5) | 0.1.5 | `MIT` | Included |
| [lyon](https://crates.io/crates/lyon/1.0.19) | 1.0.19 | `MIT OR Apache-2.0` | Missing |
| [lyon_algorithms](https://crates.io/crates/lyon_algorithms/1.0.21) | 1.0.21 | `MIT OR Apache-2.0` | Missing |
| [lyon_geom](https://crates.io/crates/lyon_geom/1.0.19) | 1.0.19 | `MIT OR Apache-2.0` | Missing |
| [lyon_path](https://crates.io/crates/lyon_path/1.0.19) | 1.0.19 | `MIT OR Apache-2.0` | Missing |
| [lyon_tessellation](https://crates.io/crates/lyon_tessellation/1.0.21) | 1.0.21 | `MIT OR Apache-2.0` | Missing |
| [mac](https://crates.io/crates/mac/0.1.1) | 0.1.1 | `MIT/Apache-2.0` | Missing |
| [maybe-rayon](https://crates.io/crates/maybe-rayon/0.1.1) | 0.1.1 | `MIT` | Included |
| [memchr](https://crates.io/crates/memchr/2.8.3) | 2.8.3 | `Unlicense OR MIT` | Included |
| [memmap2](https://crates.io/crates/memmap2/0.9.11) | 0.9.11 | `MIT OR Apache-2.0` | Included |
| [mime](https://crates.io/crates/mime/0.3.17) | 0.3.17 | `MIT OR Apache-2.0` | Included |
| [mime_guess](https://crates.io/crates/mime_guess/2.0.5) | 2.0.5 | `MIT` | Included |
| [miniz_oxide](https://crates.io/crates/miniz_oxide/0.8.9) | 0.8.9 | `MIT OR Zlib OR Apache-2.0` | Included |
| [miniz_oxide](https://crates.io/crates/miniz_oxide/0.9.1) | 0.9.1 | `MIT OR Zlib OR Apache-2.0` | Included |
| [mint](https://crates.io/crates/mint/0.5.9) | 0.5.9 | `MIT` | Included |
| [mio](https://crates.io/crates/mio/1.2.4) | 1.2.4 | `MIT` | Included |
| [moxcms](https://crates.io/crates/moxcms/0.8.1) | 0.8.1 | `BSD-3-Clause OR Apache-2.0` | Included |
| [naga](https://crates.io/crates/naga/25.0.1) | 25.0.1 | `MIT OR Apache-2.0` | Missing |
| [nanorand](https://crates.io/crates/nanorand/0.7.0) | 0.7.0 | `Zlib` | Included |
| [new_debug_unreachable](https://crates.io/crates/new_debug_unreachable/1.0.6) | 1.0.6 | `MIT` | Included |
| [no_std_io2](https://crates.io/crates/no_std_io2/0.9.4) | 0.9.4 | `Apache-2.0 OR MIT` | Included |
| [nom](https://crates.io/crates/nom/8.0.0) | 8.0.0 | `MIT` | Included |
| [nonempty](https://crates.io/crates/nonempty/0.12.0) | 0.12.0 | `MIT` | Included |
| [noop_proc_macro](https://crates.io/crates/noop_proc_macro/0.3.0) | 0.3.0 | `MIT` | Included |
| [num-bigint](https://crates.io/crates/num-bigint/0.4.8) | 0.4.8 | `MIT OR Apache-2.0` | Included |
| [num-complex](https://crates.io/crates/num-complex/0.4.6) | 0.4.6 | `MIT OR Apache-2.0` | Included |
| [num-derive](https://crates.io/crates/num-derive/0.4.2) | 0.4.2 | `MIT OR Apache-2.0` | Included |
| [num-integer](https://crates.io/crates/num-integer/0.1.47) | 0.1.47 | `MIT OR Apache-2.0` | Included |
| [num-rational](https://crates.io/crates/num-rational/0.4.2) | 0.4.2 | `MIT OR Apache-2.0` | Included |
| [num-traits](https://crates.io/crates/num-traits/0.2.19) | 0.2.19 | `MIT OR Apache-2.0` | Included |
| [num_cpus](https://crates.io/crates/num_cpus/1.17.0) | 1.17.0 | `MIT OR Apache-2.0` | Included |
| [once_cell](https://crates.io/crates/once_cell/1.21.4) | 1.21.4 | `MIT OR Apache-2.0` | Included |
| [parking](https://crates.io/crates/parking/2.2.1) | 2.2.1 | `Apache-2.0 OR MIT` | Included |
| [parking_lot](https://crates.io/crates/parking_lot/0.12.5) | 0.12.5 | `MIT OR Apache-2.0` | Included |
| [parking_lot_core](https://crates.io/crates/parking_lot_core/0.9.12) | 0.9.12 | `MIT OR Apache-2.0` | Included |
| [paste](https://crates.io/crates/paste/1.0.15) | 1.0.15 | `MIT OR Apache-2.0` | Included |
| [pastey](https://crates.io/crates/pastey/0.1.1) | 0.1.1 | `MIT OR Apache-2.0` | Included |
| [percent-encoding](https://crates.io/crates/percent-encoding/2.3.2) | 2.3.2 | `MIT OR Apache-2.0` | Included |
| [pico-args](https://crates.io/crates/pico-args/0.5.0) | 0.5.0 | `MIT` | Included |
| [pin-project](https://crates.io/crates/pin-project/1.1.13) | 1.1.13 | `Apache-2.0 OR MIT` | Included |
| [pin-project-internal](https://crates.io/crates/pin-project-internal/1.1.13) | 1.1.13 | `Apache-2.0 OR MIT` | Included |
| [pin-project-lite](https://crates.io/crates/pin-project-lite/0.2.17) | 0.2.17 | `Apache-2.0 OR MIT` | Included |
| [pin-utils](https://crates.io/crates/pin-utils/0.1.0) | 0.1.0 | `MIT OR Apache-2.0` | Included |
| [piper](https://crates.io/crates/piper/0.2.5) | 0.2.5 | `MIT OR Apache-2.0` | Included |
| [png](https://crates.io/crates/png/0.17.16) | 0.17.16 | `MIT OR Apache-2.0` | Included |
| [png](https://crates.io/crates/png/0.18.1) | 0.18.1 | `MIT OR Apache-2.0` | Included |
| [polling](https://crates.io/crates/polling/3.11.0) | 3.11.0 | `Apache-2.0 OR MIT` | Included |
| [pollster](https://crates.io/crates/pollster/0.2.5) | 0.2.5 | `Apache-2.0/MIT` | Included |
| [postage](https://crates.io/crates/postage/0.5.0) | 0.5.0 | `MIT` | Included |
| [potential_utf](https://crates.io/crates/potential_utf/0.1.6) | 0.1.6 | `Unicode-3.0` | Included |
| [ppv-lite86](https://crates.io/crates/ppv-lite86/0.2.21) | 0.2.21 | `MIT OR Apache-2.0` | Included |
| [proc-macro-error-attr2](https://crates.io/crates/proc-macro-error-attr2/2.0.0) | 2.0.0 | `MIT OR Apache-2.0` | Included |
| [proc-macro-error2](https://crates.io/crates/proc-macro-error2/2.0.1) | 2.0.1 | `MIT OR Apache-2.0` | Included |
| [proc-macro2](https://crates.io/crates/proc-macro2/1.0.107) | 1.0.107 | `MIT OR Apache-2.0` | Included |
| [prodash](https://crates.io/crates/prodash/31.0.0) | 31.0.0 | `MIT` | Included |
| [profiling](https://crates.io/crates/profiling/1.0.18) | 1.0.18 | `MIT OR Apache-2.0` | Missing |
| [profiling-procmacros](https://crates.io/crates/profiling-procmacros/1.0.18) | 1.0.18 | `MIT OR Apache-2.0` | Missing |
| [psm](https://crates.io/crates/psm/0.1.32) | 0.1.32 | `MIT OR Apache-2.0` | Included |
| [pulp](https://crates.io/crates/pulp/0.22.3) | 0.22.3 | `MIT` | Included |
| [pulp-wasm-simd-flag](https://crates.io/crates/pulp-wasm-simd-flag/0.1.1) | 0.1.1 | `MIT` | Missing |
| [pxfm](https://crates.io/crates/pxfm/0.1.30) | 0.1.30 | `BSD-3-Clause OR Apache-2.0` | Included |
| [qoi](https://crates.io/crates/qoi/0.4.1) | 0.4.1 | `MIT/Apache-2.0` | Included |
| [quick-error](https://crates.io/crates/quick-error/2.0.1) | 2.0.1 | `MIT/Apache-2.0` | Included |
| [quote](https://crates.io/crates/quote/1.0.47) | 1.0.47 | `MIT OR Apache-2.0` | Included |
| [rand](https://crates.io/crates/rand/0.9.5) | 0.9.5 | `MIT OR Apache-2.0` | Included |
| [rand_chacha](https://crates.io/crates/rand_chacha/0.9.0) | 0.9.0 | `MIT OR Apache-2.0` | Included |
| [rand_core](https://crates.io/crates/rand_core/0.9.5) | 0.9.5 | `MIT OR Apache-2.0` | Included |
| [rav1e](https://crates.io/crates/rav1e/0.8.1) | 0.8.1 | `BSD-2-Clause` | Included |
| [ravif](https://crates.io/crates/ravif/0.13.0) | 0.13.0 | `BSD-3-Clause` | Included |
| [raw-cpuid](https://crates.io/crates/raw-cpuid/11.6.0) | 11.6.0 | `MIT` | Included |
| [raw-window-handle](https://crates.io/crates/raw-window-handle/0.6.2) | 0.6.2 | `MIT OR Apache-2.0 OR Zlib` | Included |
| [rayon](https://crates.io/crates/rayon/1.12.0) | 1.12.0 | `MIT OR Apache-2.0` | Included |
| [rayon-core](https://crates.io/crates/rayon-core/1.13.0) | 1.13.0 | `MIT OR Apache-2.0` | Included |
| [reborrow](https://crates.io/crates/reborrow/0.5.5) | 0.5.5 | `MIT` | Included |
| [ref-cast](https://crates.io/crates/ref-cast/1.0.27) | 1.0.27 | `MIT OR Apache-2.0` | Included |
| [ref-cast-impl](https://crates.io/crates/ref-cast-impl/1.0.27) | 1.0.27 | `MIT OR Apache-2.0` | Included |
| [regex](https://crates.io/crates/regex/1.13.1) | 1.13.1 | `MIT OR Apache-2.0` | Included |
| [regex-automata](https://crates.io/crates/regex-automata/0.4.18) | 0.4.18 | `MIT OR Apache-2.0` | Included |
| [regex-syntax](https://crates.io/crates/regex-syntax/0.8.11) | 0.8.11 | `MIT OR Apache-2.0` | Included |
| [resvg](https://crates.io/crates/resvg/0.45.1) | 0.45.1 | `Apache-2.0 OR MIT` | Included |
| [rgb](https://crates.io/crates/rgb/0.8.53) | 0.8.53 | `MIT` | Included |
| [ring](https://crates.io/crates/ring/0.17.14) | 0.17.14 | `Apache-2.0 AND ISC` | Included |
| [roxmltree](https://crates.io/crates/roxmltree/0.20.0) | 0.20.0 | `MIT OR Apache-2.0` | Included |
| [rust-embed](https://crates.io/crates/rust-embed/8.12.0) | 8.12.0 | `MIT` | Included |
| [rust-embed-impl](https://crates.io/crates/rust-embed-impl/8.12.0) | 8.12.0 | `MIT` | Included |
| [rust-embed-utils](https://crates.io/crates/rust-embed-utils/8.12.0) | 8.12.0 | `MIT` | Included |
| [rustc-hash](https://crates.io/crates/rustc-hash/1.1.0) | 1.1.0 | `Apache-2.0/MIT` | Included |
| [rustc-hash](https://crates.io/crates/rustc-hash/2.1.3) | 2.1.3 | `Apache-2.0 OR MIT` | Included |
| [rustix](https://crates.io/crates/rustix/1.1.4) | 1.1.4 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | Included |
| [rustls](https://crates.io/crates/rustls/0.23.46) | 0.23.46 | `Apache-2.0 OR ISC OR MIT` | Included |
| [rustls-native-certs](https://crates.io/crates/rustls-native-certs/0.8.4) | 0.8.4 | `Apache-2.0 OR ISC OR MIT` | Included |
| [rustls-pemfile](https://crates.io/crates/rustls-pemfile/2.2.0) | 2.2.0 | `Apache-2.0 OR ISC OR MIT` | Included |
| [rustls-pki-types](https://crates.io/crates/rustls-pki-types/1.15.1) | 1.15.1 | `MIT OR Apache-2.0` | Included |
| [rustls-webpki](https://crates.io/crates/rustls-webpki/0.103.16) | 0.103.16 | `ISC` | Included |
| [rustversion](https://crates.io/crates/rustversion/1.0.23) | 1.0.23 | `MIT OR Apache-2.0` | Included |
| [rustybuzz](https://crates.io/crates/rustybuzz/0.20.1) | 0.20.1 | `MIT` | Included |
| [ryu](https://crates.io/crates/ryu/1.0.23) | 1.0.23 | `Apache-2.0 OR BSL-1.0` | Included |
| [same-file](https://crates.io/crates/same-file/1.0.6) | 1.0.6 | `Unlicense/MIT` | Included |
| [schannel](https://crates.io/crates/schannel/0.1.29) | 0.1.29 | `MIT` | Included |
| [schemars](https://crates.io/crates/schemars/1.2.2) | 1.2.2 | `MIT` | Included |
| [schemars_derive](https://crates.io/crates/schemars_derive/1.2.2) | 1.2.2 | `MIT` | Included |
| [scopeguard](https://crates.io/crates/scopeguard/1.2.0) | 1.2.0 | `MIT OR Apache-2.0` | Included |
| [seahash](https://crates.io/crates/seahash/4.1.0) | 4.1.0 | `MIT` | Missing |
| [serde](https://crates.io/crates/serde/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | Included |
| [serde_core](https://crates.io/crates/serde_core/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | Included |
| [serde_derive](https://crates.io/crates/serde_derive/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | Included |
| [serde_derive_internals](https://crates.io/crates/serde_derive_internals/0.30.0) | 0.30.0 | `MIT OR Apache-2.0` | Included |
| [serde_fmt](https://crates.io/crates/serde_fmt/1.1.0) | 1.1.0 | `Apache-2.0 OR MIT` | Included |
| [serde_json](https://crates.io/crates/serde_json/1.0.154) | 1.0.154 | `MIT OR Apache-2.0` | Included |
| [serde_json_lenient](https://crates.io/crates/serde_json_lenient/0.2.4) | 0.2.4 | `MIT/Apache-2.0` | Included |
| [serde_urlencoded](https://crates.io/crates/serde_urlencoded/0.7.1) | 0.7.1 | `MIT/Apache-2.0` | Included |
| [sha1](https://crates.io/crates/sha1/0.10.7) | 0.10.7 | `MIT OR Apache-2.0` | Included |
| [sha1-checked](https://crates.io/crates/sha1-checked/0.10.0) | 0.10.0 | `MIT OR Apache-2.0` | Included |
| [sha1_smol](https://crates.io/crates/sha1_smol/1.0.1) | 1.0.1 | `BSD-3-Clause` | Included |
| [sha2](https://crates.io/crates/sha2/0.10.9) | 0.10.9 | `MIT OR Apache-2.0` | Included |
| [sha2](https://crates.io/crates/sha2/0.11.0) | 0.11.0 | `MIT OR Apache-2.0` | Included |
| [shlex](https://crates.io/crates/shlex/1.3.0) | 1.3.0 | `MIT OR Apache-2.0` | Included |
| [signal-hook](https://crates.io/crates/signal-hook/0.4.4) | 0.4.4 | `MIT OR Apache-2.0` | Included |
| [signal-hook-registry](https://crates.io/crates/signal-hook-registry/1.4.8) | 1.4.8 | `MIT OR Apache-2.0` | Included |
| [simd-adler32](https://crates.io/crates/simd-adler32/0.3.10) | 0.3.10 | `MIT` | Included |
| [simd_helpers](https://crates.io/crates/simd_helpers/0.1.0) | 0.1.0 | `MIT` | Missing |
| [similar](https://crates.io/crates/similar/2.7.0) | 2.7.0 | `Apache-2.0` | Included |
| [simplecss](https://crates.io/crates/simplecss/0.2.2) | 0.2.2 | `Apache-2.0 OR MIT` | Included |
| [siphasher](https://crates.io/crates/siphasher/1.0.3) | 1.0.3 | `MIT/Apache-2.0` | Included |
| [slab](https://crates.io/crates/slab/0.4.12) | 0.4.12 | `MIT` | Included |
| [slotmap](https://crates.io/crates/slotmap/1.1.1) | 1.1.1 | `Zlib` | Included |
| [smallvec](https://crates.io/crates/smallvec/1.16.4) | 1.16.4 | `MIT OR Apache-2.0` | Included |
| [smol](https://crates.io/crates/smol/2.0.2) | 2.0.2 | `Apache-2.0 OR MIT` | Included |
| [socket2](https://crates.io/crates/socket2/0.6.5) | 0.6.5 | `MIT OR Apache-2.0` | Included |
| [spin](https://crates.io/crates/spin/0.9.9) | 0.9.9 | `MIT` | Included |
| [spirv](https://crates.io/crates/spirv/0.3.0+sdk-1.3.268.0) | 0.3.0+sdk-1.3.268.0 | `Apache-2.0` | Missing |
| [stable_deref_trait](https://crates.io/crates/stable_deref_trait/1.2.1) | 1.2.1 | `MIT OR Apache-2.0` | Included |
| [stacker](https://crates.io/crates/stacker/0.1.25) | 0.1.25 | `MIT OR Apache-2.0` | Included |
| [stacksafe](https://crates.io/crates/stacksafe/0.1.4) | 0.1.4 | `Apache-2.0` | Missing |
| [stacksafe-macro](https://crates.io/crates/stacksafe-macro/0.1.4) | 0.1.4 | `Apache-2.0` | Missing |
| [static_assertions](https://crates.io/crates/static_assertions/1.1.0) | 1.1.0 | `MIT OR Apache-2.0` | Included |
| [strict-num](https://crates.io/crates/strict-num/0.1.1) | 0.1.1 | `MIT` | Included |
| [strum](https://crates.io/crates/strum/0.26.3) | 0.26.3 | `MIT` | Included |
| [strum](https://crates.io/crates/strum/0.27.2) | 0.27.2 | `MIT` | Included |
| [strum_macros](https://crates.io/crates/strum_macros/0.26.4) | 0.26.4 | `MIT` | Included |
| [strum_macros](https://crates.io/crates/strum_macros/0.27.2) | 0.27.2 | `MIT` | Included |
| [subtle](https://crates.io/crates/subtle/2.6.1) | 2.6.1 | `BSD-3-Clause` | Included |
| [svg_fmt](https://crates.io/crates/svg_fmt/0.4.5) | 0.4.5 | `MIT/Apache-2.0` | Missing |
| [svgtypes](https://crates.io/crates/svgtypes/0.15.3) | 0.15.3 | `Apache-2.0 OR MIT` | Included |
| [syn](https://crates.io/crates/syn/1.0.109) | 1.0.109 | `MIT OR Apache-2.0` | Included |
| [syn](https://crates.io/crates/syn/2.0.119) | 2.0.119 | `MIT OR Apache-2.0` | Included |
| [syn](https://crates.io/crates/syn/3.0.9) | 3.0.9 | `MIT OR Apache-2.0` | Included |
| [sync_wrapper](https://crates.io/crates/sync_wrapper/1.0.2) | 1.0.2 | `Apache-2.0` | Included |
| [synstructure](https://crates.io/crates/synstructure/0.14.0) | 0.14.0 | `MIT` | Included |
| [taffy](https://crates.io/crates/taffy/0.9.0) | 0.9.0 | `MIT` | Missing |
| [take-until](https://crates.io/crates/take-until/0.2.0) | 0.2.0 | `MIT` | Included |
| [tempfile](https://crates.io/crates/tempfile/3.27.0) | 3.27.0 | `MIT OR Apache-2.0` | Included |
| [tendril](https://crates.io/crates/tendril/0.4.3) | 0.4.3 | `MIT/Apache-2.0` | Included |
| [termcolor](https://crates.io/crates/termcolor/1.4.1) | 1.4.1 | `Unlicense OR MIT` | Included |
| [thiserror](https://crates.io/crates/thiserror/1.0.69) | 1.0.69 | `MIT OR Apache-2.0` | Included |
| [thiserror](https://crates.io/crates/thiserror/2.0.21) | 2.0.21 | `MIT OR Apache-2.0` | Included |
| [thiserror-impl](https://crates.io/crates/thiserror-impl/1.0.69) | 1.0.69 | `MIT OR Apache-2.0` | Included |
| [thiserror-impl](https://crates.io/crates/thiserror-impl/2.0.21) | 2.0.21 | `MIT OR Apache-2.0` | Included |
| [tiff](https://crates.io/crates/tiff/0.11.3) | 0.11.3 | `MIT` | Included |
| [tiny-skia](https://crates.io/crates/tiny-skia/0.11.4) | 0.11.4 | `BSD-3-Clause` | Included |
| [tiny-skia-path](https://crates.io/crates/tiny-skia-path/0.11.4) | 0.11.4 | `BSD-3-Clause` | Included |
| [tinystr](https://crates.io/crates/tinystr/0.8.5) | 0.8.5 | `Unicode-3.0` | Included |
| [tinyvec](https://crates.io/crates/tinyvec/1.13.3) | 1.13.3 | `Zlib OR Apache-2.0 OR MIT` | Included |
| [tokio](https://crates.io/crates/tokio/1.54.1) | 1.54.1 | `MIT` | Included |
| [tokio-rustls](https://crates.io/crates/tokio-rustls/0.26.6) | 0.26.6 | `MIT OR Apache-2.0` | Included |
| [tokio-socks](https://crates.io/crates/tokio-socks/0.5.3) | 0.5.3 | `MIT` | Included |
| [tokio-util](https://crates.io/crates/tokio-util/0.7.20) | 0.7.20 | `MIT` | Included |
| [tower](https://crates.io/crates/tower/0.5.3) | 0.5.3 | `MIT` | Included |
| [tower-layer](https://crates.io/crates/tower-layer/0.3.3) | 0.3.3 | `MIT` | Included |
| [tower-service](https://crates.io/crates/tower-service/0.3.3) | 0.3.3 | `MIT` | Included |
| [tracing](https://crates.io/crates/tracing/0.1.44) | 0.1.44 | `MIT` | Included |
| [tracing-core](https://crates.io/crates/tracing-core/0.1.36) | 0.1.36 | `MIT` | Included |
| [try-lock](https://crates.io/crates/try-lock/0.2.5) | 0.2.5 | `MIT` | Included |
| [ttf-parser](https://crates.io/crates/ttf-parser/0.25.1) | 0.25.1 | `MIT OR Apache-2.0` | Included |
| [typeid](https://crates.io/crates/typeid/1.0.3) | 1.0.3 | `MIT OR Apache-2.0` | Included |
| [typenum](https://crates.io/crates/typenum/1.20.1) | 1.20.1 | `MIT OR Apache-2.0` | Included |
| [uluru](https://crates.io/crates/uluru/3.1.0) | 3.1.0 | `MPL-2.0` | Included |
| [unicase](https://crates.io/crates/unicase/2.10.0) | 2.10.0 | `MIT OR Apache-2.0` | Included |
| [unicode-bidi](https://crates.io/crates/unicode-bidi/0.3.18) | 0.3.18 | `MIT OR Apache-2.0` | Included |
| [unicode-bidi-mirroring](https://crates.io/crates/unicode-bidi-mirroring/0.4.0) | 0.4.0 | `MIT/Apache-2.0` | Included |
| [unicode-bom](https://crates.io/crates/unicode-bom/2.0.3) | 2.0.3 | `Apache-2.0` | Included |
| [unicode-ccc](https://crates.io/crates/unicode-ccc/0.4.0) | 0.4.0 | `MIT/Apache-2.0` | Included |
| [unicode-ident](https://crates.io/crates/unicode-ident/1.0.27) | 1.0.27 | `(MIT OR Apache-2.0) AND Unicode-3.0` | Included |
| [unicode-normalization](https://crates.io/crates/unicode-normalization/0.1.25) | 0.1.25 | `MIT OR Apache-2.0` | Included |
| [unicode-properties](https://crates.io/crates/unicode-properties/0.1.4) | 0.1.4 | `MIT/Apache-2.0` | Included |
| [unicode-script](https://crates.io/crates/unicode-script/0.5.8) | 0.5.8 | `MIT OR Apache-2.0` | Included |
| [unicode-vo](https://crates.io/crates/unicode-vo/0.1.0) | 0.1.0 | `MIT/Apache-2.0` | Included |
| [unicode-width](https://crates.io/crates/unicode-width/0.2.2) | 0.2.2 | `MIT OR Apache-2.0` | Included |
| [untrusted](https://crates.io/crates/untrusted/0.9.0) | 0.9.0 | `ISC` | Included |
| [url](https://crates.io/crates/url/2.5.8) | 2.5.8 | `MIT OR Apache-2.0` | Included |
| [usvg](https://crates.io/crates/usvg/0.45.1) | 0.45.1 | `Apache-2.0 OR MIT` | Included |
| [utf-8](https://crates.io/crates/utf-8/0.7.6) | 0.7.6 | `MIT OR Apache-2.0` | Included |
| [utf8_iter](https://crates.io/crates/utf8_iter/1.0.4) | 1.0.4 | `Apache-2.0 OR MIT` | Included |
| [uuid](https://crates.io/crates/uuid/1.26.0) | 1.26.0 | `Apache-2.0 OR MIT` | Included |
| [v_frame](https://crates.io/crates/v_frame/0.3.9) | 0.3.9 | `BSD-2-Clause` | Included |
| [value-bag](https://crates.io/crates/value-bag/1.14.1) | 1.14.1 | `Apache-2.0 OR MIT` | Included |
| [value-bag-serde1](https://crates.io/crates/value-bag-serde1/1.14.1) | 1.14.1 | `Apache-2.0 OR MIT` | Included |
| [waker-fn](https://crates.io/crates/waker-fn/1.2.0) | 1.2.0 | `Apache-2.0 OR MIT` | Included |
| [walkdir](https://crates.io/crates/walkdir/2.5.0) | 2.5.0 | `Unlicense/MIT` | Included |
| [want](https://crates.io/crates/want/0.3.2) | 0.3.2 | `MIT` | Included |
| [weezl](https://crates.io/crates/weezl/0.1.12) | 0.1.12 | `MIT OR Apache-2.0` | Included |
| [which](https://crates.io/crates/which/6.0.3) | 6.0.3 | `MIT` | Included |
| [winapi](https://crates.io/crates/winapi/0.3.9) | 0.3.9 | `MIT/Apache-2.0` | Included |
| [winapi-util](https://crates.io/crates/winapi-util/0.1.11) | 0.1.11 | `Unlicense OR MIT` | Included |
| [windows](https://crates.io/crates/windows/0.61.3) | 0.61.3 | `MIT OR Apache-2.0` | Included |
| [windows-collections](https://crates.io/crates/windows-collections/0.2.0) | 0.2.0 | `MIT OR Apache-2.0` | Included |
| [windows-core](https://crates.io/crates/windows-core/0.61.2) | 0.61.2 | `MIT OR Apache-2.0` | Included |
| [windows-future](https://crates.io/crates/windows-future/0.2.1) | 0.2.1 | `MIT OR Apache-2.0` | Included |
| [windows-implement](https://crates.io/crates/windows-implement/0.60.2) | 0.60.2 | `MIT OR Apache-2.0` | Included |
| [windows-interface](https://crates.io/crates/windows-interface/0.59.3) | 0.59.3 | `MIT OR Apache-2.0` | Included |
| [windows-link](https://crates.io/crates/windows-link/0.1.3) | 0.1.3 | `MIT OR Apache-2.0` | Included |
| [windows-link](https://crates.io/crates/windows-link/0.2.1) | 0.2.1 | `MIT OR Apache-2.0` | Included |
| [windows-numerics](https://crates.io/crates/windows-numerics/0.2.0) | 0.2.0 | `MIT OR Apache-2.0` | Included |
| [windows-registry](https://crates.io/crates/windows-registry/0.4.0) | 0.4.0 | `MIT OR Apache-2.0` | Included |
| [windows-registry](https://crates.io/crates/windows-registry/0.5.3) | 0.5.3 | `MIT OR Apache-2.0` | Included |
| [windows-result](https://crates.io/crates/windows-result/0.3.4) | 0.3.4 | `MIT OR Apache-2.0` | Included |
| [windows-strings](https://crates.io/crates/windows-strings/0.3.1) | 0.3.1 | `MIT OR Apache-2.0` | Included |
| [windows-strings](https://crates.io/crates/windows-strings/0.4.2) | 0.4.2 | `MIT OR Apache-2.0` | Included |
| [windows-sys](https://crates.io/crates/windows-sys/0.61.2) | 0.61.2 | `MIT OR Apache-2.0` | Included |
| [windows-targets](https://crates.io/crates/windows-targets/0.53.5) | 0.53.5 | `MIT OR Apache-2.0` | Included |
| [windows-threading](https://crates.io/crates/windows-threading/0.1.0) | 0.1.0 | `MIT OR Apache-2.0` | Included |
| [windows_x86_64_msvc](https://crates.io/crates/windows_x86_64_msvc/0.53.1) | 0.53.1 | `MIT OR Apache-2.0` | Included |
| [winsafe](https://crates.io/crates/winsafe/0.0.19) | 0.0.19 | `MIT` | Included |
| [writeable](https://crates.io/crates/writeable/0.6.4) | 0.6.4 | `Unicode-3.0` | Included |
| [xmlwriter](https://crates.io/crates/xmlwriter/0.1.0) | 0.1.0 | `MIT` | Included |
| [y4m](https://crates.io/crates/y4m/0.8.0) | 0.8.0 | `MIT` | Included |
| [yoke](https://crates.io/crates/yoke/0.8.4) | 0.8.4 | `Unicode-3.0` | Included |
| [yoke-derive](https://crates.io/crates/yoke-derive/0.8.4) | 0.8.4 | `Unicode-3.0` | Included |
| [zed-async-tar](https://crates.io/crates/zed-async-tar/0.5.0-zed) | 0.5.0-zed | `MIT/Apache-2.0` | Included |
| [zed-reqwest](https://crates.io/crates/zed-reqwest/0.12.15-zed) | 0.12.15-zed | `MIT OR Apache-2.0` | Included |
| [zerocopy](https://crates.io/crates/zerocopy/0.8.56) | 0.8.56 | `BSD-2-Clause OR Apache-2.0 OR MIT` | Included |
| [zerocopy-derive](https://crates.io/crates/zerocopy-derive/0.8.56) | 0.8.56 | `BSD-2-Clause OR Apache-2.0 OR MIT` | Included |
| [zerofrom](https://crates.io/crates/zerofrom/0.1.8) | 0.1.8 | `Unicode-3.0` | Included |
| [zerofrom-derive](https://crates.io/crates/zerofrom-derive/0.1.8) | 0.1.8 | `Unicode-3.0` | Included |
| [zeroize](https://crates.io/crates/zeroize/1.9.1) | 1.9.1 | `Apache-2.0 OR MIT` | Included |
| [zerotrie](https://crates.io/crates/zerotrie/0.2.5) | 0.2.5 | `Unicode-3.0` | Included |
| [zerovec](https://crates.io/crates/zerovec/0.11.9) | 0.11.9 | `Unicode-3.0` | Included |
| [zerovec-derive](https://crates.io/crates/zerovec-derive/0.11.7) | 0.11.7 | `Unicode-3.0` | Included |
| [zlib-rs](https://crates.io/crates/zlib-rs/0.6.7) | 0.6.7 | `Zlib` | Included |
| [zmij](https://crates.io/crates/zmij/1.0.23) | 1.0.23 | `MIT` | Included |
| [zune-core](https://crates.io/crates/zune-core/0.5.3) | 0.5.3 | `MIT OR Apache-2.0 OR Zlib` | Included |
| [zune-inflate](https://crates.io/crates/zune-inflate/0.2.54) | 0.2.54 | `MIT OR Apache-2.0 OR Zlib` | Missing |
| [zune-jpeg](https://crates.io/crates/zune-jpeg/0.5.15) | 0.5.15 | `MIT OR Apache-2.0 OR Zlib` | Included |

## 로컬 패키지에 라이선스 원문이 없는 항목

다음 패키지는 패키지 메타데이터에 라이선스가 표기되어 있으나 내려받은 소스에 독립적인 라이선스 파일이 없다. 배포 전 각 패키지의 원저장소에서 원문과 저작권 고지를 확인해야 한다.

| 패키지 | 버전 |
|---|---:|
| [blade-graphics](https://crates.io/crates/blade-graphics/0.7.1) | 0.7.1 |
| [blade-macros](https://crates.io/crates/blade-macros/0.3.0) | 0.3.0 |
| [blade-util](https://crates.io/crates/blade-util/0.3.0) | 0.3.0 |
| [convert_case](https://crates.io/crates/convert_case/0.4.0) | 0.4.0 |
| [dtor](https://crates.io/crates/dtor/0.0.6) | 0.0.6 |
| [dtor-proc-macro](https://crates.io/crates/dtor-proc-macro/0.0.5) | 0.0.5 |
| [gpu-alloc](https://crates.io/crates/gpu-alloc/0.6.2) | 0.6.2 |
| [gpu-alloc-ash](https://crates.io/crates/gpu-alloc-ash/0.7.1) | 0.7.1 |
| [gpu-alloc-types](https://crates.io/crates/gpu-alloc-types/0.3.1) | 0.3.1 |
| [hexf-parse](https://crates.io/crates/hexf-parse/0.2.1) | 0.2.1 |
| [lyon](https://crates.io/crates/lyon/1.0.19) | 1.0.19 |
| [lyon_algorithms](https://crates.io/crates/lyon_algorithms/1.0.21) | 1.0.21 |
| [lyon_geom](https://crates.io/crates/lyon_geom/1.0.19) | 1.0.19 |
| [lyon_path](https://crates.io/crates/lyon_path/1.0.19) | 1.0.19 |
| [lyon_tessellation](https://crates.io/crates/lyon_tessellation/1.0.21) | 1.0.21 |
| [mac](https://crates.io/crates/mac/0.1.1) | 0.1.1 |
| [naga](https://crates.io/crates/naga/25.0.1) | 25.0.1 |
| [profiling](https://crates.io/crates/profiling/1.0.18) | 1.0.18 |
| [profiling-procmacros](https://crates.io/crates/profiling-procmacros/1.0.18) | 1.0.18 |
| [pulp-wasm-simd-flag](https://crates.io/crates/pulp-wasm-simd-flag/0.1.1) | 0.1.1 |
| [seahash](https://crates.io/crates/seahash/4.1.0) | 4.1.0 |
| [simd_helpers](https://crates.io/crates/simd_helpers/0.1.0) | 0.1.0 |
| [spirv](https://crates.io/crates/spirv/0.3.0+sdk-1.3.268.0) | 0.3.0+sdk-1.3.268.0 |
| [stacksafe](https://crates.io/crates/stacksafe/0.1.4) | 0.1.4 |
| [stacksafe-macro](https://crates.io/crates/stacksafe-macro/0.1.4) | 0.1.4 |
| [svg_fmt](https://crates.io/crates/svg_fmt/0.4.5) | 0.4.5 |
| [taffy](https://crates.io/crates/taffy/0.9.0) | 0.9.0 |
| [zune-inflate](https://crates.io/crates/zune-inflate/0.2.54) | 0.2.54 |


## 프로젝트 자체의 라이선스

이 저장소에는 현재 자체 `LICENSE` 파일이나 `Cargo.toml`의 `license` 선언이 없다. 프로젝트 소스의 재배포 조건은 별도로 정해야 한다.
