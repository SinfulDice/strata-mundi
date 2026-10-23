"""Writes the application icon: a 16x16 pixel-art block of strata, upscaled to 256 px."""
import struct
import sys
import zlib

LAYERS = [
    (0, 3, [(0x6a, 0xb0, 0x4c), (0x5c, 0x9e, 0x40)]),
    (3, 8, [(0x8a, 0x5e, 0x3c), (0x7a, 0x50, 0x32)]),
    (8, 13, [(0x8e, 0x8e, 0x8e), (0x7c, 0x7c, 0x7c)]),
    (13, 16, [(0x4e, 0x4a, 0x56), (0x44, 0x40, 0x4c)]),
]


def pixel(x, y):
    for start, end, colors in LAYERS:
        if start <= y < end:
            return colors[(x * 7 + y * 13 + (x * y) % 5) % 3 == 0]
    raise ValueError(y)


def png(size, scale):
    rows = b""
    for y in range(size * scale):
        row = b"\x00"
        for x in range(size * scale):
            row += bytes(pixel(x // scale, y // scale)) + b"\xff"
        rows += row

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", size * scale, size * scale, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


if __name__ == "__main__":
    with open(sys.argv[1], "wb") as out:
        out.write(png(16, 16))
