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
- Use in Mingtily: offline speaker diarization runtime

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
