use bevy::prelude::*;

// Morton Curve (Z-Order) implementation for 3D points in Rust
// Range: 0..16 (exclusive) for x, y, z coordinates

/// Spreads bits of a number by inserting two zeros between each bit
/// Used to prepare coordinates for Morton encoding
#[inline]
fn spread_bits(mut value: u32) -> u32 {
    // Ensure value is within 4-bit range (0-15)
    // value &= 0xF;

    // Spread the 4 bits across 12 bits with 2 zeros between each bit
    // 0000abcd -> 00a00b00c00d
    value = (value | (value << 8)) & 0x00F00F; // 0000abcd -> 0000ab0000cd
    value = (value | (value << 4)) & 0x0C30C3; // 0000ab0000cd -> 00a0b00c0d
    value = (value | (value << 2)) & 0x249249; // 00a0b00c0d -> 0a0b0c0d

    value
}

/// Compacts spread bits back to original value
/// Reverses the spread_bits operation
#[inline]
fn compact_bits(mut value: u32) -> u32 {
    // Compact the spread bits back to 4 bits
    value &= 0x249249;
    value = (value | (value >> 2)) & 0x0C30C3;
    value = (value | (value >> 4)) & 0x00F00F;
    value = (value | (value >> 8)) & 0x00000F;

    value
}

/// Converts 3D coordinates to Morton index (linearization)
#[inline]
pub fn to_morton_index(point: UVec3) -> u32 {
    // Interleave bits: z gets the highest bits, then y, then x
    spread_bits(point.x) | (spread_bits(point.y) << 1) | (spread_bits(point.z) << 2)
}

#[inline]
pub fn from_morton_index(index: u32) -> UVec3 {
    // Extract interleaved bits for each coordinate
    let x = compact_bits(index); // Extract every 3rd bit starting from bit 0
    let y = compact_bits(index >> 1); // Extract every 3rd bit starting from bit 1
    let z = compact_bits(index >> 2); // Extract every 3rd bit starting from bit 2

    UVec3 { x, y, z }
}
