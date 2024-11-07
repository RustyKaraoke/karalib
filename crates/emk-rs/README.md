# emk-rs

This is a library to parse EMK karaoke archives.

> EMK (or SFDS) is a proprietary format used by a karaoke player called Extreme Karaoke, which is popular in Thailand.
>
> This pure-Rust implementation can parse the EMK format and extract the MIDI, lyrics, and NCN cursor files from the archive. It can also attempt to
> crack the XOR encryption key used to encrypt the EMK file, if the key is not known. The file itself is usually encrypted due to DRM. Think of this as the
> libdvdcss/libaacs of obscure karaoke formats. I hope they don't sue me. 
>
> See a video explaining the format [here](https://youtu.be/IK2L_j0kUWw).

> **Note**: This library is not affiliated with Extreme Karaoke or any of its affiliates. This is a reverse-engineered implementation of the EMK archive format.


See [the EMK format specification](emk-spec.md) for more information.