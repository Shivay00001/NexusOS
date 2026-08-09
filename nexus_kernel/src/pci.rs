use x86_64::instructions::port::Port;
use alloc::vec::Vec;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
}

impl PciDevice {
    pub fn read_config(bus: u8, device: u8, func: u8, offset: u8) -> u32 {
        let address = 
            (1 << 31) | 
            ((bus as u32) << 16) | 
            ((device as u32) << 11) | 
            ((func as u32) << 8) | 
            (offset as u32 & 0xFC);
        
        let mut port_addr = Port::new(CONFIG_ADDRESS);
        let mut port_data = Port::new(CONFIG_DATA);
        
        unsafe {
            port_addr.write(address);
            port_data.read()
        }
    }

    pub fn read_vendor(bus: u8, device: u8, func: u8) -> u16 {
        (Self::read_config(bus, device, func, 0) & 0xFFFF) as u16
    }

    pub fn read_device(bus: u8, device: u8, func: u8) -> u16 {
        ((Self::read_config(bus, device, func, 0) >> 16) & 0xFFFF) as u16
    }
}

pub fn scan_bus() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    
    // Scan bus 0
    for bus in 0..=255 {
        for device in 0..32 {
            let vendor_id = PciDevice::read_vendor(bus, device, 0);
            if vendor_id != 0xFFFF {
                // Device exists!
                let device_id = PciDevice::read_device(bus, device, 0);
                let class_word = PciDevice::read_config(bus, device, 0, 0x08);
                let class = (class_word >> 24) as u8;
                let subclass = (class_word >> 16) as u8;
                
                devices.push(PciDevice {
                    bus,
                    device,
                    function: 0,
                    vendor_id,
                    device_id,
                    class,
                    subclass,
                });
            }
        }
    }
    
    devices
}
