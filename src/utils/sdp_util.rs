use std::collections::HashMap;

#[derive(Debug)]
pub struct CodecInfo {
    pub janus_name: &'static str,
    pub payload_type: u8,
    pub need_pt_in_rtp: bool,
}

pub fn select_codec(sdp: &str) -> Option<CodecInfo> {
    let lines: Vec<&str> = sdp.lines().collect();
    let mut pt_map: HashMap<u8, String> = HashMap::new();
    for line in &lines {
        if let Some(rest) = line.strip_prefix("a=rtpmap:") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                if let Ok(pt) = parts[0].parse::<u8>() {
                    let codec = parts[1].to_lowercase();
                    pt_map.insert(pt, codec);
                }
            }
        }
    }
    let priority = [(8u8, "pcma", false), (0u8, "pcmu", false)];
    for (pt, name, need_pt) in &priority {
        if pt_map.contains_key(pt) {
            return Some(CodecInfo {
                janus_name: name,
                payload_type: *pt,
                need_pt_in_rtp: *need_pt,
            });
        }
    }
    for (pt, codec) in &pt_map {
        if codec.starts_with("opus/") {
            return Some(CodecInfo {
                janus_name: "opus",
                payload_type: *pt,
                need_pt_in_rtp: true,
            });
        }
    }
    None // G.729, GSM, Speex, iLBC → no support
}

pub fn parse_sdp_ip_port(sdp: &str) -> Option<(String, u16)> {
    let mut ip: Option<String> = None;
    let mut port: Option<u16> = None;

    for line in sdp.lines() {
        let line = line.trim();

        if line.starts_with("c=IN IP4") {
            // c=IN IP4 192.168.0.100
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                ip = Some(parts[2].to_string());
            }
        }

        if line.starts_with("m=audio") {
            // m=audio 65264 RTP/AVP ...
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(p) = parts[1].parse::<u16>() {
                    port = Some(p);
                }
            }
        }
    }

    match (ip, port) {
        (Some(i), Some(p)) => Some((i, p)),
        _ => None,
    }
}
