use cec_rs::{CecConnection, CecConnectionCfgBuilder, CecDeviceType, CecDeviceTypeVec};

pub struct Cec {
    cec: CecConnection,
}

impl Cec {
    pub fn new() -> Self {
        let cec = CecConnectionCfgBuilder::default()
            .device_name("Sinon".to_string())
            .device_types(CecDeviceTypeVec::new(CecDeviceType::PlaybackDevice))
            .activate_source(false)
            .log_message_callback(Box::new(|msg| {
                println!("[CEC] {}", &msg.message);
            }))
            .command_received_callback(Box::new(|cmd| {
                println!("[CEC] Command received: {:?}", cmd.opcode);
            }))
            .build()
            .expect("Failed to build CEC config")
            .open()
            .expect("Failed to open CEC connection");

        let a = cec.get_active_source();
        dbg!(&a);

        Self { cec }
    }

    pub fn take_focus(&mut self) {
        self.cec
            .set_active_source(CecDeviceType::PlaybackDevice)
            .expect("Failed to set active source");
    }
}

impl Default for Cec {
    fn default() -> Self {
        Self::new()
    }
}
