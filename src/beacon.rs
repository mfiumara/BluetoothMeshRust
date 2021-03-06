//! Bluetooth Mesh Beacon Layer. Currently only supports `SecureNetworkBeacon`s and
//! `UnprovisionedDeviceBeacon`s.
use crate::bytes::ToFromBytesEndian;
use crate::crypto::s1;
use crate::uuid::UUID;
use core::convert::TryInto;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum BeaconError {
    BadBytes,
    BadLength,
}

pub trait Beacon: Sized {
    fn byte_len(&self) -> usize;
    const BEACON_TYPE: BeaconType;
    fn pack_into(&self, buf: &mut [u8]) -> Result<(), BeaconError>;
    fn unpack_from(buf: &[u8]) -> Result<Self, BeaconError>;
}

#[repr(u16)]
pub enum OOBFlags {
    Other = 0x00,
    ElectronicURI = 0x01,
    MachineReadable2DCode = 0x02,
    BarCode = 0x03,
    NearFieldCommunications = 0x04,
    Number = 0x05,
    String = 0x06,
    RFU0 = 0x07,
    RFU1 = 0x08,
    RFU2 = 0x09,
    RFU3 = 0x0A,
    OnBox = 0x0B,
    InsideBox = 0x0C,
    OnPieceOfPaper = 0x0D,
    InsideManual = 0x0E,
    OnDevice = 0x0F,
}
impl From<OOBFlags> for u16 {
    fn from(f: OOBFlags) -> Self {
        f as u16
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Default, Debug)]
pub struct OOBInformation(pub u16);
impl OOBInformation {
    pub fn set(mut self, flag: OOBFlags) -> Self {
        self.0 |= 1_u16 << u16::from(flag);
        self
    }
    pub fn get(self, flag: OOBFlags) -> bool {
        self.0 & (1_u16 << u16::from(flag)) != 0
    }
}
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Debug, Hash)]
pub struct URIHash(pub u32);
impl URIHash {
    pub fn hash_data(data: &[u8]) -> URIHash {
        URIHash(
            u32::from_bytes_be(&s1(data).as_ref()[..=3])
                .expect("s1 returns 13 bytes and we only need 4"),
        )
    }
}
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Debug, Hash)]
pub struct UnprovisionedDeviceBeacon {
    pub uuid: UUID,
    pub oob_information: OOBInformation,
    pub uri_hash: Option<URIHash>,
}
impl UnprovisionedDeviceBeacon {
    pub const fn max_len() -> usize {
        Self::min_len() + 4
    }
    pub const fn min_len() -> usize {
        16 + 2
    }
}
impl Beacon for UnprovisionedDeviceBeacon {
    fn byte_len(&self) -> usize {
        if self.uri_hash.is_some() {
            Self::max_len()
        } else {
            Self::min_len()
        }
    }
    const BEACON_TYPE: BeaconType = BeaconType::Unprovisioned;

    fn pack_into(&self, buf: &mut [u8]) -> Result<(), BeaconError> {
        if buf.len() == self.byte_len() {
            buf[..16].copy_from_slice(self.uuid.as_ref());
            buf[16..18].copy_from_slice(&self.oob_information.0.to_be_bytes());
            match self.uri_hash {
                Some(uri_hash) => buf[18..].copy_from_slice(&uri_hash.0.to_be_bytes()),
                None => (),
            }
            Ok(())
        } else {
            Err(BeaconError::BadLength)
        }
    }

    fn unpack_from(buf: &[u8]) -> Result<Self, BeaconError> {
        if buf.len() == Self::min_len() {
            let uuid = UUID(
                (&buf[..16])
                    .try_into()
                    .expect("uuid length is always 16 bytes"),
            );
            let oob = OOBInformation(
                u16::from_bytes_be(&buf[16..18]).expect("OOBInformation always 2 bytes"),
            );
            Ok(Self {
                uuid,
                oob_information: oob,
                uri_hash: None,
            })
        } else if buf.len() == Self::max_len() {
            let uuid = UUID(
                (&buf[..16])
                    .try_into()
                    .expect("uuid length is always 16 bytes"),
            );
            let oob = OOBInformation(
                u16::from_bytes_be(&buf[16..18]).expect("OOBInformation always 2 bytes"),
            );
            let hash = URIHash(u32::from_bytes_be(&buf[18..]).expect("URIHash is always 4 bytes"));
            Ok(Self {
                uuid,
                oob_information: oob,
                uri_hash: Some(hash),
            })
        } else {
            Err(BeaconError::BadLength)
        }
    }
}
pub struct SecureNetworkBeacon {}
pub enum BeaconType {
    Unprovisioned = 0x00,
    SecureNetwork = 0x01,
}
pub enum BeaconPDU {
    Unprovisioned(UnprovisionedDeviceBeacon),
    SecureNetwork(SecureNetworkBeacon),
}
pub struct PackedBeacon {}
impl AsRef<[u8]> for PackedBeacon {
    fn as_ref(&self) -> &[u8] {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use crate::beacon::{Beacon, OOBFlags, OOBInformation, URIHash, UnprovisionedDeviceBeacon};
    use crate::mesh;
    use crate::uuid::UUID;

    #[test]
    pub fn test_unprovisioned() {
        let uuid = UUID(
            UUID::uuid_bytes_from_str("70cf7c9732a345b691494810d2e9cbf4").expect("from spec 8.4.1"),
        );
        let oob = OOBInformation::default()
            .set(OOBFlags::String)
            .set(OOBFlags::OnPieceOfPaper)
            .set(OOBFlags::OnDevice);
        assert_eq!(oob.0, 0xA040);
        let beacon = UnprovisionedDeviceBeacon {
            uuid,
            oob_information: oob,
            uri_hash: None,
        };
        let mut buf = [0_u8; UnprovisionedDeviceBeacon::min_len()];
        let expected: [u8; UnprovisionedDeviceBeacon::min_len()] =
            mesh::bytes_str_to_buf("70cf7c9732a345b691494810d2e9cbf4a040")
                .expect("from spec 8.4.1");
        beacon.pack_into(&mut buf[..]).expect("simple beacon pack");
        assert_eq!(buf, expected);
    }
    #[test]
    pub fn test_unprovisioned_with_uri() {
        // 0x17 is uri::URIName::https.
        let uri = "\x17//www.example.com/mesh/products/light-switch-v3";
        let _oob = OOBInformation::default()
            .set(OOBFlags::Number)
            .set(OOBFlags::InsideManual);
        let uri_hash = URIHash::hash_data(uri.as_bytes());
        assert_eq!(u32::from_be_bytes([0xD9, 0x74, 0x78, 0xB3]), uri_hash.0);
    }
}
