# Steam Clip Exporter

Exports your Steam clips to .mp4 without using the Steam GUI. Should be platform agnostic.

## Requirements

ffmpeg

## Usage

`steamclipexporter -d <directory of clips>`

You can specify an output directory using -o. All clips will end up there:

`steamclipexporter -d <directory of clips> -o <output directory>`

## Developing

`cargo run -- -d <directory> -o <output dir>`

## Contributing

Please feel free to open CRs or make improvements in any way.