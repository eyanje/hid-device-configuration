pub mod from_sdp;
pub mod to_sdp;

pub use from_sdp::{Error, PartialConfiguration};

/// Struct for language ID data.
#[derive(Clone, Copy, Debug)]
pub struct LanguageCode {
    pub iso_code: u16, // ISO 639:1988 (E/F)
    pub hid_code: u16, // Defined by HID, difficult to know.
}

pub mod language {
    use super::LanguageCode;

    pub const ENGLISH: LanguageCode = LanguageCode { iso_code: 0x656e, hid_code: 0x0409 };
}

// MIBEnum value for UTF-8, from IANA's database.
pub mod encoding {
    pub const UTF_8: u16 = 0x006a;
}


pub mod hid {
    /// Struct for representing language base IDs.
    #[derive(Clone, Copy, Debug)]
    pub struct LanguageBase {
        pub language: u16,
        pub base: u16,
    }

    pub mod descriptor_type {
        pub const REPORT: u8 = 0x22;
        pub const PHYSICAL: u8 = 0x23;
    }

    // ID and data for a class descriptor
    #[derive(Clone, Debug)]
    pub struct ClassDescriptor(pub u8, pub Vec<u8>);

    impl ClassDescriptor {
        /// Create a new report descriptor
        pub fn report(data: Vec<u8>) -> Self {
            ClassDescriptor(descriptor_type::REPORT, data)
        }

        /// Create a new physical descriptor
        pub fn physical(data: Vec<u8>) -> Self {
            ClassDescriptor(descriptor_type::PHYSICAL, data)
        }
    }
    
    #[derive(Clone, Debug, Default)]
    pub struct Configuration {
        /// Device subclass, such as mouse, keyboard, etc.
        /// Required.
        pub device_subclass: u8,
        
        /// 8-bit country code, as defined in the USB HID specification.
        /// May be zero if the device is not localized.
        pub country_code: u8,
    
        /// Shall be true if the device is a boot device.
        /// Required.
        pub virtual_cable: bool, // True if boot_device
                            
        /// Shall be true if the device is a boot device.
        /// Required.
        pub reconnect_initiate: bool, // True if boot_device
                                  
        /// Descriptors for reports.
        pub class_descriptors: Vec<ClassDescriptor>,
    
        /// Base IDs for additional languages supported.
        pub additional_languages: Vec<LanguageBase>,
        
        /// Optional boolean indicating whether this device is battery-powered.
        pub battery_power: Option<bool>,
    
        pub remote_wake: Option<bool>,
    
        pub supervision_timeout: Option<u16>,
    
        pub normally_connectable: Option<bool>,
        /// Boolean indicating whether this device is a boot device.
        ///
        /// Keyboards and pointing devices must be boot devices.
        /// Required.
        pub boot_device: bool, // True for keyboards and pointing devices
    
        pub ssr_host_max_latency: Option<u16>,
        pub ssr_host_min_timeout: Option<u16>,
    }
}

// Configuration for a HID Bluetooth profile.
#[derive(Clone, Debug)]
pub struct Configuration {
    /// Primary language of the device.
    /// The primary language of a Bluetooth HID device is assigned the offset 0x0100 and is
    /// advertised in the general HID profile.
    pub primary_language: LanguageCode,
    /// MIBEnum encoding from IANA's database
    pub encoding: u16,

    pub service_name: Option<String>,
    pub service_description: Option<String>,
    pub provider_name: Option<String>,

    pub version: u16,

    pub hid: hid::Configuration,
}

