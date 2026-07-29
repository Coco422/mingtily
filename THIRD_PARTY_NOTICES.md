# Third-Party Notices

Mingtily is distributed under the MIT License, but included libraries, downloadable model weights, and converted artifacts may use different licenses. This document is an attribution summary, not a replacement for the license files or model cards supplied by each upstream project.

## Meetily

- Source: [Zackriya-Solutions/meetily](https://github.com/Zackriya-Solutions/meetily)
- License: MIT
- Copyright: Copyright (c) 2024 Zackriya Solutions

Mingtily is an independent community fork. The original MIT notice is preserved in `LICENSE.md`.

## sherpa-onnx

- Source: [k2-fsa/sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)
- License: Apache License 2.0
- Use in Mingtily: offline speech recognition and speaker diarization runtime

## SenseVoice Small int8

- Base model: [FunAudioLLM/SenseVoiceSmall](https://huggingface.co/FunAudioLLM/SenseVoiceSmall)
- ONNX distribution: `sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09.tar.bz2` from the sherpa-onnx `asr-models` release
- License: FunASR Model Open Source License Agreement 1.1

Mingtily downloads the model only after user action, verifies the archive and installed files with built-in SHA256 values, and stores the model license with the installed asset.

## Chinese and English punctuation restoration int8

- Base model: [iic/punc_ct-transformer_zh-cn-common-vocab272727-pytorch](https://modelscope.cn/models/iic/punc_ct-transformer_zh-cn-common-vocab272727-pytorch)
- ONNX distribution: `sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12-int8.tar.bz2` from the sherpa-onnx `punctuation-models` release
- License: Apache License 2.0, according to the upstream model metadata

Mingtily downloads this optional model only after user action, verifies both the archive and installed ONNX file with built-in SHA256 values, and stores the Apache License with the installed asset. Punctuation inference remains local and falls back to the original ASR text when the model is unavailable.

## Paraformer Small int8

- ONNX repository: [csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09](https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09)
- Pinned revision: `63ddc3cd0f2810b68289a7b3876e62ef5d53d6df`
- Upstream model identified by the conversion repository: `crazyant/speech_paraformer_asr_nat-zh-cn-16k-common-vocab8358-onnx`
- License included by Mingtily for the model family: FunASR Model Open Source License Agreement 1.1

Mingtily downloads only the pinned model and token files after user action and verifies both files with built-in SHA256 values.

## Paraformer Streaming zh/en int8

- ONNX repository: [csukuangfj/sherpa-onnx-streaming-paraformer-bilingual-zh-en](https://huggingface.co/csukuangfj/sherpa-onnx-streaming-paraformer-bilingual-zh-en)
- Pinned revision: `8e40c43232a1c5c66c82111efc5820d3accca11b`
- Upstream model identified by the conversion repository: `damo/speech_paraformer_asr_nat-zh-cn-16k-common-vocab8404-online`
- License: Apache License 2.0, according to the pinned model card

Mingtily downloads the int8 encoder, int8 decoder, and token file only after user action and verifies all three files with built-in SHA256 values. Live inference remains local. Provisional hypotheses are display-only and are not stored as meeting transcripts.

## Qwen3-ASR 0.6B int8

- Base model: [Qwen/Qwen3-ASR-0.6B](https://huggingface.co/Qwen/Qwen3-ASR-0.6B)
- ONNX distribution: `sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25.tar.bz2` from the sherpa-onnx `asr-models` release
- Base-model revision inspected for attribution: `5eb144179a02acc5e5ba31e748d22b0cf3e303b0`
- License: Apache License 2.0, according to the base model card

Mingtily treats this as a Beta model because its download and installed sizes are substantially larger than the other local ASR choices. The archive and all installed inference files are verified with built-in SHA256 values.

## Pyannote segmentation 3.0 model

- Distribution used by Mingtily: `sherpa-onnx-pyannote-segmentation-3-0/model.int8.onnx`
- Source package: sherpa-onnx speaker segmentation model release
- License: MIT, as distributed with the model package

The application stores the model's accompanying license and source notice in the installed speaker model directory.

## 3D-Speaker ERes2Net embedding model

- Source: [modelscope/3D-Speaker](https://github.com/modelscope/3D-Speaker)
- Artifact: `3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx`
- License: Apache License 2.0

The application stores the model's accompanying license and source notice in the installed speaker model directory.

## NVIDIA Parakeet TDT 0.6B v3 ONNX

- ONNX repository: [istupakov/parakeet-tdt-0.6b-v3-onnx](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx)
- Pinned revision: `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce`
- Base model: NVIDIA Parakeet TDT 0.6B v3
- License: Creative Commons Attribution 4.0 International (CC BY 4.0), according to the model card

Mingtily downloads this model only after user action and verifies the expected files with built-in SHA256 values.

## whisper.cpp and Whisper model weights

- Runtime/model distribution: [ggerganov/whisper.cpp](https://github.com/ggerganov/whisper.cpp)
- Upstream model: [openai/whisper](https://github.com/openai/whisper)
- License: MIT; consult the selected model artifact and upstream repository for the complete notice

## FFmpeg sidecar

- Upstream project: [FFmpeg](https://ffmpeg.org/)
- Build-time binary mirror currently used by Mingtily: [Zackriya-Solutions/ffmpeg-binaries](https://github.com/Zackriya-Solutions/ffmpeg-binaries)
- Use in Mingtily: decoding, conversion, mixing, and packaged desktop sidecar

FFmpeg licensing depends on how a binary was configured and which optional libraries it includes. Redistributors must inspect the exact bundled binary and comply with its accompanying license and source-code obligations. Mingtily's roadmap tracks replacing the current mirror with a pinned, integrity-verified, reproducible source.

## Built-in summary models

Mingtily can optionally download one of the following model families after user action:

- Qwen3.5 GGUF conversions from `unsloth/Qwen3.5-2B-GGUF` and `unsloth/Qwen3.5-4B-GGUF`: Apache License 2.0 according to their model cards.
- Gemma 3 GGUF conversions from `bartowski/google_gemma-3-1b-it-GGUF` and `bartowski/google_gemma-3-4b-it-GGUF`: governed by the Gemma Terms of Use and the applicable model card, not the Mingtily MIT License.

Users and redistributors are responsible for reviewing the current upstream terms before downloading, redistributing, or deploying model weights.
