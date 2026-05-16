use crate::config::config::Config;
use anyhow::anyhow;
use rand::RngExt;
use rsip::headers::ToTypedHeader;
use rsip::headers::UntypedHeader;
use rsip::message::HeadersExt;
use rsip::{Header, Headers, Method, Param, Request, Response, SipMessage, StatusCode, Version};
use std::sync::atomic::{AtomicU64, Ordering};

const CHARACTERS: &[u8] = b"abcdefghijkmnpqrstxyz1234567890";
static TMP_GENERATOR: AtomicU64 = AtomicU64::new(0);

/// Generate a random string of specified length using alphanumeric characters.
#[inline]
pub fn random_string(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| CHARACTERS[rng.random_range(0..CHARACTERS.len())] as char)
        .collect()
}

/// Generate a unique SIP tag for From/To headers.
#[inline]
pub fn random_tag() -> String {
    let counter = TMP_GENERATOR.fetch_add(1, Ordering::SeqCst) + 1;
    format!("{}.{}", random_string(20), counter)
}

/// Generate a unique SIP branch parameter for Via header.
/// Branch must start with "z9hG4bK" per RFC 3261.
#[inline]
pub fn random_branch() -> String {
    let counter = TMP_GENERATOR.fetch_add(1, Ordering::SeqCst) + 1;
    format!("z9hG4bK.{}.{}", random_string(28), counter)
}

/// Extract pending transaction ID from a SIP message.
/// Format: "{cseq}_{branch}_{call_id}"
pub fn get_pending_transaction_id(msg: &SipMessage) -> Option<String> {
    let (cseq_header, via_header, call_id) = match msg {
        SipMessage::Request(req) => (
            req.cseq_header().ok()?,
            req.via_header().ok()?,
            req.call_id_header().ok()?.value().to_string(),
        ),
        SipMessage::Response(res) => (
            res.cseq_header().ok()?,
            res.via_header().ok()?,
            res.call_id_header().ok()?.value().to_string(),
        ),
    };

    let seq = cseq_header.seq().unwrap_or_default();
    let branch = extract_branch_from_via(&via_header.typed().ok()?)?;

    Some(format!("{}_{}_{}", seq, branch, call_id))
}

/// Extract transaction ID from a SIP message.
/// Format: "{cseq}_{method}_{branch}"
pub fn get_transaction_id(msg: &SipMessage) -> Option<String> {
    let (cseq_header, via_header) = match msg {
        SipMessage::Request(req) => (req.cseq_header().ok()?, req.via_header().ok()?),
        SipMessage::Response(res) => (res.cseq_header().ok()?, res.via_header().ok()?),
    };

    let seq = cseq_header.seq().unwrap_or_default();
    let method = cseq_header
        .method()
        .map(|m| m.to_string())
        .unwrap_or_default();
    let branch = extract_branch_from_via(&via_header.typed().ok()?)?;

    Some(format!(
        "{}_{}_{}",
        seq,
        method.to_lowercase(),
        branch.to_lowercase()
    ))
}

/// Extract branch parameter from typed Via header.
#[inline]
fn extract_branch_from_via(via: &rsip::typed::Via) -> Option<String> {
    via.params.iter().find_map(|p| {
        if let Param::Branch(b) = p {
            Some(b.value().to_string())
        } else {
            None
        }
    })
}

/// Build SIP dialog ID from message headers.
/// For UAC: "{call_id}_{from_tag}_{to_tag}"
/// For UAS: "{call_id}_{to_tag}_{from_tag}"
pub fn get_dialog_id(message: &SipMessage, is_uac: bool) -> anyhow::Result<String> {
    let to_header = message
        .to_header()
        .map_err(|_| anyhow!("missing To header"))?;
    let from_header = message
        .from_header()
        .map_err(|_| anyhow!("missing From header"))?;
    let call_id = message
        .call_id_header()
        .map_err(|_| anyhow!("missing Call-ID header"))?
        .value()
        .to_string();

    let to_tag = extract_to_tag(&to_header)?;
    let from_tag = extract_from_tag(&from_header)?;

    if to_tag.is_empty() || from_tag.is_empty() {
        return Err(anyhow!("empty To/From tag"));
    }

    Ok(if is_uac {
        format!("{}_{}_{}", call_id, from_tag, to_tag)
    } else {
        format!("{}_{}_{}", call_id, to_tag, from_tag)
    })
}

/// Extract tag value from a From header.
#[inline]
fn extract_from_tag(header: &rsip::headers::From) -> anyhow::Result<String> {
    header
        .tag()
        .map_err(|_| anyhow!("invalid tag"))?
        .ok_or_else(|| anyhow!("tag not present"))
        .map(|t| t.value().to_string())
}

/// Extract tag value from a To header.
#[inline]
fn extract_to_tag(header: &rsip::headers::To) -> anyhow::Result<String> {
    header
        .tag()
        .map_err(|_| anyhow!("invalid tag"))?
        .ok_or_else(|| anyhow!("tag not present"))
        .map(|t| t.value().to_string())
}

/// Build BYE request for UAS (callee) side.
///
/// When UAS sends BYE:
/// - From = To of 200 OK (UAS identity with to-tag)
/// - To = From of INVITE (UAC identity with from-tag)
/// - Route = Record-Route from 200 OK (same order)
/// - Request-URI = Contact from INVITE
pub fn build_bye_request_with_dialog_for_uas(
    invite: &Request,
    ok_response: &Response,
    config: &Config,
) -> anyhow::Result<Request> {
    let mut headers = Headers::default();

    // 1. Via header - create new branch for BYE transaction
    headers.push(build_via_header(config));

    // 2. Route headers from Record-Route of 200 OK (keep same order for UAS)
    append_route_headers(&mut headers, ok_response, false);

    // 3. Max-Forwards
    headers.push(rsip::headers::MaxForwards::default().into());

    // 4. From header = To of 200 OK (UAS identity with to-tag)
    let to_typed = ok_response.to_header()?.typed()?;
    headers.push(Header::From(
        rsip::typed::From {
            display_name: to_typed.display_name.clone(),
            uri: to_typed.uri.clone(),
            params: to_typed.params.clone(),
        }
        .into(),
    ));

    // 5. To header = From of INVITE (UAC identity with from-tag)
    let from_typed = invite.from_header()?.typed()?;
    headers.push(Header::To(
        rsip::typed::To {
            display_name: from_typed.display_name.clone(),
            uri: from_typed.uri.clone(),
            params: from_typed.params.clone(),
        }
        .into(),
    ));

    // 6. Call-ID (unchanged)
    headers.push(Header::CallId(invite.call_id_header()?.clone().into()));

    // 7. CSeq - increment sequence number, method = BYE
    let cseq = invite.cseq_header()?.typed()?;
    headers.push(Header::CSeq(
        rsip::typed::CSeq {
            seq: cseq.seq + 1,
            method: Method::Bye,
        }
        .into(),
    ));

    // 8. Contact (optional)
    if let Ok(contact) = invite.contact_header() {
        headers.push(Header::Contact(contact.clone().into()));
    }

    // 9. Content-Length: 0
    headers.push(rsip::headers::ContentLength::new("0").into());

    // 10. Request-URI: from Contact of INVITE or INVITE URI
    let req_uri = invite
        .contact_header()
        .ok()
        .and_then(|c| c.uri().ok().map(|u| u.clone()))
        .unwrap_or_else(|| invite.uri.clone());

    Ok(Request {
        method: Method::Bye,
        uri: req_uri,
        version: Version::V2,
        headers,
        body: vec![],
    })
}

/// Build BYE request for UAC (caller) side.
///
/// When UAC sends BYE:
/// - From = From of INVITE (UAC identity, unchanged)
/// - To = To of 200 OK (UAS identity with to-tag)
/// - Route = Record-Route from 200 OK (reversed order)
/// - Request-URI = Contact from 200 OK
pub fn build_bye_request_with_dialog_for_uac(
    invite: &Request,
    ok_response: &Response,
) -> anyhow::Result<Request> {
    let mut headers = Headers::default();

    // 1. Via header - create new branch for BYE transaction
    let via = invite.via_header()?;
    let mut via_typed = via.typed()?;
    let new_branch = format!("z9hG4bK{}", uuid::Uuid::new_v4());
    via_typed.params.retain(|p| !matches!(p, Param::Branch(_)));
    via_typed
        .params
        .push(Param::Branch(rsip::param::Branch::new(new_branch)));
    headers.push(Header::Via(via_typed.into()));

    // 2. Route headers from Record-Route of 200 OK (reversed order for UAC)
    append_route_headers(&mut headers, ok_response, true);

    // 3. Max-Forwards
    headers.push(rsip::headers::MaxForwards::default().into());

    // 4. From header (unchanged from INVITE, including tag)
    headers.push(Header::From(invite.from_header()?.clone().into()));

    // 5. To header (from 200 OK response, with to-tag)
    headers.push(Header::To(ok_response.to_header()?.clone().into()));

    // 6. Call-ID (unchanged)
    headers.push(Header::CallId(invite.call_id_header()?.clone().into()));

    // 7. CSeq - increment sequence number, method = BYE
    let cseq = invite.cseq_header()?.typed()?;
    headers.push(Header::CSeq(
        rsip::typed::CSeq {
            seq: cseq.seq + 1,
            method: Method::Bye,
        }
        .into(),
    ));

    // 8. Contact (optional)
    if let Ok(contact) = invite.contact_header() {
        headers.push(Header::Contact(contact.clone().into()));
    }

    // 9. Content-Length: 0
    headers.push(rsip::headers::ContentLength::new("0").into());

    // 10. Request-URI: from Contact of 200 OK or INVITE URI
    let req_uri = ok_response
        .contact_header()
        .ok()
        .and_then(|c| c.uri().ok().map(|u| u.clone()))
        .unwrap_or_else(|| invite.uri.clone());

    Ok(Request {
        method: Method::Bye,
        uri: req_uri,
        version: Version::V2,
        headers,
        body: vec![],
    })
}

/// Build CANCEL request for an INVITE transaction.
/// CANCEL uses the same Via, From, To, Call-ID as the original INVITE.
pub fn build_cancel_request(invite: &Request) -> anyhow::Result<Request> {
    let mut headers = Headers::default();

    if let Ok(via) = invite.via_header() {
        headers.push(Header::Via(via.clone().into()));
    }

    headers.push(rsip::headers::MaxForwards::default().into());

    if let Ok(from) = invite.from_header() {
        headers.push(Header::From(from.clone().into()));
    }

    if let Ok(to) = invite.to_header() {
        headers.push(Header::To(to.clone().into()));
    }

    if let Ok(call_id) = invite.call_id_header() {
        headers.push(Header::CallId(call_id.clone().into()));
    }

    let cseq = invite.cseq_header()?.typed()?;
    headers.push(Header::CSeq(
        rsip::typed::CSeq {
            seq: cseq.seq,
            method: Method::Cancel,
        }
        .into(),
    ));

    headers.push(rsip::headers::ContentLength::new("0").into());

    Ok(Request {
        method: Method::Cancel,
        uri: invite.uri.clone(),
        version: Version::V2,
        headers,
        body: vec![],
    })
}

// =============================================================================
// Helper functions
// =============================================================================

/// Build Via header using config settings.
#[inline]
fn build_via_header(config: &Config) -> Header {
    let via_uri = format!(
        "{}:{}",
        config.sip_transport.public_ip, config.sip_transport.port
    );
    rsip::typed::Via {
        version: Version::V2,
        transport: rsip::Transport::Udp,
        uri: rsip::Uri {
            host_with_port: rsip::Domain::from(via_uri).into(),
            ..Default::default()
        },
        params: vec![
            Param::Branch(random_branch().into()),
            Param::Other("rport".into(), None),
        ],
    }
    .into()
}

/// Append Route headers from Record-Route of response.
/// For UAC: reverse order. For UAS: keep same order.
#[inline]
fn append_route_headers(headers: &mut Headers, response: &Response, reverse: bool) {
    let record_routes: Vec<_> = response
        .headers
        .iter()
        .filter_map(|h| match h {
            Header::RecordRoute(rr) => Some(rr.clone()),
            _ => None,
        })
        .collect();

    let iter: Box<dyn Iterator<Item = _>> = if reverse {
        Box::new(record_routes.into_iter().rev())
    } else {
        Box::new(record_routes.into_iter())
    };

    for rr in iter {
        headers.push(Header::Route(rsip::headers::Route::new(
            rr.value().to_string(),
        )));
    }
}

pub fn build_response_183_with_sdp(
    config: &Config,
    invite: &Request,
    sdp: String,
    to_tag: rsip::common::uri::param::tag::Tag,
) -> Response {
    let mut headers = Headers::default();

    // Copy Via headers
    invite.headers.iter().for_each(|e| {
        if let Header::Via(via) = e {
            headers.push(Header::Via(via.clone().into()));
        }
    });

    // Copy Record-Route headers
    invite.headers.iter().for_each(|e| {
        if let Header::RecordRoute(r) = e {
            headers.push(Header::RecordRoute(r.clone().into()));
        }
    });

    // From header
    headers.push(Header::From(invite.from_header().unwrap().clone().into()));

    // To header — 183 cũng cần tag (best practice, một số UA yêu cầu)
    let mut to = invite.to_header().unwrap().clone();
    to.mut_tag(to_tag).unwrap();
    headers.push(Header::To(to.into()));

    // Call-ID
    headers.push(Header::CallId(
        invite.call_id_header().unwrap().clone().into(),
    ));

    // CSeq
    headers.push(Header::CSeq(invite.cseq_header().unwrap().clone().into()));

    // Contact
    let proxy = format!(
        "{}:{}",
        config.sip_transport.public_ip, config.sip_transport.port
    );
    let contact = rsip::typed::Contact {
        display_name: None,
        uri: rsip::Uri {
            scheme: Some(rsip::Scheme::Sip),
            auth: None,
            host_with_port: rsip::Domain::from(proxy).into(),
            params: Default::default(),
            headers: Default::default(),
        },
        params: Default::default(),
    };
    headers.push(Header::Contact(contact.into()));

    // Require: 100rel nếu INVITE có Supported/Require: 100rel
    let has_100rel = invite.headers.iter().any(|h| match h {
        Header::Require(v) => v.to_string().contains("100rel"),
        Header::Supported(v) => v.to_string().contains("100rel"),
        _ => false,
    });
    if has_100rel {
        headers.push(Header::Require("100rel".into()));
        // RSeq — bắt đầu từ 1, tăng dần nếu có nhiều provisional
        headers.push(Header::Other("RSeq".into(), "1".into()));
    }

    // Content
    headers.push(Header::ContentType("application/sdp".into()));
    headers.push(Header::ContentLength(sdp.len().to_string().into()));

    Response {
        version: invite.version().clone(),
        status_code: StatusCode::SessionProgress, // 183
        headers,
        body: sdp.into_bytes(),
    }
}
