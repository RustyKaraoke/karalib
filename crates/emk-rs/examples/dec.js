const fs = require("fs");
const zlib = require("zlib");

const args = process.argv.slice(2);
if (args.length < 1) {
	console.error("Usage: node emkunpack.js <file>");
	process.exit(1);
}

const data = fs.readFileSync(args[0]);

const xorKey = Buffer.from("AFF24C9CE9EA9943", "hex");
for (let i = 0; i < data.length; i++) {
	data[i] ^= xorKey[i % xorKey.length];
}

const magic = Buffer.from("2e53464453", "hex");
if (!data.slice(0, magic.length).equals(magic)) {
	console.error("Invalid magic");
	process.exit(1);
}

// Node.js doesn't support 64-bit offsets, so we assume the file is small enough
const headerPos = Number(data.readBigUInt64LE(0x22));
const headerEnd = Number(data.readBigUInt64LE(0x2a));

const header = data.slice(headerPos, headerEnd);
{
	let off = 0;
	const skipBytes = (n) => (off += n);
	const readByte = () => header[off++];
	const readUShort = () => {
		const v = header.readUInt16LE(off);
		off += 2;
		return v;
	};
	const readUInt = () => {
		const v = header.readUInt32LE(off);
		off += 4;
		return v;
	};
	const readString = () => {
		const len = readByte();
		const str = header.slice(off, off + len).toString("utf8");
		off += len;
		return str;
	};
	const checkMagic = (magic) => {
		const data = header.slice(off, off + magic.length);
		if (!data.equals(magic)) {
			throw new Error("Invalid magic: " + data.toString("hex") + " != " + magic.toString("hex"));
		}
		off += magic.length;
	};

	const readTag = () => {
		const tag = readByte();
		switch (tag) {
			case 2: {
				const v = readByte();
				console.log("BYTE: " + v);
				return v;
			}
			case 3: {
				const v = readUShort();
				console.log("USHORT: " + v);
				return v;
			}
			case 4: {
				const v = readUInt();
				console.log("UINT: " + v);
				return v;
			}
			case 6: {
				const v = readString();
				console.log("STRING: " + v);
				return v;
			}
			default:
				throw new Error("Unknown tag: 0x" + tag.toString(16));
		}
	}

	const magic = Buffer.from("53464453", "hex"); // SFDS
	while (off < header.length) {
		console.log("---------------------------");
		checkMagic(magic);
		const tag = readTag();
		const uncompressedSize = readTag();
		const unk2 = readTag();
		const dataBegin = readTag();
		const dataEnd = readTag();
		const unk5 = readTag(); // this might be "whether the data is compressed" flag, but every file I've seen has it set to 1
		const unk6 = readTag();
		skipBytes(0x10); // no idea what this is, possibly MD-5 hash?
		const unk7 = readTag();
		const unk8 = readTag();

		// the data is deflate compressed, with zlib header
		const compressedData = data.slice(dataBegin, dataEnd);

		const rawData = zlib.inflateSync(compressedData, { finishFlush: zlib.constants.Z_SYNC_FLUSH });
		if (rawData.length !== uncompressedSize) {
			throw new Error("Invalid uncompressed size");
		}

		switch (tag) {
			case "HEADER": {
				console.log("--- HEADER ---");
				console.log(rawData.toString("utf8"));
				console.log("--- END HEADER ---");
				break;
			}
		}

		const ext = {
			"HEADER": "txt",
			"MIDI_DATA": "mid",
			"LYRIC_DATA": "txt",
			"CURSOR_DATA": "bin",
		};

		const filename = tag + "." + (ext[tag] || "bin");
		fs.writeFileSync(filename, rawData);
	}
}