use crate::net::EncryptedPDU;
use crate::scheduler::TimeQueueSlotKey;
//use crate::timestamp::Timestamp;
use crate::net;
use crate::timestamp::TimestampTrait;
use core::convert::TryFrom;
use core::time::Duration;
use crypto_mac::generic_array::arr::Inc;

#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum PDUType {
    Network,
    Beacon,
    Provision,
    URI,
    Other(u8),
}
impl PDUType {
    #[must_use]
    pub fn advertisement_type(self) -> u8 {
        match self {
            PDUType::Network => 0x2A,
            PDUType::Beacon => 0x2B,
            PDUType::Provision => 0x29,
            PDUType::URI => 0x24,
            PDUType::Other(o) => o,
        }
    }
    #[must_use]
    pub fn from_advertisement_type(v: u8) -> PDUType {
        match v {
            0x2A => PDUType::Network,
            0x2B => PDUType::Beacon,
            0x29 => PDUType::Provision,
            0x24 => PDUType::URI,
            _ => PDUType::Other(v),
        }
    }
    #[must_use]
    pub fn is_other(self) -> bool {
        match self {
            Self::Other(_) => true,
            _ => false,
        }
    }
    #[must_use]
    pub fn is_mesh(self) -> bool {
        !self.is_other()
    }
}
impl From<PDUType> for u8 {
    #[must_use]
    fn from(p: PDUType) -> Self {
        p.advertisement_type()
    }
}
impl From<u8> for PDUType {
    #[must_use]
    fn from(v: u8) -> Self {
        Self::from_advertisement_type(v)
    }
}
const BLE_ADV_MAX_LEN: usize = 31;
#[derive(Copy, Clone, Hash, Debug, Default)]
pub struct RawAdvertisementPDU {
    buffer: [u8; BLE_ADV_MAX_LEN],
    length: u8,
}
impl RawAdvertisementPDU {
    #[must_use]
    pub fn new_with_length(length: usize) -> Self {
        assert!(
            length <= BLE_ADV_MAX_LEN,
            "{} bytes won't fit in one adv packet",
            length
        );
        Self {
            buffer: Default::default(),
            length: length as u8,
        }
    }
    #[must_use]
    pub fn new(bytes: &[u8]) -> Self {
        let mut out = Self::new_with_length(bytes.len());
        out.data_mut().copy_from_slice(bytes);
        out
    }
    #[must_use]
    pub fn new_payload(pdu_type: PDUType, payload: &[u8]) -> Self {
        let mut out = Self::new_with_length(payload.len() + 1);
        out.buffer[0] = pdu_type.into();
        out.data_mut()[1..].copy_from_slice(payload);
        out
    }
    #[must_use]
    pub fn pdu_type(&self) -> PDUType {
        self.buffer[0].into()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        usize::from(self.length)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.buffer[..self.len()]
    }

    #[must_use]
    pub fn data_mut(&mut self) -> &mut [u8] {
        let l = self.len();
        &mut self.buffer[..l]
    }
}
impl AsRef<[u8]> for RawAdvertisementPDU {
    #[must_use]
    fn as_ref(&self) -> &[u8] {
        self.data()
    }
}
impl AsMut<[u8]> for RawAdvertisementPDU {
    #[must_use]
    fn as_mut(&mut self) -> &mut [u8] {
        self.data_mut()
    }
}
impl TryFrom<RawAdvertisementPDU> for EncryptedPDU {
    type Error = ();

    fn try_from(value: RawAdvertisementPDU) -> Result<Self, Self::Error> {
        if value.pdu_type() == PDUType::Network {
            Ok(Self::from(value.as_ref()))
        } else {
            Err(())
        }
    }
}
pub struct TransmitParameters {}
pub struct OutgoingPDU {
    transmit_parameters: TransmitParameters,
    pdu: net::EncryptedPDU,
}
impl AsRef<net::EncryptedPDU> for OutgoingPDU {
    fn as_ref(&self) -> &EncryptedPDU {
        &self.pdu
    }
}

pub struct IncomingPDU {
    pdu: net::EncryptedPDU,
}
impl AsRef<net::EncryptedPDU> for IncomingPDU {
    fn as_ref(&self) -> &EncryptedPDU {
        &self.pdu
    }
}
pub struct MeshPDUQueue<Timestamp: TimestampTrait> {
    queue: crate::scheduler::SlottedTimeQueue<OutgoingPDU, Timestamp>,
}
pub struct IOError(());
pub trait IOBearer {
    fn send_io_pdu(&mut self, pdu: OutgoingPDU) -> Result<(), IOError>;
}
#[derive(Copy, Clone, Debug, Hash)]
pub struct PDUQueueSlot(TimeQueueSlotKey);
impl<Timestamp: TimestampTrait> MeshPDUQueue<Timestamp> {
    pub fn add(&mut self, delay: Duration, io_pdu: OutgoingPDU) -> PDUQueueSlot {
        PDUQueueSlot(self.queue.push(Timestamp::with_delay(delay), io_pdu))
    }
    pub fn cancel(&mut self, slot: PDUQueueSlot) -> Option<OutgoingPDU> {
        self.queue.remove(slot.0)
    }

    pub fn send_ready(&mut self, bearer: &mut impl IOBearer) -> Result<(), IOError> {
        while let Some((_, pdu)) = self.queue.pop_ready() {
            bearer.send_io_pdu(pdu)?
        }
        Ok(())
    }
}
