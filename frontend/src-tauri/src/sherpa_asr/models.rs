use crate::model_assets::{
    self, ArchiveFormat, ArchiveSourceSpec, DirectDownloadFileSpec, DirectSourceSpec,
    LicenseFileSpec, ModelFileSpec, ModelInstallSource, ModelInstallSpec,
};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};

pub const PROVIDER_ID: &str = "sherpa-onnx";
pub const SENSEVOICE_MODEL_ID: &str = "sensevoice-small-int8";
pub const QWEN3_ASR_MODEL_ID: &str = "qwen3-asr-0.6b-int8";
pub const FUNASR_NANO_MODEL_ID: &str = "funasr-nano-int8";
pub const PARAFORMER_SMALL_MODEL_ID: &str = "paraformer-zh-small-int8";
pub const PARAFORMER_ONLINE_MODEL_ID: &str = "paraformer-online-zh-en-int8";

const FUNASR_LICENSE: &str = include_str!("../../resources/licenses/FUNASR_MODEL_LICENSE.txt");
const APACHE_LICENSE: &str = include_str!("../../resources/licenses/APACHE-2.0.txt");

static FUNASR_LICENSE_FILES: &[LicenseFileSpec] = &[LicenseFileSpec {
    install_path: "licenses/FUNASR_MODEL_LICENSE.txt",
    contents: FUNASR_LICENSE,
}];

static APACHE_LICENSE_FILES: &[LicenseFileSpec] = &[LicenseFileSpec {
    install_path: "licenses/APACHE-2.0.txt",
    contents: APACHE_LICENSE,
}];

const SENSEVOICE_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09.tar.bz2";
const SENSEVOICE_ARCHIVE_SHA256: &str =
    "7305f7905bfcf77fa0b39388a313f3da35c68d971661a65475b56fb2162c8e63";
static SENSEVOICE_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        source_path: concat!(
            "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09",
            "/model.int8.onnx"
        ),
        install_path: "model.int8.onnx",
        size: 237_115_547,
        sha256: "12ca1a2ae7ecf3e0019ef2822307ee0b5cadc9196569e379b4c4026f8205276d",
    },
    ModelFileSpec {
        source_path: concat!(
            "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09",
            "/tokens.txt"
        ),
        install_path: "tokens.txt",
        size: 315_894,
        sha256: "f449eb28dc567533d7fa59be34e2abca8784f771850c78a47fb731a31429a1dc",
    },
];

#[cfg(test)]
const SENSEVOICE_MODELSCOPE_REVISION: &str = "c3662eb7f376d1a4fca69902934e2f1ce8d9d01a";
#[cfg(test)]
const SENSEVOICE_HUGGINGFACE_REVISION: &str = "355f4d4884d8afd08aef04b9007a8556d7b463b2";
static SENSEVOICE_MODELSCOPE_FILES: &[DirectDownloadFileSpec] = &[
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/aistoy/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09/repo?Revision=c3662eb7f376d1a4fca69902934e2f1ce8d9d01a&FilePath=model.int8.onnx",
        install_path: "model.int8.onnx",
        size: 237_115_547,
        sha256: "12ca1a2ae7ecf3e0019ef2822307ee0b5cadc9196569e379b4c4026f8205276d",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/aistoy/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09/repo?Revision=c3662eb7f376d1a4fca69902934e2f1ce8d9d01a&FilePath=tokens.txt",
        install_path: "tokens.txt",
        size: 315_894,
        sha256: "f449eb28dc567533d7fa59be34e2abca8784f771850c78a47fb731a31429a1dc",
    },
];
static SENSEVOICE_HUGGINGFACE_FILES: &[DirectDownloadFileSpec] = &[
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09/resolve/355f4d4884d8afd08aef04b9007a8556d7b463b2/model.int8.onnx",
        install_path: "model.int8.onnx",
        size: 237_115_547,
        sha256: "12ca1a2ae7ecf3e0019ef2822307ee0b5cadc9196569e379b4c4026f8205276d",
    },
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09/resolve/355f4d4884d8afd08aef04b9007a8556d7b463b2/tokens.txt",
        install_path: "tokens.txt",
        size: 315_894,
        sha256: "f449eb28dc567533d7fa59be34e2abca8784f771850c78a47fb731a31429a1dc",
    },
];
static SENSEVOICE_SOURCES: &[DirectSourceSpec] = &[
    DirectSourceSpec {
        label: "ModelScope China",
        download_size: 237_431_441,
        files: SENSEVOICE_MODELSCOPE_FILES,
    },
    DirectSourceSpec {
        label: "Hugging Face mirror",
        download_size: 237_431_441,
        files: SENSEVOICE_HUGGINGFACE_FILES,
    },
];
static SENSEVOICE_ARCHIVE_SOURCES: &[ArchiveSourceSpec] = &[ArchiveSourceSpec {
    label: "sherpa-onnx GitHub Release",
    url: SENSEVOICE_ARCHIVE_URL,
    sha256: SENSEVOICE_ARCHIVE_SHA256,
    download_size: 165_783_878,
    format: ArchiveFormat::TarBz2,
    files: SENSEVOICE_FILES,
}];

const QWEN3_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25.tar.bz2";
const QWEN3_ARCHIVE_SHA256: &str =
    "393f8a14e2f5fb96746aaab342997a40641001fbd5bf9592a080a8329178ee96";
#[cfg(test)]
const QWEN3_MODELSCOPE_REVISION: &str = "fb4dfc8713d81f913e21042e4d66e5b5cc27dc8a";
static QWEN3_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/conv_frontend.onnx",
        install_path: "conv_frontend.onnx",
        size: 44_148_281,
        sha256: "d22dc4423e0940e49884e903d2ea2f7e5567c14fc1aed97e4e26d6b8f208ef9e",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/encoder.int8.onnx",
        install_path: "encoder.int8.onnx",
        size: 182_491_662,
        sha256: "60748d3e6744a57c9c91e1b17424a6c2990567e8adceb0783940c03ed98fa9d9",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/decoder.int8.onnx",
        install_path: "decoder.int8.onnx",
        size: 755_914_231,
        sha256: "4f6885be5959ae26af3089d38ee7972c5fafbeeb1cf8d5e76eab6d8b61ca5771",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/tokenizer/merges.txt",
        install_path: "tokenizer/merges.txt",
        size: 1_671_853,
        sha256: "8831e4f1a044471340f7c0a83d7bd71306a5b867e95fd870f74d0c5308a904d5",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/tokenizer/tokenizer_config.json",
        install_path: "tokenizer/tokenizer_config.json",
        size: 12_487,
        sha256: "4942d005604266809309cabc9f4e9cb89ce855d59b14681fdc0e1cc62ea26c4c",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/tokenizer/vocab.json",
        install_path: "tokenizer/vocab.json",
        size: 2_776_833,
        sha256: "ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910",
    },
];
static QWEN3_MODELSCOPE_FILES: &[DirectDownloadFileSpec] = &[
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/jkman2023/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/repo?Revision=fb4dfc8713d81f913e21042e4d66e5b5cc27dc8a&FilePath=conv_frontend.onnx",
        install_path: "conv_frontend.onnx",
        size: 44_148_281,
        sha256: "d22dc4423e0940e49884e903d2ea2f7e5567c14fc1aed97e4e26d6b8f208ef9e",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/jkman2023/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/repo?Revision=fb4dfc8713d81f913e21042e4d66e5b5cc27dc8a&FilePath=encoder.int8.onnx",
        install_path: "encoder.int8.onnx",
        size: 182_491_662,
        sha256: "60748d3e6744a57c9c91e1b17424a6c2990567e8adceb0783940c03ed98fa9d9",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/jkman2023/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/repo?Revision=fb4dfc8713d81f913e21042e4d66e5b5cc27dc8a&FilePath=decoder.int8.onnx",
        install_path: "decoder.int8.onnx",
        size: 755_914_231,
        sha256: "4f6885be5959ae26af3089d38ee7972c5fafbeeb1cf8d5e76eab6d8b61ca5771",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/jkman2023/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/repo?Revision=fb4dfc8713d81f913e21042e4d66e5b5cc27dc8a&FilePath=tokenizer%2Fmerges.txt",
        install_path: "tokenizer/merges.txt",
        size: 1_671_853,
        sha256: "8831e4f1a044471340f7c0a83d7bd71306a5b867e95fd870f74d0c5308a904d5",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/jkman2023/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/repo?Revision=fb4dfc8713d81f913e21042e4d66e5b5cc27dc8a&FilePath=tokenizer%2Ftokenizer_config.json",
        install_path: "tokenizer/tokenizer_config.json",
        size: 12_487,
        sha256: "4942d005604266809309cabc9f4e9cb89ce855d59b14681fdc0e1cc62ea26c4c",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/jkman2023/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/repo?Revision=fb4dfc8713d81f913e21042e4d66e5b5cc27dc8a&FilePath=tokenizer%2Fvocab.json",
        install_path: "tokenizer/vocab.json",
        size: 2_776_833,
        sha256: "ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910",
    },
];
static QWEN3_DIRECT_SOURCES: &[DirectSourceSpec] = &[DirectSourceSpec {
    label: "ModelScope China",
    download_size: 987_015_347,
    files: QWEN3_MODELSCOPE_FILES,
}];
static QWEN3_ARCHIVE_SOURCES: &[ArchiveSourceSpec] = &[ArchiveSourceSpec {
    label: "sherpa-onnx GitHub Release",
    url: QWEN3_ARCHIVE_URL,
    sha256: QWEN3_ARCHIVE_SHA256,
    download_size: 878_702_423,
    format: ArchiveFormat::TarBz2,
    files: QWEN3_FILES,
}];

const FUNASR_NANO_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-funasr-nano-int8-2025-12-30.tar.bz2";
const FUNASR_NANO_ARCHIVE_SHA256: &str =
    "eb43d7ccc2e86b243f6a03b7df361033dda66db9523d1a92bf6aca2b50c9476b";
const FUNASR_NANO_MODELSCOPE_URL: &str = "https://www.modelscope.cn/api/v1/models/TonyWong/sherpa-onnx-sense-voice-funasr-nano-2512/repo?Revision=243e73293f95c5b28ebc3250022b8cc46d488dc0&FilePath=funasr-nano-int8.zip";
const FUNASR_NANO_MODELSCOPE_SHA256: &str =
    "651319890ccf75bc080574019bc748bb85167875b0cb7d3f19456280ad8f5d6e";
static FUNASR_NANO_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        source_path: "sherpa-onnx-funasr-nano-int8-2025-12-30/embedding.int8.onnx",
        install_path: "embedding.int8.onnx",
        size: 155_583_106,
        sha256: "a05d2816e284fcca29a5dccb2c14b9edeb638fd983a84cd4a447248889b6a408",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-funasr-nano-int8-2025-12-30/encoder_adaptor.int8.onnx",
        install_path: "encoder_adaptor.int8.onnx",
        size: 238_277_200,
        sha256: "d0246c823f2c34133ae0efee395d8a189c8f92643e3432f866939ee34d34492c",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-funasr-nano-int8-2025-12-30/llm.int8.onnx",
        install_path: "llm.int8.onnx",
        size: 600_339_316,
        sha256: "7f0c5a508b41474b1b1ec1cdbdefafd2cf8b3642c6915a0a425265b7b7d2c960",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-funasr-nano-int8-2025-12-30/Qwen3-0.6B/merges.txt",
        install_path: "Qwen3-0.6B/merges.txt",
        size: 1_671_853,
        sha256: "8831e4f1a044471340f7c0a83d7bd71306a5b867e95fd870f74d0c5308a904d5",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-funasr-nano-int8-2025-12-30/Qwen3-0.6B/tokenizer.json",
        install_path: "Qwen3-0.6B/tokenizer.json",
        size: 11_422_654,
        sha256: "aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4",
    },
    ModelFileSpec {
        source_path: "sherpa-onnx-funasr-nano-int8-2025-12-30/Qwen3-0.6B/vocab.json",
        install_path: "Qwen3-0.6B/vocab.json",
        size: 2_776_833,
        sha256: "ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910",
    },
];

static FUNASR_NANO_MODELSCOPE_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        source_path: "funasr-nano-int8/embedding.int8.onnx",
        install_path: "embedding.int8.onnx",
        size: 155_584_380,
        sha256: "95e61cd0c9c3b9543339a4cf973c95c116815e745ccc1e0285cbd81f76d18644",
    },
    ModelFileSpec {
        source_path: "funasr-nano-int8/encoder_adaptor.int8.onnx",
        install_path: "encoder_adaptor.int8.onnx",
        size: 237_792_748,
        sha256: "f36dea2e30fbc33b5db1d7a7265cc976c5e5586c77b042d5adb1ad27c72db422",
    },
    ModelFileSpec {
        source_path: "funasr-nano-int8/llm.int8.onnx",
        install_path: "llm.int8.onnx",
        size: 600_339_316,
        sha256: "7f0c5a508b41474b1b1ec1cdbdefafd2cf8b3642c6915a0a425265b7b7d2c960",
    },
    ModelFileSpec {
        source_path: "funasr-nano-int8/Qwen3-0.6B/merges.txt",
        install_path: "Qwen3-0.6B/merges.txt",
        size: 1_671_853,
        sha256: "8831e4f1a044471340f7c0a83d7bd71306a5b867e95fd870f74d0c5308a904d5",
    },
    ModelFileSpec {
        source_path: "funasr-nano-int8/Qwen3-0.6B/tokenizer.json",
        install_path: "Qwen3-0.6B/tokenizer.json",
        size: 11_422_654,
        sha256: "aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4",
    },
    ModelFileSpec {
        source_path: "funasr-nano-int8/Qwen3-0.6B/vocab.json",
        install_path: "Qwen3-0.6B/vocab.json",
        size: 2_776_833,
        sha256: "ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910",
    },
];

static FUNASR_NANO_SOURCES: &[ArchiveSourceSpec] = &[
    ArchiveSourceSpec {
        label: "ModelScope China",
        url: FUNASR_NANO_MODELSCOPE_URL,
        sha256: FUNASR_NANO_MODELSCOPE_SHA256,
        download_size: 745_309_529,
        format: ArchiveFormat::Zip,
        files: FUNASR_NANO_MODELSCOPE_FILES,
    },
    ArchiveSourceSpec {
        label: "sherpa-onnx GitHub Release",
        url: FUNASR_NANO_ARCHIVE_URL,
        sha256: FUNASR_NANO_ARCHIVE_SHA256,
        download_size: 841_730_611,
        format: ArchiveFormat::TarBz2,
        files: FUNASR_NANO_FILES,
    },
];

#[cfg(test)]
const PARAFORMER_MODELSCOPE_REVISION: &str = "766dd802fdd67371e41ccdb25b4fcaa6dc44392f";
#[cfg(test)]
const PARAFORMER_HUGGINGFACE_REVISION: &str = "63ddc3cd0f2810b68289a7b3876e62ef5d53d6df";
static PARAFORMER_MODELSCOPE_FILES: &[DirectDownloadFileSpec] = &[
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/pengzhendong/sherpa-onnx-paraformer-zh-small/repo?Revision=766dd802fdd67371e41ccdb25b4fcaa6dc44392f&FilePath=model.int8.onnx",
        install_path: "model.int8.onnx",
        size: 81_828_675,
        sha256: "3ef6c19369b912f7caf3cef8e545c5ccd1a33d9d7ec792a46668dc41c4b229ec",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/pengzhendong/sherpa-onnx-paraformer-zh-small/repo?Revision=766dd802fdd67371e41ccdb25b4fcaa6dc44392f&FilePath=tokens.txt",
        install_path: "tokens.txt",
        size: 75_352,
        sha256: "4b2d964e18b9cf139b473003b6698fb2ed9a2a5ec55b93daa677b28f578897aa",
    },
];
static PARAFORMER_HUGGINGFACE_FILES: &[DirectDownloadFileSpec] = &[
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09/resolve/63ddc3cd0f2810b68289a7b3876e62ef5d53d6df/model.int8.onnx",
        install_path: "model.int8.onnx",
        size: 81_828_675,
        sha256: "3ef6c19369b912f7caf3cef8e545c5ccd1a33d9d7ec792a46668dc41c4b229ec",
    },
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09/resolve/63ddc3cd0f2810b68289a7b3876e62ef5d53d6df/tokens.txt",
        install_path: "tokens.txt",
        size: 75_352,
        sha256: "4b2d964e18b9cf139b473003b6698fb2ed9a2a5ec55b93daa677b28f578897aa",
    },
];
static PARAFORMER_SOURCES: &[DirectSourceSpec] = &[
    DirectSourceSpec {
        label: "ModelScope China",
        download_size: 81_904_027,
        files: PARAFORMER_MODELSCOPE_FILES,
    },
    DirectSourceSpec {
        label: "Hugging Face mirror",
        download_size: 81_904_027,
        files: PARAFORMER_HUGGINGFACE_FILES,
    },
];

#[cfg(test)]
const PARAFORMER_ONLINE_MODELSCOPE_REVISION: &str = "c9d662eca7ca87765078148501659faa24e842ed";
#[cfg(test)]
const PARAFORMER_ONLINE_HUGGINGFACE_REVISION: &str = "8e40c43232a1c5c66c82111efc5820d3accca11b";
static PARAFORMER_ONLINE_MODELSCOPE_FILES: &[DirectDownloadFileSpec] = &[
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/pengzhendong/sherpa-onnx-streaming-paraformer-bilingual-zh-en/repo?Revision=c9d662eca7ca87765078148501659faa24e842ed&FilePath=encoder.int8.onnx",
        install_path: "encoder.int8.onnx",
        size: 165_462_184,
        sha256: "81a70226a8934e6ed92aa1d4fc486b428b5398e2f2619ed4897b7294cab90e9a",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/pengzhendong/sherpa-onnx-streaming-paraformer-bilingual-zh-en/repo?Revision=c9d662eca7ca87765078148501659faa24e842ed&FilePath=decoder.int8.onnx",
        install_path: "decoder.int8.onnx",
        size: 71_664_561,
        sha256: "f3cca9f77bb9d93c8fcbfb63ae617b6b1ee96818df3aa3b151c40658fe38594f",
    },
    DirectDownloadFileSpec {
        url: "https://www.modelscope.cn/api/v1/models/pengzhendong/sherpa-onnx-streaming-paraformer-bilingual-zh-en/repo?Revision=c9d662eca7ca87765078148501659faa24e842ed&FilePath=tokens.txt",
        install_path: "tokens.txt",
        size: 75_756,
        sha256: "59aba8873a2ed1e122c25fee421e25f283b63290efbde85c1f01a853d83cb6e6",
    },
];
static PARAFORMER_ONLINE_HUGGINGFACE_FILES: &[DirectDownloadFileSpec] = &[
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-streaming-paraformer-bilingual-zh-en/resolve/8e40c43232a1c5c66c82111efc5820d3accca11b/encoder.int8.onnx",
        install_path: "encoder.int8.onnx",
        size: 165_462_184,
        sha256: "81a70226a8934e6ed92aa1d4fc486b428b5398e2f2619ed4897b7294cab90e9a",
    },
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-streaming-paraformer-bilingual-zh-en/resolve/8e40c43232a1c5c66c82111efc5820d3accca11b/decoder.int8.onnx",
        install_path: "decoder.int8.onnx",
        size: 71_664_561,
        sha256: "f3cca9f77bb9d93c8fcbfb63ae617b6b1ee96818df3aa3b151c40658fe38594f",
    },
    DirectDownloadFileSpec {
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-streaming-paraformer-bilingual-zh-en/resolve/8e40c43232a1c5c66c82111efc5820d3accca11b/tokens.txt",
        install_path: "tokens.txt",
        size: 75_756,
        sha256: "59aba8873a2ed1e122c25fee421e25f283b63290efbde85c1f01a853d83cb6e6",
    },
];
static PARAFORMER_ONLINE_SOURCES: &[DirectSourceSpec] = &[
    DirectSourceSpec {
        label: "ModelScope China",
        download_size: 237_202_501,
        files: PARAFORMER_ONLINE_MODELSCOPE_FILES,
    },
    DirectSourceSpec {
        label: "Hugging Face mirror",
        download_size: 237_202_501,
        files: PARAFORMER_ONLINE_HUGGINGFACE_FILES,
    },
];

static SENSEVOICE_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: SENSEVOICE_MODEL_ID,
    provider: PROVIDER_ID,
    backend: "sense-voice",
    source: ModelInstallSource::HybridVariants {
        direct_sources: SENSEVOICE_SOURCES,
        archive_sources: SENSEVOICE_ARCHIVE_SOURCES,
    },
    download_size: 237_431_441,
    installed_size: 237_431_441,
    licenses: FUNASR_LICENSE_FILES,
};

static QWEN3_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: QWEN3_ASR_MODEL_ID,
    provider: PROVIDER_ID,
    backend: "qwen3-asr",
    source: ModelInstallSource::HybridVariants {
        direct_sources: QWEN3_DIRECT_SOURCES,
        archive_sources: QWEN3_ARCHIVE_SOURCES,
    },
    download_size: 878_702_423,
    installed_size: 987_015_347,
    licenses: APACHE_LICENSE_FILES,
};

static FUNASR_NANO_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: FUNASR_NANO_MODEL_ID,
    provider: PROVIDER_ID,
    backend: "funasr-nano",
    source: ModelInstallSource::ArchiveVariants {
        sources: FUNASR_NANO_SOURCES,
    },
    download_size: 745_309_529,
    installed_size: 1_009_587_784,
    licenses: APACHE_LICENSE_FILES,
};

static PARAFORMER_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: PARAFORMER_SMALL_MODEL_ID,
    provider: PROVIDER_ID,
    backend: "paraformer-offline",
    source: ModelInstallSource::DirectFileVariants {
        sources: PARAFORMER_SOURCES,
    },
    download_size: 81_904_027,
    installed_size: 81_904_027,
    licenses: FUNASR_LICENSE_FILES,
};

static PARAFORMER_ONLINE_SPEC: ModelInstallSpec = ModelInstallSpec {
    id: PARAFORMER_ONLINE_MODEL_ID,
    provider: PROVIDER_ID,
    backend: "paraformer-online",
    source: ModelInstallSource::DirectFileVariants {
        sources: PARAFORMER_ONLINE_SOURCES,
    },
    download_size: 237_202_501,
    installed_size: 237_202_501,
    licenses: APACHE_LICENSE_FILES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SherpaAsrBackend {
    SenseVoice,
    Qwen3Asr,
    FunAsrNano,
    ParaformerOffline,
    ParaformerOnline,
}

#[derive(Debug, Clone)]
pub struct InstalledSherpaModel {
    pub id: String,
    pub backend: SherpaAsrBackend,
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct SherpaAsrModelStatus {
    pub id: String,
    pub name: String,
    pub backend: String,
    pub status: String,
    pub download_size: u64,
    pub installed_size: u64,
    pub languages: Vec<String>,
    pub language_hint: String,
    pub streaming_mode: String,
    pub license: String,
    pub recommended: bool,
    pub beta: bool,
    pub path: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportedSherpaModel {
    pub model_id: String,
    pub path: String,
}

pub fn all_specs() -> [&'static ModelInstallSpec; 5] {
    [
        &SENSEVOICE_SPEC,
        &PARAFORMER_SPEC,
        &PARAFORMER_ONLINE_SPEC,
        &QWEN3_SPEC,
        &FUNASR_NANO_SPEC,
    ]
}

pub fn spec_for_model(model_id: &str) -> Option<&'static ModelInstallSpec> {
    all_specs().into_iter().find(|spec| spec.id == model_id)
}

pub fn model_root<R: Runtime>(app: &AppHandle<R>, model_id: &str) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .context("Unable to resolve app data directory")?
        .join("models")
        .join("asr")
        .join(PROVIDER_ID)
        .join(model_id))
}

pub fn installed_model<R: Runtime>(
    app: &AppHandle<R>,
    model_id: &str,
) -> Result<Option<InstalledSherpaModel>> {
    let spec = spec_for_model(model_id).ok_or_else(|| anyhow!("Unknown Sherpa ASR model"))?;
    let root = model_root(app, model_id)?;
    if model_assets::validate_installation(&root, spec).is_err() {
        return Ok(None);
    }
    Ok(Some(InstalledSherpaModel {
        id: model_id.to_string(),
        backend: backend_for_model(model_id)?,
        root,
    }))
}

pub fn list_status<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<SherpaAsrModelStatus>> {
    all_specs()
        .into_iter()
        .map(|spec| status_for_spec(app, spec))
        .collect()
}

pub async fn download_model<R: Runtime>(app: &AppHandle<R>, model_id: &str) -> Result<()> {
    let spec = spec_for_model(model_id).ok_or_else(|| anyhow!("Unknown Sherpa ASR model"))?;
    let root = model_root(app, model_id)?;
    model_assets::install_model(app, &root, spec, "sherpa-asr-model-download").await
}

pub async fn delete_model<R: Runtime>(app: &AppHandle<R>, model_id: &str) -> Result<()> {
    let _ = spec_for_model(model_id).ok_or_else(|| anyhow!("Unknown Sherpa ASR model"))?;
    model_assets::delete_model(&model_root(app, model_id)?).await
}

pub async fn import_archive<R: Runtime>(
    app: &AppHandle<R>,
    archive_path: &Path,
) -> Result<ImportedSherpaModel> {
    let path_for_match = archive_path.to_path_buf();
    let spec = tokio::task::spawn_blocking(move || {
        all_specs()
            .into_iter()
            .find_map(
                |spec| match model_assets::recognizes_archive(&path_for_match, spec) {
                    Ok(true) => Some(Ok(spec)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
            )
            .transpose()
    })
    .await
    .map_err(|error| anyhow!("Offline archive recognition failed: {error}"))??
    .ok_or_else(|| anyhow!("This archive does not match any supported Sherpa ONNX model"))?;
    let root = model_root(app, spec.id)?;
    model_assets::import_model_archive(&root, spec, archive_path).await?;
    log::info!(
        "Imported verified offline model archive: model={}, source={}",
        spec.id,
        archive_path.display()
    );
    Ok(ImportedSherpaModel {
        model_id: spec.id.to_string(),
        path: root.to_string_lossy().to_string(),
    })
}

pub async fn import_directory<R: Runtime>(
    app: &AppHandle<R>,
    source_root: &Path,
) -> Result<ImportedSherpaModel> {
    let path_for_match = source_root.to_path_buf();
    let spec = tokio::task::spawn_blocking(move || {
        all_specs()
            .into_iter()
            .find_map(
                |spec| match model_assets::recognizes_directory(&path_for_match, spec) {
                    Ok(true) => Some(Ok(spec)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
            )
            .transpose()
    })
    .await
    .map_err(|error| anyhow!("Offline model directory recognition failed: {error}"))??
    .ok_or_else(|| {
        anyhow!("No complete supported Sherpa ONNX model was found in this directory")
    })?;
    let root = model_root(app, spec.id)?;
    model_assets::import_model_directory(&root, spec, source_root).await?;
    log::info!(
        "Imported verified offline model directory: model={}, source={}",
        spec.id,
        source_root.display()
    );
    Ok(ImportedSherpaModel {
        model_id: spec.id.to_string(),
        path: root.to_string_lossy().to_string(),
    })
}

pub fn backend_for_model(model_id: &str) -> Result<SherpaAsrBackend> {
    match model_id {
        SENSEVOICE_MODEL_ID => Ok(SherpaAsrBackend::SenseVoice),
        QWEN3_ASR_MODEL_ID => Ok(SherpaAsrBackend::Qwen3Asr),
        FUNASR_NANO_MODEL_ID => Ok(SherpaAsrBackend::FunAsrNano),
        PARAFORMER_SMALL_MODEL_ID => Ok(SherpaAsrBackend::ParaformerOffline),
        PARAFORMER_ONLINE_MODEL_ID => Ok(SherpaAsrBackend::ParaformerOnline),
        _ => Err(anyhow!("Unknown Sherpa ASR model: {model_id}")),
    }
}

fn status_for_spec<R: Runtime>(
    app: &AppHandle<R>,
    spec: &'static ModelInstallSpec,
) -> Result<SherpaAsrModelStatus> {
    let root = model_root(app, spec.id)?;
    let validation = model_assets::validate_installation(&root, spec);
    let status = if validation.is_ok() {
        "available"
    } else if root.exists() {
        "corrupt"
    } else {
        "missing"
    };
    let (name, languages, language_hint, streaming_mode, license, recommended, beta) = match spec.id
    {
        SENSEVOICE_MODEL_ID => (
            "SenseVoice Small int8",
            vec!["zh", "yue", "en", "ja", "ko"],
            "auto-or-fixed",
            "vad-segmented",
            "FunASR Model License 1.1",
            true,
            false,
        ),
        PARAFORMER_SMALL_MODEL_ID => (
            "Paraformer Small int8",
            vec!["zh", "en"],
            "auto-only",
            "vad-segmented",
            "FunASR Model License 1.1",
            false,
            false,
        ),
        PARAFORMER_ONLINE_MODEL_ID => (
            "Paraformer Streaming zh/en int8",
            vec!["zh", "en"],
            "auto-only",
            "continuous",
            "Apache-2.0",
            false,
            true,
        ),
        QWEN3_ASR_MODEL_ID => (
            "Qwen3-ASR 0.6B int8",
            vec!["zh", "yue", "en", "ja", "ko", "de", "fr", "es", "pt", "ru"],
            "auto-or-fixed",
            "vad-segmented",
            "Apache-2.0",
            false,
            true,
        ),
        FUNASR_NANO_MODEL_ID => (
            "FunASR Nano int8",
            vec![
                "zh", "yue", "en", "ja", "ko", "vi", "id", "th", "ms", "tl", "ar", "hi",
            ],
            "auto-only",
            "vad-segmented",
            "Apache-2.0",
            false,
            true,
        ),
        _ => unreachable!(),
    };

    Ok(SherpaAsrModelStatus {
        id: spec.id.to_string(),
        name: name.to_string(),
        backend: spec.backend.to_string(),
        status: status.to_string(),
        download_size: spec.download_size,
        installed_size: spec.installed_size,
        languages: languages.into_iter().map(str::to_string).collect(),
        language_hint: language_hint.to_string(),
        streaming_mode: streaming_mode.to_string(),
        license: license.to_string(),
        recommended,
        beta,
        path: root.to_string_lossy().to_string(),
        error: validation.err().map(|error| error.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_unique_model_ids() {
        let specs = all_specs();
        let unique = specs
            .iter()
            .map(|spec| spec.id)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(specs.len(), 5);
        assert_eq!(unique.len(), specs.len());
    }

    #[test]
    fn paraformer_prefers_pinned_modelscope_and_keeps_fallback() {
        assert_eq!(PARAFORMER_MODELSCOPE_REVISION.len(), 40);
        assert_eq!(PARAFORMER_HUGGINGFACE_REVISION.len(), 40);
        assert_eq!(PARAFORMER_SOURCES[0].label, "ModelScope China");
        assert!(PARAFORMER_SOURCES[0]
            .files
            .iter()
            .all(|file| file.url.contains(PARAFORMER_MODELSCOPE_REVISION)));
        assert!(PARAFORMER_SOURCES[1]
            .files
            .iter()
            .all(|file| file.url.contains(PARAFORMER_HUGGINGFACE_REVISION)));
    }

    #[test]
    fn online_paraformer_prefers_pinned_modelscope_and_keeps_fallback() {
        assert_eq!(PARAFORMER_ONLINE_MODELSCOPE_REVISION.len(), 40);
        assert_eq!(PARAFORMER_ONLINE_HUGGINGFACE_REVISION.len(), 40);
        assert_eq!(PARAFORMER_ONLINE_SOURCES[0].label, "ModelScope China");
        assert!(PARAFORMER_ONLINE_SOURCES[0]
            .files
            .iter()
            .all(|file| file.url.contains(PARAFORMER_ONLINE_MODELSCOPE_REVISION)));
        assert!(PARAFORMER_ONLINE_SOURCES[1]
            .files
            .iter()
            .all(|file| file.url.contains(PARAFORMER_ONLINE_HUGGINGFACE_REVISION)));
    }

    #[test]
    fn qwen3_asr_prefers_pinned_modelscope_and_keeps_archive_fallback() {
        assert_eq!(QWEN3_MODELSCOPE_REVISION.len(), 40);
        assert_eq!(QWEN3_DIRECT_SOURCES[0].label, "ModelScope China");
        assert!(QWEN3_DIRECT_SOURCES[0]
            .files
            .iter()
            .all(|file| file.url.contains(QWEN3_MODELSCOPE_REVISION)));
        assert_eq!(QWEN3_ARCHIVE_SOURCES[0].sha256, QWEN3_ARCHIVE_SHA256);
    }

    #[test]
    fn sensevoice_prefers_pinned_modelscope_and_keeps_fallbacks() {
        let ModelInstallSource::HybridVariants {
            direct_sources,
            archive_sources,
        } = SENSEVOICE_SPEC.source
        else {
            panic!("SenseVoice should use verified hybrid sources");
        };
        assert_eq!(direct_sources[0].label, "ModelScope China");
        assert!(direct_sources[0]
            .files
            .iter()
            .all(|file| file.url.contains(SENSEVOICE_MODELSCOPE_REVISION)));
        assert!(direct_sources[1]
            .files
            .iter()
            .all(|file| file.url.contains(SENSEVOICE_HUGGINGFACE_REVISION)));
        assert_eq!(archive_sources[0].sha256, SENSEVOICE_ARCHIVE_SHA256);
    }

    #[test]
    fn funasr_nano_artifact_metadata_is_complete() {
        assert_eq!(FUNASR_NANO_FILES.len(), 6);
        assert_eq!(FUNASR_NANO_MODELSCOPE_FILES.len(), 6);
        assert_eq!(FUNASR_NANO_ARCHIVE_SHA256.len(), 64);
        assert_eq!(FUNASR_NANO_MODELSCOPE_SHA256.len(), 64);
        assert_eq!(FUNASR_NANO_SPEC.download_size, 745_309_529);
        assert_eq!(FUNASR_NANO_SPEC.installed_size, 1_009_587_784);
        assert_eq!(FUNASR_NANO_SOURCES[0].label, "ModelScope China");
        assert_eq!(FUNASR_NANO_SOURCES[0].format, ArchiveFormat::Zip);
        assert_eq!(FUNASR_NANO_SOURCES[1].format, ArchiveFormat::TarBz2);
    }

    #[tokio::test]
    #[ignore = "requires the audited ModelScope FunASR Nano archive"]
    async fn modelscope_funasr_archive_imports_and_validates() {
        let archive = std::env::var("MINGTILY_FUNASR_ARCHIVE")
            .expect("set MINGTILY_FUNASR_ARCHIVE to funasr-nano-int8.zip");
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join(FUNASR_NANO_MODEL_ID);
        model_assets::import_model_archive(&root, &FUNASR_NANO_SPEC, Path::new(&archive))
            .await
            .unwrap();
        model_assets::validate_installation(&root, &FUNASR_NANO_SPEC).unwrap();
    }
}
