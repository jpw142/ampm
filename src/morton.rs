use bevy::math::UVec3;

fn spread(mut w: u64) -> u64 {
    w &= 0x00000000001fffff;
    w = (w | w << 32) & 0x001f00000000ffff;
    w = (w | w << 16) & 0x001f0000ff0000ff;
    w = (w | w << 8) & 0x010f00f00f00f00f;
    w = (w | w << 4) & 0x10c30c30c30c30c3;
    w = (w | w << 2) & 0x1249249249249249;
    w
}

// This spreads the indexes and then interleaves them 
// X00X00X
// 0Y00Y00Y
// 00Z00Z00Z
// XYZXYZXYZ
pub fn morton_encode3(x: u32, y: u32, z: u32) -> u64 {
    spread(x as u64) | (spread(y as u64) << 1) | (spread(z as u64) << 2)
}

// Decoding

fn compact(mut w: u64) -> u32 {
    w &= 0x1249249249249249;
    w = (w ^ (w >> 2)) & 0x30c30c30c30c30c3;
    w = (w ^ (w >> 4)) & 0xf00f00f00f00f00f;
    w = (w ^ (w >> 8)) & 0x00ff0000ff0000ff;
    w = (w ^ (w >> 16)) & 0x00ff00000000ffff;
    w = (w ^ (w >> 32)) & 0x00000000001fffff;
    w as u32
}

pub fn morton_decode3(code: u64) -> [u32; 3] {
    [compact(code), compact(code >> 1), compact(code >> 2)]
}


const PACKED_NBH_SHIFTS: [u64; 27] = [
    2688, 2176, 2432, 2560, 2048, 2304, 2624, 2112, 2368, 640, 128, 384, 512, 0, 256, 576, 64, 320,
    1664, 1152, 1408, 1536, 1024, 1280, 1600, 1088, 1344,
];

const PACKED_NBH_REGION_SHIFTS: [u64; 7] = [16384, 8192, 24576, 4096, 20480, 12288, 28672];

const NBH_SHIFTS: [UVec3; 27] = [
    UVec3::new(2, 2, 2),
    UVec3::new(2, 0, 2),
    UVec3::new(2, 1, 2),
    UVec3::new(0, 2, 2),
    UVec3::new(0, 0, 2),
    UVec3::new(0, 1, 2),
    UVec3::new(1, 2, 2),
    UVec3::new(1, 0, 2),
    UVec3::new(1, 1, 2),
    UVec3::new(2, 2, 0),
    UVec3::new(2, 0, 0),
    UVec3::new(2, 1, 0),
    UVec3::new(0, 2, 0),
    UVec3::new(0, 0, 0),
    UVec3::new(0, 1, 0),
    UVec3::new(1, 2, 0),
    UVec3::new(1, 0, 0),
    UVec3::new(1, 1, 0),
    UVec3::new(2, 2, 1),
    UVec3::new(2, 0, 1),
    UVec3::new(2, 1, 1),
    UVec3::new(0, 2, 1),
    UVec3::new(0, 0, 1),
    UVec3::new(0, 1, 1),
    UVec3::new(1, 2, 1),
    UVec3::new(1, 0, 1),
    UVec3::new(1, 1, 1),
];

const BLOCK_MX: u64 =
0b0_001_001_001_001_001_001_001_001_001_001_001_001_001_001_001_001_001___000011_0000_00;
const BLOCK_MY: u64 =
0b0_010_010_010_010_010_010_010_010_010_010_010_010_010_010_010_010_010___001100_0000_00;
const BLOCK_MZ: u64 =
0b0_100_100_100_100_100_100_100_100_100_100_100_100_100_100_100_100_100___110000_0000_00;

const PACK_HEADER: usize = 12;
pub const PACK_ALIGN: usize = 6;
pub const REGION_ID_MASK: u64 = !0b0___111111111111; // Mask that only retain the region part of the index.


pub const PACKED_NEG_ONE: u64 = 18446744073709551552;
pub const PACKED_NEG_TWO: u64 = 18446744073709550208;
pub const PACKED_PLUS_FIVE: u64 = 30016;

pub fn pack(x: u32, y: u32, z: u32) -> u64 {
    let mut res = morton_encode3(x >> 2, y >> 2, z >> 2) as u64;

    res = res << PACK_HEADER;
    res = res | ((x as u64 & 0b0011) << PACK_ALIGN);
    res = res | ((y as u64 & 0b0011) << (PACK_ALIGN + 2));
    res | ((z as u64 & 0b0011) << (PACK_ALIGN + 4))
}

fn unpack(xyz: u64) -> [u32; 3] {
    let [x, y, z] = morton_decode3(xyz >> PACK_HEADER);

    [
        (x << 2) | ((xyz as u32 >> (PACK_ALIGN)) & 0b0011),
        (z << 2) | ((xyz as u32 >>  (PACK_ALIGN + 4)) & 0b0011),
        (y << 2) | ((xyz as u32 >> (PACK_ALIGN + 2)) & 0b0011),
    ]
}

