
# The EMK format

the EMK format is a proprietary format used by Extreme Karaoke. It is a container format that contains the 3 NCN files (MIDI, lyrics, and CUR) and some metadata.

## Encryption

EMK files are usually XOR encrypted with a key of `AFF24C9CE9EA9943`. Which then produces a zlib compressed stream.

You may be able to find the key using an XOR cracker, but emk-rs provides a brute-force method to find the key which may be computationally expensive.

Try finding keys using [this tool](https://wiremask.eu/tools/xor-cracker/).

## The Data

When extracted, the EMK file contains the header list from `0x22`-`0x2A` to `0x2A`-`0x32` (u64) is the start and end of the header list. There are various headers in the header list, and each data type has a magic value prefix that is used to identify the data type.

The header type prefixes are as follows:

- `0x02` - A single byte, the next byte is the data
- `0x03` - A "short" (16-bit) integer, the next 2 bytes are the data
- `0x04` - A regular (32-bit) integer, the next 4 bytes are the data
- `0x06` - A string, the next byte is the length of the string, and the next N bytes are the string data.


Next are the actual headers. Headers are a collection of data that are stored inside the header list. The header data is in the following order:

- Tag - A string that identifies the header.
  There are 4 known types of headers:
    - `HEADER` - EMK metadata, contains the version and signature of the file.
    - `SONG_INFO` - Song metadata, contains various information about the song.
    - `MIDI_DATA` - MIDI data, contains the MIDI file.
    - `LYRIC_DATA` - Lyrics data, contains the lyrics file.
    - `CUR_DATA` - CUR data, contains the standard NCN CUR file.
- Uncompressed size - The size of the data when uncompressed.
- Start of compressed data - The offset to the start of the compressed data in the decoded EMK file.
- End of compressed data - The offset to the end of the compressed data in the decoded EMK file.
- Unknown - Unknown data, usually 0x01.
- Unknown - Unknown data, usually 0x00.
- MD5 hash - 16-bit MD5 hash of the data.
- Unknown - Unknown data, Usually contains an empty string.
- Unknown - Unknown data, usually 0x00.

More data is needed to fully understand the unknown data fields in the header.

The header ends when another header tag is found. The header list ends when the offset of the header list is reached.

To get the data from the header, get the start and end of the compressed data offsets, and then decompress the data using those offsets using zlib.

The output will be a normal NCN file, with some extra metadata
