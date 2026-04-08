
#[derive(Clone, Debug)]
pub struct Candidate {
    pub candidate: String,
    pub sdp_mid: String,
    pub sdp_m_line_index: usize,
}

impl Candidate {
    pub fn new(candidate: String, sdp_mid: Option<String>, sdp_m_line_index: Option<i64>) -> Self {
        Candidate {
            candidate,
            sdp_mid: sdp_mid.unwrap_or_else(|| "0".to_string()),
            sdp_m_line_index: sdp_m_line_index.unwrap_or(0) as usize,
        }
    }
}