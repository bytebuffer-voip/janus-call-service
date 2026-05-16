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

pub fn build_sdp_answer(janus_ip: &str, janus_port: u16, codec: &CodecInfo) -> String {
    let codec_line = match codec.payload_type {
        8 => "a=rtpmap:8 PCMA/8000\r\n".to_string(),
        0 => "a=rtpmap:0 PCMU/8000\r\n".to_string(),
        pt => format!(
            "a=rtpmap:{pt} opus/48000/2\r\n\
             a=fmtp:{pt} minptime=20; useinbandfec=1\r\n",
            pt = pt
        ),
    };

    format!(
        "v=0\r\n\
         o=RustApp 0 0 IN IP4 {ip}\r\n\
         s=OmiStack\r\n\
         c=IN IP4 {ip}\r\n\
         t=0 0\r\n\
         m=audio {port} RTP/AVP {pt} 101\r\n\
         {codec}\
         a=rtpmap:101 telephone-event/8000\r\n\
         a=fmtp:101 0-16\r\n\
         a=sendrecv\r\n\
         a=ptime:20\r\n",
        ip = janus_ip,
        port = janus_port,
        pt = codec.payload_type,
        codec = codec_line,
    )
}

pub fn sdp_set_direction(sdp: &str, direction: &str) -> String {
    let direction_attrs = ["a=sendrecv", "a=sendonly", "a=recvonly", "a=inactive"];
    let mut found = false;
    let result = sdp
        .lines()
        .map(|line| {
            if direction_attrs
                .iter()
                .any(|d| line.trim_start().starts_with(d))
            {
                found = true;
                format!("a={}", direction)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n");
    if !found {
        let result = sdp
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("m=") {
                    format!("a={}\r\n{}", direction, line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\r\n");
        return result;
    }
    result
}
