use bluer::{Uuid, UuidExt};
use sdp_xml::Tag;
use sdp_xml_reader::{self, parse_sdp_xml};
use hid_device_id::bluetooth::attribute_id;
use std::fmt::{self, Display, Formatter};

use crate::{Configuration, LanguageCode};
use crate::hid::{self, ClassDescriptor, LanguageBase};

/// Error type for reading configurations
#[derive(Debug)]
pub enum Error {
    XmlParseError(sdp_xml_reader::Error),
    ExpectedRecord(Tag),
    ExpectedAttribute(Tag),
    ExpectedSequence(u16, Tag),
    ExpectedBoolean(u16, Tag),
    ExpectedUInt8(u16, Tag),
    ExpectedUInt16(u16, Tag),
    ExpectedText(u16, Tag),
    ExpectedUuid(u16, Tag),
    UnexpectedSequenceLen { attribute: u16, expected: usize, actual: usize },
    UnexpectedUuid { attribute: u16, expected: Uuid, actual: Uuid },
    DuplicateValue(u16),
    DuplicateAttribute(u16, &'static str),
    DuplicateDescriptorId,
    DuplicateDescriptorText,
    MissingRecord(&'static str),
    UnexpectedTag(Tag),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::XmlParseError(e) =>
                write!(f, "XML parse error: {}", e),
            Self::ExpectedRecord(tag) =>
                write!(f, "expected record, received {}", tag.name()),
            Self::ExpectedAttribute(tag) =>
                write!(f, "expected attribute, received {}", tag.name()),
            Self::ExpectedSequence(attribute, tag) =>
                write!(f, "in attribute 0x{:04x}: expected sequence, received {}",
                       attribute, tag.name()),
            Self::ExpectedBoolean(attribute, tag) =>
                write!(f, "in attribute 0x{:04x}: expected boolean, received {}",
                       attribute, tag.name()),
            Self::ExpectedUInt8(attribute, tag) =>
                write!(f, "in attribute 0x{:04x}: expected uint8, received {}",
                       attribute, tag.name()),
            Self::ExpectedUInt16(attribute, tag) =>
                write!(f, "in attribute 0x{:04x}: expected uint16, received {}",
                       attribute, tag.name()),
            Self::ExpectedText(attribute, tag) =>
                write!(f, "in attribute 0x{:04x}: expected text, received {}", attribute, tag.name()),
            Self::ExpectedUuid(attribute, tag) =>
                write!(f, "in attribute 0x{:04x}: expected uuid, received {}", attribute, tag.name()),
            Self::UnexpectedSequenceLen { attribute, expected, actual } =>
                write!(f, "in attribute 0x{:04x}: expected sequence of length {}, received sequence of length {}",
                       attribute, expected, actual),
            Self::UnexpectedUuid { attribute, expected, actual } =>
                write!(f, "in attribute 0x{:04x}: expected uuid {}, received {}",
                       attribute, expected, actual),
            Self::DuplicateValue(attribute) =>
                write!(f, "in attribute 0x{:04x}: unexpected duplicate value", attribute),
            Self::DuplicateAttribute(id, name) =>
                write!(f, "duplicate attribute {} (0x{:04x})", name, id),
            Self::DuplicateDescriptorId =>
                write!(f, "unexpected duplicate descriptor ID"),
            Self::DuplicateDescriptorText =>
                write!(f, "unexpected duplicate descriptor text"),
            Self::MissingRecord(name) =>
                write!(f, "missing record {}", name),
            Self::UnexpectedTag(tag) =>
                write!(f, "unexpected tag {}", tag.name()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;


/// Match the given tag as a sequence, or return an error.
fn expect_boolean(attribute: u16, tag: Tag) -> Result<bool> {
    match tag {
        Tag::Boolean(value) => Ok(value),
        _ => Err(Error::ExpectedBoolean(attribute, tag)),
    }
}

/// Match the given tag as a UInt8, or return an error.
fn expect_uint8(attribute: u16, tag: Tag) -> Result<u8> {
    match tag {
        Tag::UInt8(value) => Ok(value),
        _ => Err(Error::ExpectedUInt8(attribute, tag)),
    }
}

/// Match the given tag as a UInt16, or return an error.
fn expect_uint16(attribute: u16, tag: Tag) -> Result<u16> {
    match tag {
        Tag::UInt16(value) => Ok(value),
        _ => Err(Error::ExpectedUInt16(attribute, tag)),
    }
}

/// Match the given tag as Text, or return an error.
fn expect_text(attribute: u16, tag: Tag) -> Result<String> {
    match tag {
        Tag::Text(value) => Ok(value),
        _ => Err(Error::ExpectedText(attribute, tag)),
    }
}


/// Match the given tag as a sequence, or return an error.
fn expect_sequence(attribute: u16, tag: Tag) -> Result<Vec<Tag>> {
    match tag {
        Tag::Sequence(children) => Ok(children),
        _ => Err(Error::ExpectedSequence(attribute, tag)),
    }
}

/// Return an error if the sequence does not have the given length.
fn expect_len(attribute: u16, sequence: &Vec<Tag>, len: usize) -> Result<()> {
    if sequence.len() != len {
        Err(Error::UnexpectedSequenceLen {
            attribute,
            expected: len,
            actual: sequence.len(),
        })
    } else {
        Ok(())
    }
}

/// Match the given tag as the given Uuid, or return an error.
fn expect_uuid(attribute: u16, tag: Tag, expected: Uuid) -> Result<()> {
    match tag {
        Tag::Uuid(actual) => {
            if expected == actual {
                Ok(())
            } else {
                Err(Error::UnexpectedUuid { attribute, expected, actual })
            }
        },
        _ => Err(Error::ExpectedUuid(attribute, tag)),
    }
}


/// Initialize the given attribute with the given value, if the attribute has no prior value. If
/// the attribute is already initialized, return an error using the given attribute ID and name.
fn try_initialize_attribute<T>(
    attribute: &mut Option<T>,
    value: T,
    attribute_id: u16,
    attribute_name: &'static str
) -> Result<()> {
    if attribute.is_some() {
        return Err(Error::DuplicateAttribute(attribute_id, attribute_name));
    }
    attribute.replace(value);
    Ok(())
}

/// Initialize an option with the given value, if it is uninitialized. Return an error otherwise.
fn try_initialize<T>(attribute: u16, dest: &mut Option<T>, value: T) -> Result<()> {
    if dest.is_some() {
        return Err(Error::DuplicateValue(attribute));
    }
    dest.replace(value);
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct PartialConfiguration {
    primary_language: Option<u16>,
    encoding: Option<u16>,
    service_name: Option<String>,
    service_description: Option<String>,
    provider_name: Option<String>,
    version: Option<u16>,

    hid_device_subclass: Option<u8>,
    hid_country_code: Option<u8>,
    hid_virtual_cable: Option<bool>,
    hid_reconnect_initiate: Option<bool>,
    hid_descriptor_list: Vec<ClassDescriptor>,
    hid_lang_base_id_list: Vec<LanguageBase>,
    hid_battery_power: Option<bool>,
    hid_remote_wake: Option<bool>,
    hid_supervision_timeout: Option<u16>,
    hid_normally_connectable: Option<bool>,
    hid_boot_device: Option<bool>,
    hid_ssr_host_max_latency: Option<u16>,
    hid_ssr_host_min_timeout: Option<u16>,
}

impl PartialConfiguration {
    pub fn from_sdp_xml(xml: &[u8]) -> Result<Self> {
        let mut partial_configuration = Self::default();

        let maybe_record = parse_sdp_xml(xml)
            .map_err(|e| Error::XmlParseError(e))?;
        let maybe_attributes = match maybe_record {
            Tag::Record(attributes) => attributes,
            _ => {
                return Err(Error::ExpectedRecord(maybe_record));
            },
        };
        // Convert the list of (maybe) attributes to a list of attributes, or, if there is a
        // non-attribute, to an error.
        let attributes_res: Result<Vec<(u16, Tag)>> = maybe_attributes.into_iter()
            .map(|tag| match tag { 
                Tag::Attribute(id, child) => Ok((id, *child)),
                _ => Err(Error::ExpectedAttribute(tag)),
            }).collect();
        // If an error occured during the conversion process, return it.
        let attributes = match attributes_res {
            Ok(a) => a,
            Err(e) => {
                return Err(e);
            },
        };
        for (id, child) in attributes {
            match id {
                attribute_id::LANGUAGE_BASE_ATTRIBUTE_ID_LIST => {
                    let mut language_base_attribute_id = expect_sequence(id, child)?;
                    expect_len(id, &language_base_attribute_id, 3)?;
                    let lang = expect_uint16(id, language_base_attribute_id.remove(0))?;
                    let encoding = expect_uint16(id, language_base_attribute_id.remove(0))?;
                    try_initialize_attribute( 
                        &mut partial_configuration.primary_language, lang,
                        id, "Language Base Attribute ID List")?;
                    try_initialize_attribute( 
                        &mut partial_configuration.encoding, encoding,
                        id, "Language Base Attribute ID List")?;
                },
                attribute_id::SERVICE_NAME => {
                    let text = expect_text(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.service_name, text,
                        id, "Service Name")?;
                    // Duplicate attribute "Service Name" (0x1124)
                },
                attribute_id::SERVICE_DESCRIPTION => {
                    let text = expect_text(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.service_description, text,
                        id, "Service Description")?;
                },
                attribute_id::PROVIDER_NAME => {
                    let text = expect_text(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.provider_name, text,
                        id, "Provider Name")?;
                },
                attribute_id::BLUETOOTH_PROFILE_DESCRIPTOR_LIST => {
                    let mut seq_1_children = expect_sequence(id, child)?;
                    expect_len(id, &seq_1_children, 1)?;
                    let mut seq_2_children = expect_sequence(id, seq_1_children.remove(0))?;
                    expect_len(id, &seq_2_children, 2)?;
                    // TODO
                    // Should be a sequence containing a sequence containing
                    // uuid = 1124
                    // value = some version, like 0x0101.
                    expect_uuid(id, seq_2_children.remove(0), Uuid::from_u16(0x1124))?;
                    let version = expect_uint16(id, seq_2_children.remove(0))?;
                    try_initialize_attribute( 
                        &mut partial_configuration.version, version,
                        id, "Profile Descriptor List")?;
                },
                attribute_id::hid::HID_DEVICE_SUBCLASS => {
                    let value = expect_uint8(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_device_subclass, value,
                        id, "HID Device Subclass")?;
                },
                attribute_id::hid::HID_COUNTRY_CODE => {
                    let value = expect_uint8(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_country_code, value,
                        id, "HID Country Code")?;
                },
                attribute_id::hid::HID_VIRTUAL_CABLE => {
                    let value = expect_boolean(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_virtual_cable, value,
                        id, "HID Virtual Cable")?;
                },
                attribute_id::hid::HID_RECONNECT_INITIATE => {
                    let value = expect_boolean(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_reconnect_initiate, value,
                        id, "HID Reconnect Initiate")?;
                },
                attribute_id::hid::HID_DESCRIPTOR_LIST => {
                    let maybe_descriptors = expect_sequence(id, child)?;
                    for maybe_descriptor in maybe_descriptors {
                        // Each descriptor is a sequence containing an ID (u8) and text.
                        let descriptor = expect_sequence(id, maybe_descriptor)?;
                        let mut descriptor_type = None;
                        let mut descriptor_value = None;
                        // Read each element in the descriptor, searching for an ID and descriptor
                        // text.
                        for element in descriptor {
                            match element {
                                Tag::UInt8(v) => {
                                    try_initialize(id, &mut descriptor_type, v)
                                        .map_err(|_| Error::DuplicateDescriptorId)?;
                                },
                                Tag::Text(v) => {
                                    try_initialize(id, &mut descriptor_value, v.into_bytes())
                                        .map_err(|_| Error::DuplicateDescriptorText)?;
                                },
                                Tag::RawText(v) => {
                                    try_initialize(id, &mut descriptor_value, v)
                                        .map_err(|_| Error::DuplicateDescriptorText)?;
                                },
                                _ => {
                                    return Err(Error::UnexpectedTag(element));
                                },
                            };
                        }
                        // Convert the optional descriptor type and value into a concrete class
                        // descriptor.
                        let class_descriptor = match (descriptor_type, descriptor_value) {
                            (Some(t), Some(v)) => ClassDescriptor(t, v),
                            (None, _) => {
                                return Err(Error::MissingRecord("descriptor id"));
                            },
                            (Some(_), None) => {
                                return Err(Error::MissingRecord("descriptor value"));
                            },
                        };
                        // Add the new class descriptor.
                        partial_configuration.hid_descriptor_list.push(class_descriptor);
                    }
                },
                attribute_id::hid::HID_LANG_BASE_ATTRIBUTE => {
                    let lang_base_id_list = expect_sequence(id, child)?;
                    for maybe_lang_base_id in lang_base_id_list {
                        let mut lang_base_id = expect_sequence(id, maybe_lang_base_id)?;
                        expect_len(id, &lang_base_id, 2)?;
                        let lang = expect_uint16(id, lang_base_id.remove(0))?;
                        let base = expect_uint16(id, lang_base_id.remove(0))?;
                        partial_configuration.hid_lang_base_id_list.push(LanguageBase {
                            language: lang,
                            base,
                        });
                    }
                },
                attribute_id::hid::HID_BATTERY_POWER => {
                    let value = expect_boolean(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_battery_power, value,
                        id, "HID Battery Power")?;
                },
                attribute_id::hid::HID_REMOTE_WAKE => {
                    let value = expect_boolean(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_remote_wake, value,
                        id, "HID Remote Wake")?;
                },
                attribute_id::hid::HID_SUPERVISION_TIMEOUT => {
                    let value = expect_uint16(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_supervision_timeout, value,
                        id, "HID Supervision Timeout")?;
                },
                attribute_id::hid::HID_NORMALLY_CONNECTABLE => {
                    let value = expect_boolean(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_normally_connectable, value,
                        id, "HID Normally Connectable")?;
                },
                attribute_id::hid::HID_BOOT_DEVICE => {
                    let value = expect_boolean(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_boot_device, value,
                        id, "HID Boot Device")?;
                },
                attribute_id::hid::HID_SSR_HOST_MAX_LATENCY => {
                    let value = expect_uint16(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_ssr_host_max_latency, value,
                        id, "HID SSR Host Max Latency")?;
                },
                attribute_id::hid::HID_SSR_HOST_MIN_TIMEOUT => {
                    let value = expect_uint16(id, child)?;
                    try_initialize_attribute( 
                        &mut partial_configuration.hid_ssr_host_min_timeout, value,
                        id, "HID SSR Host Min Timeout")?;
                },
                // Ignore other attributes.
                _ => (),
            }
        }
        Ok(partial_configuration)
    }
}

impl TryFrom<PartialConfiguration> for Configuration {
    type Error = Error;

    fn try_from(partial_configuration: PartialConfiguration) -> Result<Self> {
        // Parse primary language from base and HID configuration data.
        let iso_code = partial_configuration.primary_language
            .ok_or(Error::MissingRecord("primary language"))?;
        let hid_code = partial_configuration.hid_lang_base_id_list.first()
            .ok_or(Error::MissingRecord("HID language"))?
            .language;
        // Create Configuration
        Ok(Configuration {
            primary_language: LanguageCode { iso_code, hid_code },
            encoding: partial_configuration.encoding
                .ok_or(Error::MissingRecord("encoding"))?,
            service_name: partial_configuration.service_name,
            service_description: partial_configuration.service_description,
            provider_name: partial_configuration.provider_name,
            version: partial_configuration.version
                .ok_or(Error::MissingRecord("version"))?,
            hid: hid::Configuration {
                device_subclass: partial_configuration.hid_device_subclass
                                 .ok_or(Error::MissingRecord("device subclass"))?,
                country_code: partial_configuration.hid_country_code
                                 .ok_or(Error::MissingRecord("country code"))?,
                virtual_cable: partial_configuration.hid_virtual_cable
                                 .ok_or(Error::MissingRecord("virtual cable"))?,
                reconnect_initiate: partial_configuration.hid_reconnect_initiate
                                 .ok_or(Error::MissingRecord("reconnect initiate"))?,
                class_descriptors: partial_configuration.hid_descriptor_list,
                additional_languages: partial_configuration.hid_lang_base_id_list,
                battery_power: partial_configuration.hid_battery_power,
                remote_wake: partial_configuration.hid_remote_wake,
                supervision_timeout: partial_configuration.hid_supervision_timeout,
                normally_connectable: partial_configuration.hid_normally_connectable,
                boot_device: partial_configuration.hid_boot_device
                                 .ok_or(Error::MissingRecord("boot device"))?,
                ssr_host_max_latency: partial_configuration.hid_ssr_host_max_latency,
                ssr_host_min_timeout: partial_configuration.hid_ssr_host_min_timeout,
            },
        })
    }
}

