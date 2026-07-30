use crate::engine::mcu_protocol::{parse_mcu_midi_input, McuDeviceState, McuEvent};

#[tauri::command]
pub fn connect_mcu_device_cmd(device_name: String) -> Result<McuDeviceState, String> {
    Ok(McuDeviceState::new(&device_name))
}

#[tauri::command]
pub fn send_mcu_display_text_cmd(line1: String, line2: String) -> Result<McuDeviceState, String> {
    let mut state = McuDeviceState::new("MCU Surface");
    state.update_lcd_line(0, &line1);
    state.update_lcd_line(1, &line2);
    Ok(state)
}

#[tauri::command]
pub fn process_mcu_input_cmd(status: u8, data1: u8, data2: u8) -> Result<McuEvent, String> {
    Ok(parse_mcu_midi_input(status, data1, data2))
}
