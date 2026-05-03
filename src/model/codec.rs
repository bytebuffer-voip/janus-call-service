
#[derive(Debug)]
pub struct CodecInfo {
    pub janus_name: &'static str,
    pub payload_type: u8,
    pub need_pt_in_rtp: bool,
}