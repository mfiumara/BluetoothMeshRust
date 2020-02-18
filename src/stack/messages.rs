//! Appliation layer.

use crate::address::{Address, UnicastAddress};
use crate::btle::RSSI;
use crate::crypto::aes::MicSize;
use crate::crypto::nonce::{AppNonce, AppNonceParts, DeviceNonce, DeviceNonceParts};
use crate::device_state::SeqRange;
use crate::lower::SegO;
use crate::mesh::{AppKeyIndex, ElementIndex, IVIndex, NetKeyIndex, SequenceNumber, TTL};
use crate::upper::{AppPayload, EncryptedAppPayload};
use crate::{control, lower, net, upper};

pub enum MessageKeys {
    Device(NetKeyIndex),
    App(AppKeyIndex),
}
pub struct OutgoingDestination {
    pub dst: Address,
    pub ttl: Option<TTL>,
    pub app_key_index: AppKeyIndex,
}
pub struct OutgoingMessage<Storage: AsRef<[u8]> + AsMut<[u8]>> {
    pub app_payload: AppPayload<Storage>,
    pub mic_size: MicSize,
    pub force_segment: bool,
    pub encryption_key: MessageKeys,
    pub iv_index: IVIndex,
    pub source_element_index: ElementIndex,
    pub dst: Address,
    pub ttl: Option<TTL>,
}
pub struct OutgoingLowerTransportMessage {
    pub pdu: lower::PDU,
    pub src: UnicastAddress,
    pub dst: Address,
    pub ttl: Option<TTL>,
    pub seq: Option<SequenceNumber>,
    pub iv_index: IVIndex,
    pub net_key_index: NetKeyIndex,
}
impl<Storage: AsRef<[u8]> + AsMut<[u8]>> OutgoingMessage<Storage> {
    pub fn data_with_mic_len(&self) -> usize {
        self.app_payload.0.as_ref().len() + self.mic_size.byte_size()
    }
    pub fn should_segment(&self) -> bool {
        self.force_segment || self.app_payload.should_segment(self.mic_size)
    }
    pub fn seg_o(&self) -> Option<SegO> {
        if !self.should_segment() {
            None
        } else {
            Some(upper::calculate_seg_o(
                self.data_with_mic_len(),
                lower::SegmentedAccessPDU::max_seg_len(),
            ))
        }
    }
}
pub struct OutgoingUpperTransportMessage<Storage: AsRef<[u8]> + AsMut<[u8]>> {
    pub upper_pdu: upper::PDU<Storage>,
    pub iv_index: IVIndex,
    pub seq: SeqRange,
    pub seg_count: SegO,
    pub net_key_index: NetKeyIndex,
    pub src: UnicastAddress,
    pub dst: Address,
    pub ttl: Option<TTL>,
}
pub struct EncryptedIncomingMessage<Storage: AsRef<[u8]>> {
    pub encrypted_app_payload: EncryptedAppPayload<Storage>,
    pub seq: SequenceNumber,
    pub seg_count: u8,
    pub iv_index: IVIndex,
    pub net_key_index: NetKeyIndex,
    pub dst: Address,
    pub src: UnicastAddress,
    pub ttl: Option<TTL>,
    pub rssi: Option<RSSI>,
}
impl<Storage: AsRef<[u8]>> EncryptedIncomingMessage<Storage> {
    pub fn app_nonce_parts(&self) -> AppNonceParts {
        AppNonceParts {
            aszmic: self.szmic(),
            seq: self.seq,
            src: self.src,
            dst: self.dst,
            iv_index: self.iv_index,
        }
    }
    pub fn app_nonce(&self) -> AppNonce {
        self.app_nonce_parts().to_nonce()
    }
    pub fn szmic(&self) -> bool {
        self.encrypted_app_payload.mic().is_big()
    }
    pub fn device_nonce_parts(&self) -> DeviceNonceParts {
        DeviceNonceParts {
            aszmic: self.szmic(),
            seq: self.seq,
            src: self.src,
            dst: self.dst,
            iv_index: self.iv_index,
        }
    }
    pub fn device_nonce(&self) -> DeviceNonce {
        self.device_nonce_parts().to_nonce()
    }
}
pub struct IncomingControlMessage {
    pub control_pdu: control::ControlPDU,
    pub src: UnicastAddress,
    pub rssi: Option<RSSI>,
    pub ttl: Option<TTL>,
}
pub struct IncomingMessage<Storage: AsRef<[u8]>> {
    pub payload: Storage,
    pub src: UnicastAddress,
    pub dst: Address,
    pub seq: SequenceNumber,
    pub iv_index: IVIndex,
    pub net_key_index: NetKeyIndex,
    pub app_key_index: Option<AppKeyIndex>,
    pub ttl: Option<TTL>,
    pub rssi: Option<RSSI>,
}
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct IncomingNetworkPDU {
    pub pdu: net::PDU,
    pub net_key_index: NetKeyIndex,
    pub iv_index: IVIndex,
    pub rssi: Option<RSSI>,
}
pub struct IncomingTransportPDU<Storage: AsRef<[u8]> + AsMut<[u8]>> {
    pub upper_pdu: upper::PDU<Storage>,
    pub iv_index: IVIndex,
    pub seg_count: u8,
    pub seq: SequenceNumber,
    pub net_key_index: NetKeyIndex,
    pub ttl: Option<TTL>,
    pub rssi: Option<RSSI>,
    pub src: UnicastAddress,
    pub dst: Address,
}
