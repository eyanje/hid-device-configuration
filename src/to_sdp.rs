use bluer::id::ServiceClass;
use sdp_xml::Tag;
use hid_device_id::bluetooth::{attribute_id, protocol, psm};
use uuid::Uuid;

use crate::{Configuration, hid};

// Unit = 625 microseconds for each duration.

// For later, consider a design where every attribute is a separate struct that implements some
// kind of automatic conversion to a tag.
// This way, we can correlate each setting's type to a potential attribute.

impl Configuration {
    pub fn to_sdp_tag(&self) -> Tag {
        // Supervision timeout: Optional. Default 2 seconds. Unit is 625
        // microseconds, one baseband slot.
        // Normally connectable: false because we are not always in page scan mode.
        // HID boot device:  Required for keyboards and mice. Also mandatory argument.

        let mut attributes = Vec::new();

        // Add service class ID list attribute

        attributes.push(Tag::attribute(
                attribute_id::SERVICE_CLASS_ID_LIST,
                [Uuid::from(ServiceClass::Hid),]));

        // Add protocol descriptor list (L2CAP:HIDControl -> HIDP)

        attributes.push(Tag::attribute(
                attribute_id::PROTOCOL_DESCRIPTOR_LIST,
                (
                    // Protocol Descriptor 0
                    (
                        protocol::L2CAP,
                        psm::HID_CONTROL,
                        ),
                        // Protocol Descriptor 1
                        (
                            protocol::HID_PROTOCOL,
                            ),
                            )));

        // Add browse group list (optional)

        attributes.push(Tag::attribute(
                attribute_id::BROWSE_GROUP_LIST,
                (Uuid::from(ServiceClass::PublicBrowseGroup),)));


        // Add primary base attribute ID

        attributes.push(Tag::attribute(
                attribute_id::LANGUAGE_BASE_ATTRIBUTE_ID_LIST,
                ( // Described in 5.1.8 of Bluetooth Core
                    self.primary_language.iso_code, // 0x656eu16, English, ISO 639:1988 (E/F)
                    self.encoding, // 0x006au16, // MIBEnum value for UTF-8, from IANA's database
                    0x0100u16, // Base ID: Primary Language
                               // This value is why the strings are offset from 0x0100.
                               // However, for the primary language, the offset must be 0x0100.
                               // Don't forget the u16.
                )));

        // Add additional protocol descriptor lists (L2CAP:HIDInterrupt -> HIDP)

        attributes.push(Tag::attribute(
                attribute_id::ADDITIONAL_PROTOCOL_DESCRIPTOR_LISTS,
                (
                    (
                        // Protocol Descriptor List 0
                        (
                            protocol::L2CAP,
                            psm::HID_INTERRUPT,
                            ),
                            // Protocol Descriptor 1
                            (
                                protocol::HID_PROTOCOL,
                                ),
                                ),
                                )));

        // Add the service name, if it has been given.

        // Note: if we don't clone here, the stack overflows. Probably an issue with the type
        // inference. We should fix the inference rules to prevent ambiguous situations like these.
        // Probably a cycle where Strings and char strings are converted between each other.
        if let Some(service_name) = &self.service_name {
            attributes.push(Tag::attribute(
                    attribute_id::SERVICE_NAME,
                    service_name.clone()));
        }

        // Add the service description, if it has been given.

        if let Some(service_description) = &self.service_description {
            attributes.push(Tag::attribute(
                    attribute_id::SERVICE_DESCRIPTION,
                    service_description.clone()));
        }

        // Add the provider name, if it has been given.

        if let Some(provider_name) = &self.provider_name {
            attributes.push(Tag::attribute(
                    attribute_id::PROVIDER_NAME,
                    provider_name.clone()));
        }

        // Add profile descriptor list, which contains the HID UUID and the version.

        attributes.push(Tag::attribute(
                attribute_id::BLUETOOTH_PROFILE_DESCRIPTOR_LIST,
                (
                    // Profile 0
                    (
                        Uuid::from(ServiceClass::Hid),
                        self.version, // Version
                    ),
                    )));

        // Add the HID parser version (1.1.1).

        attributes.push(Tag::attribute(
                attribute_id::hid::HID_PARSER_VERSION,
                0x0111u16)); // Mandatory this value

        // Add the HID device subclass.

        attributes.push(Tag::attribute(
                attribute_id::hid::HID_DEVICE_SUBCLASS,
                self.hid.device_subclass));

        // Add the country code.

        attributes.push(Tag::attribute(
                attribute_id::hid::HID_COUNTRY_CODE,
                self.hid.country_code)); // Optional, can be 0

        // Add the virtual cable attribute.

        attributes.push(Tag::attribute(
                attribute_id::hid::HID_VIRTUAL_CABLE,
                self.hid.virtual_cable)); // If HIDBootDevice is true, 5.3.4.12

        // Add the ReconnectInitiate attribute.

        attributes.push(Tag::attribute(
                attribute_id::hid::HID_RECONNECT_INITIATE,
                self.hid.reconnect_initiate)); // If HIDBootDevice is true, 5.3.4.12

        // Add HID descriptor lists.
        // This will likely contain a HID report descriptor.

        let descriptor_list: Vec<_> = self.hid.class_descriptors.iter()
            .map(|hid::ClassDescriptor(t, data)| (t, Tag::bytes(&data)))
            .collect();
        attributes.push(Tag::attribute(
                attribute_id::hid::HID_DESCRIPTOR_LIST,
                Tag::sequence(descriptor_list)));

        // Add language base attribute
        // Described in 5.3.4.8 of HID

        let mut language_bases = Vec::new();
        // Add primary language
        language_bases.push((self.primary_language.hid_code, 0x0100));
        // Add additional languages
        let additional_language_bases = self.hid.additional_languages.iter()
            .map(|l| (l.language, l.base));
        language_bases.extend(additional_language_bases);

        attributes.push(Tag::attribute(
                attribute_id::hid::HID_LANG_BASE_ATTRIBUTE,
                Tag::sequence(language_bases)));

        // Add battery power, if specified

        if let Some(battery_power) = self.hid.battery_power {
            attributes.push(Tag::attribute(
                    attribute_id::hid::HID_BATTERY_POWER,
                    battery_power));
        }

        // Add the remote wake attribute, if it has been specified.

        if let Some(remote_wake) = self.hid.remote_wake {
            attributes.push(Tag::attribute(
                    attribute_id::hid::HID_REMOTE_WAKE,
                    remote_wake));
        }

        // Specify the HIDSupervisionTimeout attribute, if it has been given.

        if let Some(supervision_timeout) = self.hid.supervision_timeout {
            attributes.push(Tag::attribute(
                    attribute_id::hid::HID_SUPERVISION_TIMEOUT,
                    supervision_timeout));
        }

        // Specify the HIDNormallyConnectable attribute, if it has been given.

        if let Some(normally_connectable) = self.hid.normally_connectable {
            attributes.push(Tag::attribute(
                    attribute_id::hid::HID_NORMALLY_CONNECTABLE,
                    normally_connectable)); // False because we are not always in page scan mode.
        }

        // Specify the boot device attribute.

        attributes.push(Tag::attribute(
                attribute_id::hid::HID_BOOT_DEVICE,
                self.hid.boot_device)); // Required for keyboards and mice.

        // Add the SSR Host Max Latency attribute, if it has been given.

        if let Some(ssr_host_max_latency) = self.hid.ssr_host_max_latency {
            attributes.push(Tag::attribute(
                    attribute_id::hid::HID_SSR_HOST_MAX_LATENCY,
                    ssr_host_max_latency));
        }

        // Add the SSR Host Min Latency attribute, if it has been given.

        if let Some(ssr_host_min_timeout) = self.hid.ssr_host_min_timeout {
            attributes.push(Tag::attribute(
                    attribute_id::hid::HID_SSR_HOST_MIN_TIMEOUT,
                    ssr_host_min_timeout));
        }

        // Construct document from attribute list

        Tag::record(attributes)
    }
}
