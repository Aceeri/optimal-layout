use bevy::prelude::*;

// Morton Curve (Z-Order) implementation for 3D points in Rust
// Range: 0..16 (exclusive) for x, y, z coordinates

/// Spreads bits of a number by inserting two zeros between each bit
/// Used to prepare coordinates for Morton encoding
fn spread_bits(mut value: u32) -> u32 {
    // Ensure value is within 4-bit range (0-15)
    value &= 0xF;

    // Spread the 4 bits across 12 bits with 2 zeros between each bit
    // 0000abcd -> 00a00b00c00d
    value = (value | (value << 8)) & 0x00F00F; // 0000abcd -> 0000ab0000cd
    value = (value | (value << 4)) & 0x0C30C3; // 0000ab0000cd -> 00a0b00c0d
    value = (value | (value << 2)) & 0x249249; // 00a0b00c0d -> 0a0b0c0d

    value
}

/// Compacts spread bits back to original value
/// Reverses the spread_bits operation
fn compact_bits(mut value: u32) -> u32 {
    // Compact the spread bits back to 4 bits
    value &= 0x249249;
    value = (value | (value >> 2)) & 0x0C30C3;
    value = (value | (value >> 4)) & 0x00F00F;
    value = (value | (value >> 8)) & 0x00000F;

    value
}

/// Converts 3D coordinates to Morton index (linearization)
pub fn to_morton_index(point: UVec3) -> u32 {
    // Interleave bits: z gets the highest bits, then y, then x
    spread_bits(point.x) | (spread_bits(point.y) << 1) | (spread_bits(point.z) << 2)
}

/// Morton encoder/decoder for 3D points
pub struct Morton3D;

impl Morton3D {
    /// Converts 3D coordinates to Morton index (linearization)
    pub fn encode(point: UVec3) -> Result<u32, &'static str> {
        if point.x >= 16 || point.y >= 16 || point.z >= 16 {
            return Err("Coordinates must be in range [0, 16)");
        }

        // Interleave bits: z gets the highest bits, then y, then x
        Ok(spread_bits(point.x) | (spread_bits(point.y) << 1) | (spread_bits(point.z) << 2))
    }

    /// Converts Morton index back to 3D coordinates (delinearization)
    pub fn decode(index: u32) -> UVec3 {
        // Extract interleaved bits for each coordinate
        let x = compact_bits(index); // Extract every 3rd bit starting from bit 0
        let y = compact_bits(index >> 1); // Extract every 3rd bit starting from bit 1
        let z = compact_bits(index >> 2); // Extract every 3rd bit starting from bit 2

        UVec3 { x, y, z }
    }

    /// Utility function to visualize bit interleaving
    pub fn visualize_bits(point: UVec3) -> Result<(), &'static str> {
        let UVec3 { x, y, z } = point;
        let morton_index = Self::encode(point)?;

        println!("\nBit visualization for point ({}, {}, {}):", x, y, z);
        println!("X = {:04b} (binary)", x);
        println!("Y = {:04b} (binary)", y);
        println!("Z = {:04b} (binary)", z);
        println!("Morton = {:012b} (interleaved: ZYXZYXZYXZYX)", morton_index);
        println!("Morton Index = {}", morton_index);

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_encode_decode_roundtrip() {
//         for x in 0..16 {
//             for y in 0..16 {
//                 for z in 0..16 {
//                     let encoded = Morton3D::encode(x, y, z).unwrap();
//                     let decoded = Morton3D::decode(encoded);
//                     assert_eq!(decoded.x, x);
//                     assert_eq!(decoded.y, y);
//                     assert_eq!(decoded.z, z);
//                 }
//             }
//         }
//     }

//     #[test]
//     fn test_point3d_struct() {
//         let point = UVec3::new(5, 10, 15);
//         let morton = to_morton_index(point);
//         let expected = Morton3D::encode(5, 10, 15).unwrap();
//         assert_eq!(morton, expected);
//     }

//     #[test]
//     fn test_boundary_conditions() {
//         // Test valid boundaries
//         assert!(Morton3D::encode(0, 0, 0).is_ok());
//         assert!(Morton3D::encode(15, 15, 15).is_ok());

//         // Test invalid boundaries
//         assert!(Morton3D::encode(16, 0, 0).is_err());
//         assert!(Morton3D::encode(0, 16, 0).is_err());
//         assert!(Morton3D::encode(0, 0, 16).is_err());
//     }

//     #[test]
//     fn test_known_values() {
//         // Test some known Morton values
//         assert_eq!(Morton3D::encode(0, 0, 0).unwrap(), 0);
//         assert_eq!(Morton3D::encode(1, 0, 0).unwrap(), 1);
//         assert_eq!(Morton3D::encode(0, 1, 0).unwrap(), 2);
//         assert_eq!(Morton3D::encode(0, 0, 1).unwrap(), 4);
//         assert_eq!(Morton3D::encode(1, 1, 1).unwrap(), 7);
//     }
// }
