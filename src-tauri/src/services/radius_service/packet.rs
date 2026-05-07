use super::{
    RadiusAccountingRequest, RadiusChapAuthRequest, RadiusMschapV2AuthRequest,
    RadiusPapAuthRequest, RadiusPapAuthResult, RadiusReplyAttribute,
};
use crate::error::{AppError, AppResult};
use crate::models::RadiusAccountingStatusType;
use md5::compute as md5_compute;
use radius::core::avp::AVP;
use radius::core::code::Code;
use radius::core::packet::Packet;
use radius::core::{rfc2865, rfc2866, rfc2869, vsa::StringVSA};
use std::net::Ipv4Addr;
use std::str::FromStr;

const MIKROTIK_VENDOR_ID: i32 = 14988;
const MIKROTIK_GROUP_VENDOR_TYPE: u8 = 3;
const MICROSOFT_VENDOR_ID: [u8; 4] = 311_i32.to_be_bytes();
const MS_CHAP_CHALLENGE_VENDOR_TYPE: u8 = 11;
const MS_CHAP2_RESPONSE_VENDOR_TYPE: u8 = 25;
const MS_CHAP2_SUCCESS_VENDOR_TYPE: u8 = 26;
const MESSAGE_AUTHENTICATOR_TYPE: u8 = 80;
const MESSAGE_AUTHENTICATOR_ATTRIBUTE_LENGTH: u8 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadiusAccessDecision {
    Accept,
    Reject,
}

pub fn decode_request(bytes: &[u8], secret: &[u8]) -> AppResult<Packet> {
    Packet::decode(bytes, secret)
        .map_err(|error| AppError::Validation(format!("failed to decode radius packet: {error}")))
}

pub fn extract_pap_auth_request(
    packet: &Packet,
    source_ip: &str,
) -> AppResult<RadiusPapAuthRequest> {
    let username = rfc2865::lookup_user_name(packet)
        .transpose()
        .map_err(|error| AppError::Validation(format!("invalid user-name attribute: {error}")))?
        .ok_or_else(|| AppError::Validation("missing user-name attribute".to_string()))?;

    let password = rfc2865::lookup_user_password(packet)
        .transpose()
        .map_err(|error| AppError::Validation(format!("invalid user-password attribute: {error}")))?
        .ok_or_else(|| AppError::Validation("missing user-password attribute".to_string()))?;

    let password = String::from_utf8(password)
        .map_err(|_| AppError::Validation("user-password is not valid utf-8".to_string()))?;

    Ok(RadiusPapAuthRequest {
        source_ip: source_ip.to_string(),
        username,
        password,
    })
}

pub fn is_chap_access_request(packet: &Packet) -> bool {
    rfc2865::lookup_chap_password(packet).is_some()
}

pub fn is_mschapv2_access_request(packet: &Packet) -> bool {
    lookup_vendor_specific(packet, MS_CHAP2_RESPONSE_VENDOR_TYPE).is_some()
}

pub fn extract_chap_auth_request(
    packet: &Packet,
    source_ip: &str,
) -> AppResult<RadiusChapAuthRequest> {
    let username = rfc2865::lookup_user_name(packet)
        .transpose()
        .map_err(|error| AppError::Validation(format!("invalid user-name attribute: {error}")))?
        .ok_or_else(|| AppError::Validation("missing user-name attribute".to_string()))?;

    let chap_password = rfc2865::lookup_chap_password(packet)
        .ok_or_else(|| AppError::Validation("missing chap-password attribute".to_string()))?;

    if chap_password.len() != 17 {
        return Err(AppError::Validation(
            "chap-password attribute must be 17 bytes".to_string(),
        ));
    }

    let challenge = rfc2865::lookup_chap_challenge(packet)
        .unwrap_or_else(|| packet.get_authenticator().clone());

    Ok(RadiusChapAuthRequest {
        source_ip: source_ip.to_string(),
        username,
        chap_identifier: chap_password[0],
        challenge,
        response: chap_password[1..].to_vec(),
    })
}

pub fn extract_mschapv2_auth_request(
    packet: &Packet,
    source_ip: &str,
) -> AppResult<RadiusMschapV2AuthRequest> {
    let username = rfc2865::lookup_user_name(packet)
        .transpose()
        .map_err(|error| AppError::Validation(format!("invalid user-name attribute: {error}")))?
        .ok_or_else(|| AppError::Validation("missing user-name attribute".to_string()))?;

    let challenge = lookup_vendor_specific(packet, MS_CHAP_CHALLENGE_VENDOR_TYPE)
        .ok_or_else(|| AppError::Validation("missing ms-chap-challenge attribute".to_string()))?;
    if challenge.len() != 16 {
        return Err(AppError::Validation(
            "ms-chap-challenge attribute must be 16 bytes".to_string(),
        ));
    }

    let response = lookup_vendor_specific(packet, MS_CHAP2_RESPONSE_VENDOR_TYPE)
        .ok_or_else(|| AppError::Validation("missing ms-chap2-response attribute".to_string()))?;
    if response.len() != 50 {
        return Err(AppError::Validation(
            "ms-chap2-response attribute must be 50 bytes".to_string(),
        ));
    }

    Ok(RadiusMschapV2AuthRequest {
        source_ip: source_ip.to_string(),
        username,
        ident: response[0],
        peer_challenge: response[2..18].to_vec(),
        reserved: response[18..26].to_vec(),
        nt_response: response[26..50].to_vec(),
        authenticator_challenge: challenge,
    })
}

pub fn build_access_response(request: &Packet, result: &RadiusPapAuthResult) -> AppResult<Vec<u8>> {
    let code = match result.decision {
        RadiusAccessDecision::Accept => Code::AccessAccept,
        RadiusAccessDecision::Reject => Code::AccessReject,
    };

    let mut response = request.make_response_packet(code);

    if matches!(result.decision, RadiusAccessDecision::Reject) {
        if let Some(reason) = result.rejection_reason.as_deref() {
            rfc2865::add_reply_message(&mut response, reason);
        }
    } else {
        for attribute in &result.reply_attributes.attributes {
            add_reply_attribute(&mut response, attribute)?;
        }

        if let Some(success) = &result.mschapv2_success {
            add_vendor_bytes_attribute(
                &mut response,
                MS_CHAP2_SUCCESS_VENDOR_TYPE,
                &[[success.ident].as_slice(), success.message.as_bytes()].concat(),
            );
        }
    }

    if request_has_message_authenticator(request) {
        rfc2869::add_message_authenticator(&mut response, &[0; 16]);
        return finalize_response_with_message_authenticator(request, &response);
    }

    response
        .encode()
        .map_err(|error| AppError::Validation(format!("failed to encode access response: {error}")))
}

pub fn validate_message_authenticator(
    bytes: &[u8],
    packet: &Packet,
    required: bool,
) -> AppResult<()> {
    let Some(offset) = locate_message_authenticator(bytes) else {
        if required {
            return Err(AppError::Validation(
                "missing Message-Authenticator attribute".to_string(),
            ));
        }
        return Ok(());
    };

    let expected = compute_message_authenticator(bytes, packet.get_secret())?;
    let actual = &bytes[offset..offset + 16];
    if actual != expected.as_slice() {
        return Err(AppError::Validation(
            "invalid Message-Authenticator attribute".to_string(),
        ));
    }

    Ok(())
}

pub fn extract_accounting_request(
    packet: &Packet,
    source_ip: &str,
) -> AppResult<RadiusAccountingRequest> {
    let username = rfc2865::lookup_user_name(packet)
        .transpose()
        .map_err(|error| AppError::Validation(format!("invalid user-name attribute: {error}")))?
        .ok_or_else(|| AppError::Validation("missing user-name attribute".to_string()))?;

    let acct_session_id = rfc2866::lookup_acct_session_id(packet)
        .transpose()
        .map_err(|error| {
            AppError::Validation(format!("invalid acct-session-id attribute: {error}"))
        })?
        .ok_or_else(|| AppError::Validation("missing acct-session-id attribute".to_string()))?;

    let status_type = rfc2866::lookup_acct_status_type(packet)
        .transpose()
        .map_err(|error| {
            AppError::Validation(format!("invalid acct-status-type attribute: {error}"))
        })?
        .ok_or_else(|| AppError::Validation("missing acct-status-type attribute".to_string()))
        .and_then(map_status_type)?;

    let framed_ip_address = rfc2865::lookup_framed_ip_address(packet)
        .transpose()
        .map_err(|error| {
            AppError::Validation(format!("invalid framed-ip-address attribute: {error}"))
        })?
        .map(|address| address.to_string());

    let calling_station_id = rfc2865::lookup_calling_station_id(packet)
        .transpose()
        .map_err(|error| {
            AppError::Validation(format!("invalid calling-station-id attribute: {error}"))
        })?;

    let session_time_seconds = rfc2866::lookup_acct_session_time(packet)
        .transpose()
        .map_err(|error| {
            AppError::Validation(format!("invalid acct-session-time attribute: {error}"))
        })?
        .map(i64::from);

    let input_octets = rfc2866::lookup_acct_input_octets(packet)
        .transpose()
        .map_err(|error| {
            AppError::Validation(format!("invalid acct-input-octets attribute: {error}"))
        })?
        .map(i64::from);

    let output_octets = rfc2866::lookup_acct_output_octets(packet)
        .transpose()
        .map_err(|error| {
            AppError::Validation(format!("invalid acct-output-octets attribute: {error}"))
        })?
        .map(i64::from);

    let terminate_cause = rfc2866::lookup_acct_terminate_cause(packet)
        .transpose()
        .map_err(|error| {
            AppError::Validation(format!("invalid acct-terminate-cause attribute: {error}"))
        })?
        .map(|value| value.to_string());

    Ok(RadiusAccountingRequest {
        source_ip: source_ip.to_string(),
        username,
        radius_identity: None,
        acct_session_id,
        status_type,
        framed_ip_address,
        calling_station_id,
        session_time_seconds,
        input_octets,
        output_octets,
        terminate_cause,
        occurred_at: None,
        raw_attributes_json: None,
    })
}

pub fn build_accounting_response(request: &Packet) -> AppResult<Vec<u8>> {
    request
        .make_response_packet(Code::AccountingResponse)
        .encode()
        .map_err(|error| {
            AppError::Validation(format!("failed to encode accounting response: {error}"))
        })
}

fn add_reply_attribute(packet: &mut Packet, attribute: &RadiusReplyAttribute) -> AppResult<()> {
    match attribute.name {
        "Mikrotik-Group" => {
            rfc2865::add_vsa_attribute(
                packet,
                &StringVSA::new(
                    MIKROTIK_VENDOR_ID,
                    MIKROTIK_GROUP_VENDOR_TYPE,
                    &attribute.value,
                ),
            );
            Ok(())
        }
        "Framed-IP-Address" => {
            let address = Ipv4Addr::from_str(&attribute.value).map_err(|_| {
                AppError::Validation(format!(
                    "invalid framed-ip-address reply value: {}",
                    attribute.value
                ))
            })?;
            rfc2865::add_framed_ip_address(packet, &address);
            Ok(())
        }
        "Framed-Pool" => {
            rfc2869::add_framed_pool(packet, &attribute.value);
            Ok(())
        }
        "MS-CHAP2-Success" => {
            add_vendor_bytes_attribute(
                packet,
                MS_CHAP2_SUCCESS_VENDOR_TYPE,
                attribute.value.as_bytes(),
            );
            Ok(())
        }
        other => Err(AppError::Validation(format!(
            "unsupported radius reply attribute: {other}"
        ))),
    }
}

fn map_status_type(value: u32) -> AppResult<RadiusAccountingStatusType> {
    match value {
        rfc2866::ACCT_STATUS_TYPE_START => Ok(RadiusAccountingStatusType::Start),
        rfc2866::ACCT_STATUS_TYPE_INTERIM_UPDATE => Ok(RadiusAccountingStatusType::InterimUpdate),
        rfc2866::ACCT_STATUS_TYPE_STOP => Ok(RadiusAccountingStatusType::Stop),
        rfc2866::ACCT_STATUS_TYPE_ACCOUNTING_ON => Ok(RadiusAccountingStatusType::AccountingOn),
        rfc2866::ACCT_STATUS_TYPE_ACCOUNTING_OFF => Ok(RadiusAccountingStatusType::AccountingOff),
        _ => Err(AppError::Validation(format!(
            "unsupported acct-status-type value: {value}"
        ))),
    }
}

fn locate_message_authenticator(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 20 {
        return None;
    }

    let mut index = 20usize;
    while index + 2 <= bytes.len() {
        let attr_type = bytes[index];
        let attr_len = bytes[index + 1] as usize;
        if !(2..=255).contains(&attr_len) || index + attr_len > bytes.len() {
            return None;
        }
        if attr_type == MESSAGE_AUTHENTICATOR_TYPE
            && bytes[index + 1] == MESSAGE_AUTHENTICATOR_ATTRIBUTE_LENGTH
        {
            return Some(index + 2);
        }
        index += attr_len;
    }

    None
}

fn request_has_message_authenticator(packet: &Packet) -> bool {
    rfc2869::lookup_message_authenticator(packet).is_some()
}

fn finalize_response_with_message_authenticator(
    request: &Packet,
    response: &Packet,
) -> AppResult<Vec<u8>> {
    let mut encoded = response.encode().map_err(|error| {
        AppError::Validation(format!("failed to encode access response: {error}"))
    })?;
    let Some(offset) = locate_message_authenticator(&encoded) else {
        return Err(AppError::Validation(
            "response message-authenticator attribute is missing".to_string(),
        ));
    };

    let mut hmac_input = encoded.clone();
    hmac_input[4..20].copy_from_slice(request.get_authenticator());
    hmac_input[offset..offset + 16].fill(0);
    let digest = hmac_md5(response.get_secret(), &hmac_input);
    encoded[offset..offset + 16].copy_from_slice(&digest);

    let response_authenticator = md5_compute(
        [
            &encoded[..4],
            request.get_authenticator().as_slice(),
            &encoded[20..],
            response.get_secret().as_slice(),
        ]
        .concat(),
    );
    encoded[4..20].copy_from_slice(&response_authenticator.0);
    Ok(encoded)
}

fn lookup_vendor_specific(packet: &Packet, vendor_type: u8) -> Option<Vec<u8>> {
    packet.lookup_all(26).into_iter().find_map(|attribute| {
        let bytes = attribute.encode_bytes();
        if bytes.len() < 6 {
            return None;
        }
        if bytes[..4] != MICROSOFT_VENDOR_ID {
            return None;
        }
        if bytes[4] != vendor_type {
            return None;
        }

        let vendor_length = bytes[5] as usize;
        if vendor_length < 2 || 4 + vendor_length > bytes.len() {
            return None;
        }

        Some(bytes[6..4 + vendor_length].to_vec())
    })
}

fn add_vendor_bytes_attribute(packet: &mut Packet, vendor_type: u8, value: &[u8]) {
    let payload = [
        &MICROSOFT_VENDOR_ID[..],
        &[vendor_type, (value.len() + 2) as u8],
        value,
    ]
    .concat();
    packet.add(AVP::from_bytes(26, &payload));
}

fn compute_message_authenticator(bytes: &[u8], secret: &[u8]) -> AppResult<Vec<u8>> {
    let offset = locate_message_authenticator(bytes).ok_or_else(|| {
        AppError::Validation("missing Message-Authenticator attribute".to_string())
    })?;

    let mut normalized = bytes.to_vec();
    normalized[offset..offset + 16].fill(0);
    Ok(hmac_md5(secret, &normalized))
}

fn hmac_md5(key: &[u8], data: &[u8]) -> Vec<u8> {
    const BLOCK_SIZE: usize = 64;

    let mut normalized_key = if key.len() > BLOCK_SIZE {
        md5_compute(key).0.to_vec()
    } else {
        key.to_vec()
    };
    normalized_key.resize(BLOCK_SIZE, 0);

    let mut inner_pad = vec![0x36; BLOCK_SIZE];
    let mut outer_pad = vec![0x5c; BLOCK_SIZE];
    for (idx, byte) in normalized_key.iter().enumerate() {
        inner_pad[idx] ^= byte;
        outer_pad[idx] ^= byte;
    }

    let inner_hash = md5_compute([inner_pad, data.to_vec()].concat());
    md5_compute([outer_pad, inner_hash.0.to_vec()].concat())
        .0
        .to_vec()
}

#[cfg(test)]
mod tests {
    use super::{
        build_access_response, build_accounting_response, decode_request,
        extract_accounting_request, extract_chap_auth_request, extract_mschapv2_auth_request,
        extract_pap_auth_request, validate_message_authenticator, RadiusAccessDecision,
    };
    use crate::services::radius_service::{
        RadiusChapAuthRequest, RadiusMschapV2AuthRequest, RadiusPapAuthResult,
        RadiusReplyAttributes,
    };
    use radius::core::code::Code;
    use radius::core::packet::Packet;
    use radius::core::{rfc2865, rfc2866, rfc2869};
    use std::net::Ipv4Addr;

    const SHARED_SECRET: &[u8] = b"radius-secret";

    fn add_valid_message_authenticator(packet: &mut Packet) -> Vec<u8> {
        rfc2869::add_message_authenticator(packet, &[0; 16]);
        let mut encoded = packet.encode().expect("encode packet");
        let start = encoded
            .windows(2)
            .position(|window| window[0] == 80 && window[1] == 18)
            .map(|index| index + 2)
            .expect("message-authenticator attr");
        let digest = super::compute_message_authenticator(&encoded, SHARED_SECRET)
            .expect("compute message authenticator");
        encoded[start..start + 16].copy_from_slice(&digest);
        encoded
    }

    fn response_message_authenticator(bytes: &[u8]) -> Option<Vec<u8>> {
        let start = bytes
            .windows(2)
            .position(|window| window[0] == 80 && window[1] == 18)
            .map(|index| index + 2)?;
        Some(bytes[start..start + 16].to_vec())
    }

    #[test]
    fn packet_adapter_extracts_pap_request_and_builds_access_accept() {
        let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "alice");
        rfc2865::add_user_password(&mut request, b"secret-1").expect("user password attribute");
        let request_bytes = request.encode().expect("encode request");
        let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");

        let auth_request =
            extract_pap_auth_request(&decoded, "203.0.113.10").expect("extract pap request");
        assert_eq!(auth_request.username, "alice");
        assert_eq!(auth_request.password, "secret-1");

        let response_bytes = build_access_response(
            &decoded,
            &RadiusPapAuthResult {
                decision: RadiusAccessDecision::Accept,
                rejection_reason: None,
                reply_attributes: RadiusReplyAttributes {
                    attributes: vec![
                        crate::services::radius_service::RadiusReplyAttribute {
                            name: "Mikrotik-Group",
                            value: "basic".into(),
                        },
                        crate::services::radius_service::RadiusReplyAttribute {
                            name: "Framed-IP-Address",
                            value: "10.10.10.2".into(),
                        },
                        crate::services::radius_service::RadiusReplyAttribute {
                            name: "Framed-Pool",
                            value: "pool-a".into(),
                        },
                    ],
                },
                mschapv2_success: None,
            },
        )
        .expect("build access accept");

        let response = Packet::decode(&response_bytes, SHARED_SECRET).expect("decode response");
        assert_eq!(response.get_code(), Code::AccessAccept);
        assert_eq!(
            rfc2865::lookup_framed_ip_address(&response)
                .transpose()
                .expect("framed ip attribute"),
            Some(Ipv4Addr::new(10, 10, 10, 2))
        );
        assert_eq!(
            rfc2869::lookup_framed_pool(&response)
                .transpose()
                .expect("framed pool attribute"),
            Some("pool-a".to_string())
        );
        let vendor_value = response
            .lookup(26)
            .expect("mikrotik group vsa should exist")
            .encode_bytes();
        assert_eq!(&vendor_value[..6], &[0, 0, 58, 140, 3, 7]);
        assert_eq!(&vendor_value[6..], b"basic");
    }

    #[test]
    fn packet_adapter_extracts_chap_request() {
        let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "alice");
        rfc2865::add_chap_challenge(&mut request, b"0123456789abcdef");
        rfc2865::add_chap_password(
            &mut request,
            &[7_u8, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        );
        let request_bytes = request.encode().expect("encode request");
        let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");

        let auth_request =
            extract_chap_auth_request(&decoded, "203.0.113.10").expect("extract chap");

        assert_eq!(
            auth_request,
            RadiusChapAuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "alice".into(),
                chap_identifier: 7,
                challenge: b"0123456789abcdef".to_vec(),
                response: vec![1; 16],
            }
        );
    }

    #[test]
    fn packet_adapter_extracts_mschapv2_request() {
        let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "User");
        super::add_vendor_bytes_attribute(
            &mut request,
            super::MS_CHAP_CHALLENGE_VENDOR_TYPE,
            &[
                0x5B, 0x5D, 0x7C, 0x7D, 0x7B, 0x3F, 0x2F, 0x3E, 0x3C, 0x2C, 0x60, 0x21, 0x32, 0x26,
                0x26, 0x28,
            ],
        );
        super::add_vendor_bytes_attribute(
            &mut request,
            super::MS_CHAP2_RESPONSE_VENDOR_TYPE,
            &[
                7, 0, 0x21, 0x40, 0x23, 0x24, 0x25, 0x5E, 0x26, 0x2A, 0x28, 0x29, 0x5F, 0x2B, 0x3A,
                0x33, 0x7C, 0x7E, 0, 0, 0, 0, 0, 0, 0, 0, 0x82, 0x30, 0x9E, 0xCD, 0x8D, 0x70, 0x8B,
                0x5E, 0xA0, 0x8F, 0xAA, 0x39, 0x81, 0xCD, 0x83, 0x54, 0x42, 0x33, 0x11, 0x4A, 0x3D,
                0x85, 0xD6, 0xDF,
            ],
        );
        let request_bytes = request.encode().expect("encode request");
        let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");

        let auth_request =
            extract_mschapv2_auth_request(&decoded, "203.0.113.10").expect("extract mschapv2");

        assert_eq!(
            auth_request,
            RadiusMschapV2AuthRequest {
                source_ip: "203.0.113.10".into(),
                username: "User".into(),
                ident: 7,
                peer_challenge: vec![
                    0x21, 0x40, 0x23, 0x24, 0x25, 0x5E, 0x26, 0x2A, 0x28, 0x29, 0x5F, 0x2B, 0x3A,
                    0x33, 0x7C, 0x7E,
                ],
                reserved: vec![0; 8],
                nt_response: vec![
                    0x82, 0x30, 0x9E, 0xCD, 0x8D, 0x70, 0x8B, 0x5E, 0xA0, 0x8F, 0xAA, 0x39, 0x81,
                    0xCD, 0x83, 0x54, 0x42, 0x33, 0x11, 0x4A, 0x3D, 0x85, 0xD6, 0xDF,
                ],
                authenticator_challenge: vec![
                    0x5B, 0x5D, 0x7C, 0x7D, 0x7B, 0x3F, 0x2F, 0x3E, 0x3C, 0x2C, 0x60, 0x21, 0x32,
                    0x26, 0x26, 0x28,
                ],
            }
        );
    }

    #[test]
    fn packet_adapter_validates_message_authenticator_when_present() {
        let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "alice");
        let encoded = add_valid_message_authenticator(&mut request);
        let decoded = decode_request(&encoded, SHARED_SECRET).expect("decode request");

        validate_message_authenticator(&encoded, &decoded, true)
            .expect("message authenticator should validate");
    }

    #[test]
    fn packet_adapter_rejects_missing_message_authenticator_when_required() {
        let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "alice");
        let encoded = request.encode().expect("encode request");
        let decoded = decode_request(&encoded, SHARED_SECRET).expect("decode request");

        let error = validate_message_authenticator(&encoded, &decoded, true)
            .expect_err("message authenticator should be required");

        assert!(error.to_string().contains("missing Message-Authenticator"));
    }

    #[test]
    fn packet_adapter_allows_missing_message_authenticator_when_not_required() {
        let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "alice");
        let encoded = request.encode().expect("encode request");
        let decoded = decode_request(&encoded, SHARED_SECRET).expect("decode request");

        validate_message_authenticator(&encoded, &decoded, false)
            .expect("message authenticator should be optional");
    }

    #[test]
    fn packet_adapter_builds_access_reject_with_reply_message() {
        let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "alice");
        let response_bytes = build_access_response(
            &request,
            &RadiusPapAuthResult {
                decision: RadiusAccessDecision::Reject,
                rejection_reason: Some("invalid_password".into()),
                reply_attributes: RadiusReplyAttributes { attributes: vec![] },
                mschapv2_success: None,
            },
        )
        .expect("build access reject");

        let response = Packet::decode(&response_bytes, SHARED_SECRET).expect("decode response");
        assert_eq!(response.get_code(), Code::AccessReject);
        assert_eq!(
            rfc2865::lookup_reply_message(&response)
                .transpose()
                .expect("reply message attribute"),
            Some("invalid_password".to_string())
        );
    }

    #[test]
    fn packet_adapter_preserves_message_authenticator_in_access_response_when_request_has_it() {
        let mut request = Packet::new(Code::AccessRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "alice");
        let request_bytes = add_valid_message_authenticator(&mut request);
        let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");

        let response_bytes = build_access_response(
            &decoded,
            &RadiusPapAuthResult {
                decision: RadiusAccessDecision::Accept,
                rejection_reason: None,
                reply_attributes: RadiusReplyAttributes { attributes: vec![] },
                mschapv2_success: None,
            },
        )
        .expect("build access accept");

        let response = Packet::decode(&response_bytes, SHARED_SECRET).expect("decode response");
        assert_eq!(response.get_code(), Code::AccessAccept);
        let actual = response_message_authenticator(&response_bytes)
            .expect("response should include message-authenticator");
        assert_ne!(actual, vec![0; 16]);
        assert!(Packet::is_authentic_response(
            &response_bytes,
            &request_bytes,
            SHARED_SECRET
        ));
    }

    #[test]
    fn packet_adapter_extracts_accounting_request_and_builds_accounting_response() {
        let mut request = Packet::new(Code::AccountingRequest, SHARED_SECRET);
        rfc2865::add_user_name(&mut request, "alice");
        rfc2866::add_acct_status_type(&mut request, rfc2866::ACCT_STATUS_TYPE_INTERIM_UPDATE);
        rfc2866::add_acct_session_id(&mut request, "sess-1");
        rfc2866::add_acct_session_time(&mut request, 120);
        rfc2866::add_acct_input_octets(&mut request, 1024);
        rfc2866::add_acct_output_octets(&mut request, 2048);
        rfc2865::add_framed_ip_address(&mut request, &Ipv4Addr::new(10, 10, 10, 2));
        rfc2865::add_calling_station_id(&mut request, "AA:BB:CC:DD:EE:FF");
        let request_bytes = request.encode().expect("encode accounting request");
        let decoded = decode_request(&request_bytes, SHARED_SECRET).expect("decode request");

        let accounting_request =
            extract_accounting_request(&decoded, "203.0.113.10").expect("extract accounting");
        assert_eq!(accounting_request.username, "alice");
        assert_eq!(accounting_request.acct_session_id, "sess-1");
        assert_eq!(
            accounting_request.status_type,
            crate::models::RadiusAccountingStatusType::InterimUpdate
        );
        assert_eq!(accounting_request.input_octets, Some(1024));
        assert_eq!(accounting_request.output_octets, Some(2048));
        assert_eq!(accounting_request.session_time_seconds, Some(120));

        let response_bytes =
            build_accounting_response(&decoded).expect("build accounting response");
        let response = Packet::decode(&response_bytes, SHARED_SECRET).expect("decode response");
        assert_eq!(response.get_code(), Code::AccountingResponse);
    }
}
