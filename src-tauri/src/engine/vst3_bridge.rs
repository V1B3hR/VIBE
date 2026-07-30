#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(static_mut_refs)]

use super::graph::{AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::Path;
use uuid::Uuid;
use windows_sys::core::GUID;

// --- VST3 Constants ---
pub const kSample32: i32 = 0;
pub const kSample64: i32 = 1;
pub const kResultOk: i32 = 0;

// --- VST3 COM Interface IDs ---
pub const IPluginFactory_UUID: GUID = GUID {
    data1: 0x7A43813E,
    data2: 0x9F43,
    data3: 0x45F4,
    data4: [0x82, 0xD2, 0x62, 0xF6, 0x1E, 0x48, 0x44, 0x00],
};
pub const IComponent_UUID: GUID = GUID {
    data1: 0xE8317F60,
    data2: 0x6D58,
    data3: 0x4505,
    data4: [0xA0, 0xD3, 0x78, 0x72, 0x65, 0x2B, 0x06, 0x5E],
};
pub const IAudioProcessor_UUID: GUID = GUID {
    data1: 0x52528157,
    data2: 0xDE7A,
    data3: 0x4131,
    data4: [0xB2, 0xC1, 0x30, 0xD1, 0x0D, 0x12, 0xEA, 0xA6],
};
pub const IEditController_UUID: GUID = GUID {
    data1: 0xDCD7BBE3,
    data2: 0x7742,
    data3: 0x448D,
    data4: [0xA8, 0x74, 0xAA, 0xCC, 0x97, 0x9C, 0x75, 0x9E],
};
pub const IPlugView_UUID: GUID = GUID {
    data1: 0xFF53F398,
    data2: 0xD4EA,
    data3: 0x4BA4,
    data4: [0xA9, 0x91, 0xBD, 0x7B, 0x24, 0x02, 0x12, 0x0C],
};

pub const IComponentHandler_UUID: GUID = GUID {
    data1: 0x93A0BEA3,
    data2: 0x0BD0,
    data3: 0x4CF7,
    data4: [0x9A, 0x97, 0x95, 0x83, 0x60, 0x9E, 0x56, 0x21],
};
pub const IHostApplication_UUID: GUID = GUID {
    data1: 0x58E595CC,
    data2: 0xDB2D,
    data3: 0x4969,
    data4: [0x8B, 0x6A, 0xAF, 0x8C, 0x36, 0xA6, 0x64, 0xE5],
};
pub const IPlugViewContentScaleSupport_UUID: GUID = GUID {
    data1: 0x6D2A8564,
    data2: 0x9042,
    data3: 0x4366,
    data4: [0xAE, 0x85, 0xEB, 0x3E, 0x87, 0xA5, 0xB6, 0xB1],
};
pub const IPlugFrame_UUID: GUID = GUID {
    data1: 0x7E196024,
    data2: 0x228E,
    data3: 0x48DF,
    data4: [0xAA, 0x24, 0x61, 0xA4, 0x27, 0xAE, 0x20, 0xAF],
};
pub const IUnitInfo_UUID: GUID = GUID {
    data1: 0x3B5C1258,
    data2: 0x067E,
    data3: 0x481E,
    data4: [0xAF, 0x6B, 0x9C, 0xC9, 0x0F, 0xC9, 0xBC, 0xFC],
};

// --- VST3 Structures ---
#[repr(C)]
pub struct PClassInfo {
    pub cid: [i8; 32],
    pub cardinality: i32,
    pub category: [i8; 32],
    pub name: [i8; 64],
}

// --- VTables ---
#[repr(C)]
struct IUnknownVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct IPluginFactoryVtbl {
    base: IUnknownVtbl,
    get_factory_info: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    count_classes: unsafe extern "system" fn(*mut c_void) -> i32,
    get_class_info: unsafe extern "system" fn(*mut c_void, i32, *mut PClassInfo) -> i32,
    create_instance:
        unsafe extern "system" fn(*mut c_void, *const i8, *const i8, *mut *mut c_void) -> i32,
}

#[repr(C)]
struct IComponentVtbl {
    base: IUnknownVtbl,
    initialize: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    terminate: unsafe extern "system" fn(*mut c_void) -> i32,
    get_controller_class_id: unsafe extern "system" fn(*mut c_void, *mut GUID) -> i32,
    set_io_mode: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    get_bus_count: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
    get_bus_info: unsafe extern "system" fn(*mut c_void, i32, i32, i32, *mut c_void) -> i32,
    get_routing_info: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> i32,
    activate_bus: unsafe extern "system" fn(*mut c_void, i32, i32, i32, i32) -> i32,
    set_active: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    set_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    get_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
}

#[repr(C)]
struct IAudioProcessorVtbl {
    base: IUnknownVtbl,
    set_bus_arrangements:
        unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut c_void, i32) -> i32,
    get_bus_arrangement: unsafe extern "system" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
    can_process_sample_size: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    set_processing: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    set_process_setup: unsafe extern "system" fn(*mut c_void, *const ProcessSetup) -> i32,
    process: unsafe extern "system" fn(*mut c_void, *mut ProcessData) -> i32,
    get_latency_samples: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct IEditControllerVtbl {
    base: IUnknownVtbl,
    set_component_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    set_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    get_state: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    get_parameter_count: unsafe extern "system" fn(*mut c_void) -> i32,
    get_parameter_info: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32,
    get_param_string_by_value: unsafe extern "system" fn(*mut c_void, i32, f64, *mut c_void) -> i32,
    get_param_value_by_string:
        unsafe extern "system" fn(*mut c_void, i32, *const c_void, *mut f64) -> i32,
    normalized_param_to_plain: unsafe extern "system" fn(*mut c_void, i32, f64) -> f64,
    plain_param_to_normalized: unsafe extern "system" fn(*mut c_void, i32, f64) -> f64,
    get_param_normalized: unsafe extern "system" fn(*mut c_void, i32) -> f64,
    set_param_normalized: unsafe extern "system" fn(*mut c_void, i32, f64) -> i32,
    set_component_handler: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    create_view: unsafe extern "system" fn(*mut c_void, *const i8, *mut *mut c_void) -> i32,
}

#[repr(C)]
struct IPlugViewVtbl {
    base: IUnknownVtbl,
    is_platform_type_supported: unsafe extern "system" fn(*mut c_void, *const i8) -> i32,
    attached: unsafe extern "system" fn(*mut c_void, *mut c_void, *const i8) -> i32,
    removed: unsafe extern "system" fn(*mut c_void) -> i32,
    on_wheel: unsafe extern "system" fn(*mut c_void, f32) -> i32,
    on_key_down: unsafe extern "system" fn(*mut c_void, i16, i16, i16) -> i32,
    on_key_up: unsafe extern "system" fn(*mut c_void, i16, i16, i16) -> i32,
    get_size: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    on_size: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    on_focus: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    set_frame: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    can_resize: unsafe extern "system" fn(*mut c_void) -> i32,
    check_size_constraint: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
}

#[repr(C)]
struct IPlugViewContentScaleSupportVtbl {
    base: IUnknownVtbl,
    set_content_scale_factor: unsafe extern "system" fn(*mut c_void, f32) -> i32,
}

#[repr(C)]
struct IPlugFrameVtbl {
    base: IUnknownVtbl,
    resize_view: unsafe extern "system" fn(*mut c_void, *mut c_void, *const ViewRect) -> i32,
}

#[repr(C)]
struct ProgramListInfo {
    id: i32,
    name: [u16; 128],
    program_count: i32,
}

#[repr(C)]
struct IUnitInfoVtbl {
    base: IUnknownVtbl,
    get_unit_count: unsafe extern "system" fn(*mut c_void) -> i32,
    get_unit_info: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> i32, // Simplified UnitInfo pointer
    get_program_list_count: unsafe extern "system" fn(*mut c_void) -> i32,
    get_program_list_info: unsafe extern "system" fn(*mut c_void, i32, *mut ProgramListInfo) -> i32,
    get_program_name: unsafe extern "system" fn(*mut c_void, i32, i32, *mut u16) -> i32,
    get_program_info: unsafe extern "system" fn(*mut c_void, i32, i32, *mut c_void) -> i32,
    has_program_pitch_names: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
    get_program_pitch_name: unsafe extern "system" fn(*mut c_void, i32, i32, i16, *mut u16) -> i32,
    get_selected_unit: unsafe extern "system" fn(*mut c_void) -> i32,
    select_unit: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    get_unit_by_bus: unsafe extern "system" fn(*mut c_void, i32, i32, i32, *mut i32) -> i32,
    set_unit_program_index: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
}

pub const IBStream_UUID: GUID = GUID {
    data1: 0x8405BF93,
    data2: 0x949C,
    data3: 0x42E5,
    data4: [0xA9, 0x58, 0x82, 0xF2, 0x05, 0x72, 0x56, 0xCE],
};

#[repr(C)]
struct Vst3ParameterInfo {
    id: u32,
    title: [u16; 128],
    short_title: [u16; 128],
    units: [u16; 128],
    step_count: i32,
    default_normalized_value: f64,
    unit_id: i32,
    flags: i32,
}

#[repr(C)]
struct ViewRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct IBStreamVtbl {
    base: IUnknownVtbl,
    read: unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut i32) -> i32,
    write: unsafe extern "system" fn(*mut c_void, *const c_void, i32, *mut i32) -> i32,
    seek: unsafe extern "system" fn(*mut c_void, i64, i32, *mut i64) -> i32,
    tell: unsafe extern "system" fn(*mut c_void, *mut i64) -> i32,
}

#[repr(C)]
struct ProcessSetup {
    process_mode: i32,
    symbolic_sample_size: i32,
    max_block_size: i32,
    sample_rate: f64,
}

type GetFactoryFunc = unsafe extern "system" fn() -> *mut c_void;

#[repr(C)]
struct ProcessData {
    process_mode: i32,
    symbolic_sample_size: i32,
    num_samples: i32,
    num_inputs: i32,
    num_outputs: i32,
    inputs: *mut AudioBusBuffers,
    outputs: *mut AudioBusBuffers,
    parameter_changes: *mut c_void, // IParameterChanges
    event_list: *mut c_void,        // IEventList
}

// ─── VST3 Event Types ─────────────────────────────────────────────────────────

#[repr(i16)]
#[allow(dead_code)]
enum Vst3EventType {
    NoteOn  = 0,
    NoteOff = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vst3NoteOnEvent {
    channel:   i16,
    pitch:     i16,
    tuning:    f32,
    velocity:  f32,
    length:    i32,
    note_id:   i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vst3NoteOffEvent {
    channel:   i16,
    pitch:     i16,
    velocity:  f32,
    note_id:   i32,
    tuning:    f32,
}

#[repr(C)]
union Vst3EventUnion {
    note_on:   Vst3NoteOnEvent,
    note_off:  Vst3NoteOffEvent,
    _raw:      [u8; 40],
}

#[repr(C)]
pub struct Vst3Event {
    bus_index:      i32,
    sample_offset:  i32,
    ppq_position:   f64,
    flags:          u16,
    event_type:     i16,
    event:          Vst3EventUnion,
}

// ─── IEventList COM Implementation ────────────────────────────────────────────

#[repr(C)]
struct IEventListVtbl {
    base:       IUnknownVtbl,
    get_count:  unsafe extern "system" fn(*mut c_void) -> i32,
    get_event:  unsafe extern "system" fn(*mut c_void, i32, *mut Vst3Event) -> i32,
    add_event:  unsafe extern "system" fn(*mut c_void, *const Vst3Event) -> i32,
}

pub struct EventList {
    vtable: *const IEventListVtbl,
    ref_count: u32,
    pub events: Vec<Vst3Event>,
}

unsafe impl Send for EventList {}

impl EventList {
    pub fn new() -> Self {
        Self {
            vtable: &EVENT_LIST_VTABLE,
            ref_count: 1,
            events: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self as *const Self as *mut c_void
    }

    pub fn push_note_on(&mut self, channel: u8, pitch: u8, velocity: u8, offset: i32) {
        self.events.push(Vst3Event {
            bus_index: 0,
            sample_offset: offset,
            ppq_position: 0.0,
            flags: 1, // kIsLive
            event_type: 0, // NoteOn
            event: Vst3EventUnion {
                note_on: Vst3NoteOnEvent {
                    channel: channel as i16,
                    pitch: pitch as i16,
                    tuning: 0.0,
                    velocity: velocity as f32 / 127.0,
                    length: 0,
                    note_id: -1,
                },
            },
        });
    }

    pub fn push_note_off(&mut self, channel: u8, pitch: u8, velocity: u8, offset: i32) {
        self.events.push(Vst3Event {
            bus_index: 0,
            sample_offset: offset,
            ppq_position: 0.0,
            flags: 1,
            event_type: 1, // NoteOff
            event: Vst3EventUnion {
                note_off: Vst3NoteOffEvent {
                    channel: channel as i16,
                    pitch: pitch as i16,
                    velocity: velocity as f32 / 127.0,
                    note_id: -1,
                    tuning: 0.0,
                },
            },
        });
    }
}

static EVENT_LIST_VTABLE: IEventListVtbl = IEventListVtbl {
    base: IUnknownVtbl {
        query_interface: event_list_qi,
        add_ref:         event_list_add_ref,
        release:         event_list_release,
    },
    get_count: event_list_get_count,
    get_event: event_list_get_event,
    add_event: event_list_add_event,
};

unsafe extern "system" fn event_list_qi(this: *mut c_void, _iid: *const GUID, out: *mut *mut c_void) -> i32 {
    *out = this; kResultOk
}
unsafe extern "system" fn event_list_add_ref(this: *mut c_void) -> u32 {
    let s = &mut *(this as *mut EventList); s.ref_count += 1; s.ref_count
}
unsafe extern "system" fn event_list_release(this: *mut c_void) -> u32 {
    let s = &mut *(this as *mut EventList); s.ref_count = s.ref_count.saturating_sub(1); s.ref_count
}
unsafe extern "system" fn event_list_get_count(this: *mut c_void) -> i32 {
    let s = &*(this as *mut EventList); s.events.len() as i32
}
unsafe extern "system" fn event_list_get_event(this: *mut c_void, idx: i32, out: *mut Vst3Event) -> i32 {
    let s = &*(this as *mut EventList);
    if let Some(e) = s.events.get(idx as usize) {
        std::ptr::copy_nonoverlapping(e as *const Vst3Event, out, 1);
        kResultOk
    } else { -1 }
}
unsafe extern "system" fn event_list_add_event(_this: *mut c_void, _ev: *const Vst3Event) -> i32 {
    kResultOk
}

// ─── IParameterChanges COM Implementation ─────────────────────────────────────

#[repr(C)]
struct IParameterChangesVtbl {
    base:               IUnknownVtbl,
    get_param_count:    unsafe extern "system" fn(*mut c_void) -> i32,
    get_param_data:     unsafe extern "system" fn(*mut c_void, i32) -> *mut c_void,
    add_param_data:     unsafe extern "system" fn(*mut c_void, *const u32, *mut i32) -> *mut c_void,
}

#[repr(C)]
struct IParamValueQueueVtbl {
    base:               IUnknownVtbl,
    get_param_id:       unsafe extern "system" fn(*mut c_void) -> u32,
    get_point_count:    unsafe extern "system" fn(*mut c_void) -> i32,
    get_point:          unsafe extern "system" fn(*mut c_void, i32, *mut i32, *mut f64) -> i32,
    add_point:          unsafe extern "system" fn(*mut c_void, i32, f64, *mut i32) -> i32,
}

/// A single parameter queue for IParameterChanges
pub struct ParamValueQueue {
    vtable:   *const IParamValueQueueVtbl,
    ref_count: u32,
    pub param_id: u32,
    /// (sample_offset, normalized_value)
    pub points: Vec<(i32, f64)>,
}

unsafe impl Send for ParamValueQueue {}

impl ParamValueQueue {
    fn new(param_id: u32) -> Box<Self> {
        Box::new(Self {
            vtable: &PARAM_QUEUE_VTABLE,
            ref_count: 1,
            param_id,
            points: Vec::new(),
        })
    }
}

static PARAM_QUEUE_VTABLE: IParamValueQueueVtbl = IParamValueQueueVtbl {
    base: IUnknownVtbl {
        query_interface: pq_qi,
        add_ref:         pq_add_ref,
        release:         pq_release,
    },
    get_param_id:    pq_get_id,
    get_point_count: pq_get_count,
    get_point:       pq_get_point,
    add_point:       pq_add_point,
};

unsafe extern "system" fn pq_qi(this: *mut c_void, _: *const GUID, out: *mut *mut c_void) -> i32 { *out = this; kResultOk }
unsafe extern "system" fn pq_add_ref(this: *mut c_void) -> u32 { let s = &mut *(this as *mut ParamValueQueue); s.ref_count += 1; s.ref_count }
unsafe extern "system" fn pq_release(this: *mut c_void) -> u32 { let s = &mut *(this as *mut ParamValueQueue); s.ref_count = s.ref_count.saturating_sub(1); s.ref_count }
unsafe extern "system" fn pq_get_id(this: *mut c_void) -> u32 { let s = &*(this as *mut ParamValueQueue); s.param_id }
unsafe extern "system" fn pq_get_count(this: *mut c_void) -> i32 { let s = &*(this as *mut ParamValueQueue); s.points.len() as i32 }
unsafe extern "system" fn pq_get_point(this: *mut c_void, idx: i32, sample_off: *mut i32, val: *mut f64) -> i32 {
    let s = &*(this as *mut ParamValueQueue);
    if let Some(&(off, v)) = s.points.get(idx as usize) {
        *sample_off = off; *val = v; kResultOk
    } else { -1 }
}
unsafe extern "system" fn pq_add_point(_: *mut c_void, _: i32, _: f64, _: *mut i32) -> i32 { kResultOk }

/// Container holding all queued param changes for a single process block
pub struct ParameterChanges {
    vtable:    *const IParameterChangesVtbl,
    ref_count: u32,
    pub queues: Vec<Box<ParamValueQueue>>,
    queue_ptrs: Vec<*mut c_void>,
}

unsafe impl Send for ParameterChanges {}

impl ParameterChanges {
    pub fn new() -> Self {
        Self {
            vtable: &PARAM_CHANGES_VTABLE,
            ref_count: 1,
            queues: Vec::new(),
            queue_ptrs: Vec::new(),
        }
    }

    pub fn add_point(&mut self, param_id: u32, sample_offset: i32, value: f64) {
        if let Some(q) = self.queues.iter_mut().find(|q| q.param_id == param_id) {
            q.points.push((sample_offset, value));
        } else {
            let mut q = ParamValueQueue::new(param_id);
            q.points.push((sample_offset, value));
            self.queue_ptrs.push(q.as_mut() as *mut ParamValueQueue as *mut c_void);
            self.queues.push(q);
        }
    }

    pub fn set_parameter(&mut self, param_id: u32, value: f64) {
        // Clear previous and set a single point at 0
        if let Some(q) = self.queues.iter_mut().find(|q| q.param_id == param_id) {
            q.points.clear();
            q.points.push((0, value));
        } else {
            let mut q = ParamValueQueue::new(param_id);
            q.points.push((0, value));
            self.queue_ptrs.push(q.as_mut() as *mut ParamValueQueue as *mut c_void);
            self.queues.push(q);
        }
    }

    pub fn clear(&mut self) {
        self.queues.clear();
        self.queue_ptrs.clear();
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self as *const Self as *mut c_void
    }
}

static PARAM_CHANGES_VTABLE: IParameterChangesVtbl = IParameterChangesVtbl {
    base: IUnknownVtbl {
        query_interface: pc_qi,
        add_ref:         pc_add_ref,
        release:         pc_release,
    },
    get_param_count: pc_get_count,
    get_param_data:  pc_get_data,
    add_param_data:  pc_add_data,
};

unsafe extern "system" fn pc_qi(this: *mut c_void, _: *const GUID, out: *mut *mut c_void) -> i32 { *out = this; kResultOk }
unsafe extern "system" fn pc_add_ref(this: *mut c_void) -> u32 { let s = &mut *(this as *mut ParameterChanges); s.ref_count += 1; s.ref_count }
unsafe extern "system" fn pc_release(this: *mut c_void) -> u32 { let s = &mut *(this as *mut ParameterChanges); s.ref_count = s.ref_count.saturating_sub(1); s.ref_count }
unsafe extern "system" fn pc_get_count(this: *mut c_void) -> i32 { let s = &*(this as *mut ParameterChanges); s.queues.len() as i32 }
unsafe extern "system" fn pc_get_data(this: *mut c_void, idx: i32) -> *mut c_void {
    let s = &*(this as *mut ParameterChanges);
    s.queue_ptrs.get(idx as usize).copied().unwrap_or(std::ptr::null_mut())
}
unsafe extern "system" fn pc_add_data(_: *mut c_void, _: *const u32, _: *mut i32) -> *mut c_void { std::ptr::null_mut() }

#[repr(C)]
struct AudioBusBuffers {
    num_channels: i32,
    silence_flags: u64,
    buffers: *mut *mut c_void,
}

/// VST3 Buffers
struct Vst3ScratchBuffers {
    input_channels: Vec<*mut f64>,
    output_channels: Vec<*mut f64>,
    inputs_f32: Vec<Vec<f32>>,
    outputs_f32: Vec<Vec<f32>>,
    input_channels_f32: Vec<*mut f32>,
    output_channels_f32: Vec<*mut f32>,
    supports_f64: bool,
    max_block_size: usize,
}

impl Vst3ScratchBuffers {
    fn new(channels: usize, block_size: usize, supports_f64: bool) -> Self {
        let mut s = Self {
            input_channels: vec![std::ptr::null_mut(); channels],
            output_channels: vec![std::ptr::null_mut(); channels],
            inputs_f32: if !supports_f64 {
                vec![vec![0.0; block_size]; channels]
            } else {
                vec![]
            },
            outputs_f32: if !supports_f64 {
                vec![vec![0.0; block_size]; channels]
            } else {
                vec![]
            },
            input_channels_f32: vec![std::ptr::null_mut(); channels],
            output_channels_f32: vec![std::ptr::null_mut(); channels],
            supports_f64,
            max_block_size: block_size,
        };
        s.sync_ptrs();
        s
    }

    fn sync_ptrs(&mut self) {
        if !self.supports_f64 {
            for i in 0..self.inputs_f32.len() {
                self.input_channels_f32[i] = self.inputs_f32[i].as_mut_ptr();
                self.output_channels_f32[i] = self.outputs_f32[i].as_mut_ptr();
            }
        }
    }

    fn resize(&mut self, frames: usize) {
        if frames > self.max_block_size {
            if !self.supports_f64 {
                for ch in &mut self.inputs_f32 {
                    ch.resize(frames, 0.0);
                }
                for ch in &mut self.outputs_f32 {
                    ch.resize(frames, 0.0);
                }
            }
            self.max_block_size = frames;
            self.sync_ptrs();
        }
    }

}


// =====================================================================
// IComponentHandler — host-side COM object.
// Plugin GUI calls ch_perform_edit() when user moves a knob.
// We queue (param_id, value) into a shared Arc<Mutex<Vec>> so the
// Tauri command layer can poll changes and push them to the UI.
// =====================================================================

#[repr(C)]
struct IComponentHandlerVtbl {
    base:              IUnknownVtbl,
    begin_edit:        unsafe extern "system" fn(*mut c_void, u32) -> i32,
    perform_edit:      unsafe extern "system" fn(*mut c_void, u32, f64) -> i32,
    end_edit:          unsafe extern "system" fn(*mut c_void, u32) -> i32,
    restart_component: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

#[repr(C)]
struct ComponentHandler {
    vtable:    *const IComponentHandlerVtbl,
    ref_count: std::sync::atomic::AtomicU32,
    feedback:  std::sync::Arc<std::sync::Mutex<Vec<(u32, f64)>>>,
    pdc_recalc: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
unsafe impl Send for ComponentHandler {}
unsafe impl Sync for ComponentHandler {}

unsafe extern "system" fn ch_query_interface(
    this: *mut c_void, iid: *const GUID, obj: *mut *mut c_void,
) -> i32 {
    if (*iid).data1 == IComponentHandler_UUID.data1 { *obj = this; return kResultOk; }
    -1
}
// --- IBStream implementation ---
// (Already implemented at the end of file as MemoryStream)
unsafe extern "system" fn ch_add_ref(this: *mut c_void) -> u32 {
    (*(this as *mut ComponentHandler)).ref_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}
unsafe extern "system" fn ch_release(this: *mut c_void) -> u32 {
    let v = (*(this as *mut ComponentHandler)).ref_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    v.saturating_sub(1)
}
unsafe extern "system" fn ch_begin_edit(_: *mut c_void, _: u32) -> i32 { kResultOk }
unsafe extern "system" fn ch_perform_edit(this: *mut c_void, param_id: u32, value: f64) -> i32 {
    if let Ok(mut q) = (*(this as *mut ComponentHandler)).feedback.lock() {
        q.push((param_id, value));
    }
    kResultOk
}
unsafe extern "system" fn ch_end_edit(_: *mut c_void, _: u32) -> i32 { kResultOk }
unsafe extern "system" fn ch_restart_component(this: *mut c_void, flags: i32) -> i32 {
    let handler = &*(this as *mut ComponentHandler);
    // kLatencyChanged = 1 << 3
    if flags & (1 << 3) != 0 {
        handler.pdc_recalc.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    kResultOk
}

static COMPONENT_HANDLER_VTABLE: IComponentHandlerVtbl = IComponentHandlerVtbl {
    base: IUnknownVtbl {
        query_interface: ch_query_interface,
        add_ref:         ch_add_ref,
        release:         ch_release,
    },
    begin_edit:        ch_begin_edit,
    perform_edit:      ch_perform_edit,
    end_edit:          ch_end_edit,
    restart_component: ch_restart_component,
};

impl ComponentHandler {
    fn new(feedback: std::sync::Arc<std::sync::Mutex<Vec<(u32, f64)>>>, pdc_recalc: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Box<Self> {
        Box::new(Self {
            vtable:    &COMPONENT_HANDLER_VTABLE,
            ref_count: std::sync::atomic::AtomicU32::new(1),
            feedback,
            pdc_recalc,
        })
    }
    fn as_com_ptr(h: &mut Box<Self>) -> *mut c_void {
        h.as_mut() as *mut ComponentHandler as *mut c_void
    }
}

// =====================================================================
// IHostApplication stub — lets plugins (FabFilter, etc.) identify host.
// =====================================================================

#[repr(C)]
struct IHostApplicationVtbl {
    base:            IUnknownVtbl,
    get_name:        unsafe extern "system" fn(*mut c_void, *mut u16) -> i32,
    create_instance: unsafe extern "system" fn(*mut c_void, *const GUID, *const GUID, *mut *mut c_void) -> i32,
}

#[repr(C)]
pub struct HostApplicationObject {
    vtable:    *const IHostApplicationVtbl,
    ref_count: std::sync::atomic::AtomicU32,
}
unsafe impl Send for HostApplicationObject {}
unsafe impl Sync for HostApplicationObject {}

unsafe extern "system" fn ha_query_interface(
    this: *mut c_void, iid: *const GUID, obj: *mut *mut c_void,
) -> i32 {
    if (*iid).data1 == IHostApplication_UUID.data1 { *obj = this; return kResultOk; }
    -1
}
unsafe extern "system" fn ha_add_ref(this: *mut c_void) -> u32 {
    (*(this as *mut HostApplicationObject)).ref_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}
unsafe extern "system" fn ha_release(this: *mut c_void) -> u32 {
    (*(this as *mut HostApplicationObject)).ref_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed).saturating_sub(1)
}
unsafe extern "system" fn ha_get_name(_: *mut c_void, name: *mut u16) -> i32 {
    let n: Vec<u16> = "VIBE".encode_utf16().collect();
    for (i, &ch) in n.iter().enumerate() { *name.add(i) = ch; }
    *name.add(n.len()) = 0;
    kResultOk
}
unsafe extern "system" fn ha_create_instance(_: *mut c_void, _: *const GUID, _: *const GUID, _: *mut *mut c_void) -> i32 { -1 }

static HOST_APPLICATION_VTABLE: IHostApplicationVtbl = IHostApplicationVtbl {
    base: IUnknownVtbl { query_interface: ha_query_interface, add_ref: ha_add_ref, release: ha_release },
    get_name:        ha_get_name,
    create_instance: ha_create_instance,
};

impl HostApplicationObject {
    pub fn new() -> Box<Self> {
        Box::new(Self { vtable: &HOST_APPLICATION_VTABLE, ref_count: std::sync::atomic::AtomicU32::new(1) })
    }
    pub fn as_com_ptr(h: &mut Box<Self>) -> *mut c_void {
        h.as_mut() as *mut HostApplicationObject as *mut c_void
    }
}

// ─── IPlugFrame — host side ──────────────────────────────────────────────────
#[repr(C)]
struct PlugFrameObject {
    vtable:    *const IPlugFrameVtbl,
    ref_count: std::sync::atomic::AtomicU32,
    pending_resize: std::sync::Arc<std::sync::Mutex<Option<(u32, u32)>>>,
}

unsafe extern "system" fn pf_query_interface(
    this: *mut c_void, iid: *const GUID, obj: *mut *mut c_void,
) -> i32 {
    if (*iid).data1 == IPlugFrame_UUID.data1 { *obj = this; return kResultOk; }
    -1
}
unsafe extern "system" fn pf_add_ref(this: *mut c_void) -> u32 {
    (*(this as *mut PlugFrameObject)).ref_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}
unsafe extern "system" fn pf_release(this: *mut c_void) -> u32 {
    (*(this as *mut PlugFrameObject)).ref_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed).saturating_sub(1)
}
unsafe extern "system" fn pf_resize_view(this: *mut c_void, _view: *mut c_void, rect: *const ViewRect) -> i32 {
    let pf = &*(this as *mut PlugFrameObject);
    let r = unsafe { &*rect };
    let w = (r.right - r.left) as u32;
    let h = (r.bottom - r.top) as u32;
    if let Ok(mut pending) = pf.pending_resize.lock() {
        *pending = Some((w, h));
    }
    kResultOk
}

static PLUG_FRAME_VTABLE: IPlugFrameVtbl = IPlugFrameVtbl {
    base: IUnknownVtbl { query_interface: pf_query_interface, add_ref: pf_add_ref, release: pf_release },
    resize_view: pf_resize_view,
};

impl PlugFrameObject {
    fn new(pending_resize: std::sync::Arc<std::sync::Mutex<Option<(u32, u32)>>>) -> Box<Self> {
        Box::new(Self { vtable: &PLUG_FRAME_VTABLE, ref_count: std::sync::atomic::AtomicU32::new(1), pending_resize })
    }
}

pub struct Vst3Bridge {
    id: Uuid,
    name: String,
    path: String,
    parameters: Vec<Parameter>,
    param_ids: Vec<u32>,
    component: *mut c_void,
    processor: *mut c_void,
    editor_controller: *mut c_void,
    plug_view: *mut c_void,
    library: Library,
    scratch: Vst3ScratchBuffers,
    is_active: bool,
    /// Queued MIDI events for the next process() block
    event_list: EventList,
    /// Queued parameter changes for the next process() block
    param_changes: ParameterChanges,
    /// Plugin-reported PDC latency in samples
    reported_latency: usize,
    /// IComponentHandler COM object — plugin GUI writes param changes here
    component_handler: Option<Box<ComponentHandler>>,
    /// Shared queue for GUI-driven parameter changes (plugin → host)
    param_feedback: std::sync::Arc<std::sync::Mutex<Vec<(u32, f64)>>>, // Still u32 in the queue
    /// Host application object (stub) passed to the plugin
    host_app: Box<HostApplicationObject>,
    /// Smoothed CPU usage percentage (0.0 - 1.0)
    cpu_usage: std::sync::atomic::AtomicU32, // Bit-packed f32
    /// IPlugFrame COM object for editor resizing
    plug_frame: Option<Box<PlugFrameObject>>,
    /// Flag for PDC recalculation
    pdc_recalc_needed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Pending resize from plugin
    pending_resize: std::sync::Arc<std::sync::Mutex<Option<(u32, u32)>>>,
    num_input_buses: i32,
    num_output_buses: i32,
}

unsafe impl Send for Vst3Bridge {}
unsafe impl Sync for Vst3Bridge {}

impl Vst3Bridge {
    pub fn new(path: &str, sample_rate: f64, block_size: usize) -> Result<Self, String> {
        let path_obj = Path::new(path);
        let library = unsafe { Library::new(path_obj).map_err(|e| e.to_string())? };

        let factory_ptr = unsafe {
            let func: Symbol<GetFactoryFunc> = library
                .get(b"GetPluginFactory\0")
                .map_err(|_| "No factory")?;
            func()
        };

        if factory_ptr.is_null() {
            return Err("VST3: Factory is null".into());
        }

        let mut component_ptr: *mut c_void = std::ptr::null_mut();

        unsafe {
            // Fix: cast via pointer to pointer
            let factory_vtable = *(factory_ptr as *mut *mut IPluginFactoryVtbl);
            let count = ((*factory_vtable).count_classes)(factory_ptr);

            println!("VST3: Factory has {} classes", count);

            for i in 0..count {
                let mut info: PClassInfo = std::mem::zeroed();
                if ((*factory_vtable).get_class_info)(factory_ptr, i, &mut info) == kResultOk {
                    // Helper to check category string
                    let _cat_ptr = info.category.as_ptr() as *const u8;
                    let cat_len = (0..32).position(|i| info.category[i] == 0).unwrap_or(32);
                    let category = std::str::from_utf8(std::mem::transmute::<&[i8], &[u8]>(
                        &info.category[..cat_len],
                    ))
                    .unwrap_or("");

                    println!("VST3 Class {}: Category='{}'", i, category);

                    if category.contains("Audio Module Class") {
                        // VST3 v1 uses strings for IDs in createInstance
                        // IComponent IID: E8317F60-6D58-4505-A0D3-7872652B065E
                        // We use the raw hex string format (32 chars) as per VST3 expectations for PClassInfo::cid
                        let iid_str = b"E8317F606D584505A0D37872652B065E\0";
                        let res = ((*factory_vtable).create_instance)(
                            factory_ptr,
                            info.cid.as_ptr(),
                            iid_str.as_ptr() as *const i8,
                            &mut component_ptr,
                        );

                        if res == kResultOk && !component_ptr.is_null() {
                            println!("VST3: Created Component Instance");
                            break;
                        }
                    }
                }
            }

            if component_ptr.is_null() {
                return Err("VST3: Could not create Audio Module component".into());
            }

            // 3. Initialize Component with Host Application context
            let mut host_obj = HostApplicationObject::new();
            let host_ptr = HostApplicationObject::as_com_ptr(&mut host_obj);
            
            let component_vtable = *(component_ptr as *mut *mut IComponentVtbl);
            let init_res = ((*component_vtable).initialize)(component_ptr, host_ptr);
            if init_res != kResultOk {
                return Err(format!("VST3: Initialize failed code {}", init_res));
            }
        }

        let (num_input_buses, num_output_buses) = unsafe {
            let component_vtable = *(component_ptr as *mut *mut IComponentVtbl);
            let inputs = ((*component_vtable).get_bus_count)(component_ptr, 0, 0);
            let outputs = ((*component_vtable).get_bus_count)(component_ptr, 0, 1);
            
            for i in 0..inputs {
                ((*component_vtable).activate_bus)(component_ptr, 0, 0, i, 1);
            }
            for i in 0..outputs {
                ((*component_vtable).activate_bus)(component_ptr, 0, 1, i, 1);
            }
            (inputs, outputs)
        };

        let host_app = HostApplicationObject::new();
        let pdc_recalc_needed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let pending_resize = std::sync::Arc::new(std::sync::Mutex::new(None));

        // Scope block closed to allow mut borrow later if needed, but we keep pointers.
        let mut processor_ptr: *mut c_void = std::ptr::null_mut();

        unsafe {
            // 4. Query Audio Processor
            let component_unknown_vtable = *(component_ptr as *mut *mut IUnknownVtbl);
            let query_res = ((*component_unknown_vtable).query_interface)(
                component_ptr,
                &IAudioProcessor_UUID,
                &mut processor_ptr,
            );

            if query_res != kResultOk || processor_ptr.is_null() {
                return Err("VST3: Failed to get IAudioProcessor interface".into());
            }
            println!("VST3: Got AudioProcessor Interface");

            // 5. Setup Bus Arrangements (Stereo / Stereo)
            let processor_vtable = *(processor_ptr as *mut *mut IAudioProcessorVtbl);
            ((*processor_vtable).set_bus_arrangements)(
                processor_ptr,
                std::ptr::null_mut(),
                3,
                std::ptr::null_mut(),
                3,
            );

            // 6. Setup Processing
            let setup = ProcessSetup {
                process_mode: 0, // kRealtime
                symbolic_sample_size: kSample64,
                max_block_size: block_size as i32,
                sample_rate,
            };
            ((*processor_vtable).set_process_setup)(processor_ptr, &setup);

            // 7. Activate
            let component_vtable = *(component_ptr as *mut *mut IComponentVtbl);
            ((*component_vtable).set_active)(component_ptr, 1);
            ((*processor_vtable).set_processing)(processor_ptr, 1);
        }

        // Probe f64 support; fall back to f32 if not available
        let supports_f64 = unsafe {
            let processor_vtable = *(processor_ptr as *mut *mut IAudioProcessorVtbl);
            ((*processor_vtable).can_process_sample_size)(processor_ptr, kSample64) == kResultOk
        };

        // Query PDC latency (called after set_process_setup)
        let reported_latency: usize = 0; // Will be updated after first process block if needed

        let mut b = Self {
            id: Uuid::new_v4(),
            name: path_obj.file_stem().unwrap().to_string_lossy().into(),
            path: path.to_string(),
            parameters: Vec::new(),
            param_ids: Vec::new(),
            component: component_ptr,
            processor: processor_ptr,
            editor_controller: std::ptr::null_mut(),
            plug_view: std::ptr::null_mut(),
            library,
            scratch: Vst3ScratchBuffers::new(2, block_size, supports_f64),
            is_active: true,
            event_list: EventList::new(),
            param_changes: ParameterChanges::new(),
            reported_latency,
            component_handler: None,
            param_feedback: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            host_app,
            cpu_usage: std::sync::atomic::AtomicU32::new(0),
            plug_frame: Some(PlugFrameObject::new(pending_resize.clone())),
            pdc_recalc_needed,
            pending_resize,
            num_input_buses,
            num_output_buses,
        };

        // 8. Parameter Discovery
        unsafe {
            let unknown = b.component as *mut *mut IUnknownVtbl;
            let mut controller: *mut c_void = std::ptr::null_mut();
            if ((*(*unknown)).query_interface)(
                b.component,
                &IEditController_UUID,
                &mut controller,
            ) == kResultOk
            {
                b.editor_controller = controller;
                let controller_vtable = *(controller as *mut *mut IEditControllerVtbl);
                let param_count = ((*controller_vtable).get_parameter_count)(controller);
                
                for i in 0..param_count {
                    let mut info: Vst3ParameterInfo = std::mem::zeroed();
                    if ((*controller_vtable).get_parameter_info)(
                        controller,
                        i,
                        &mut info as *mut _ as *mut c_void,
                    ) == kResultOk
                    {
                        // Some sanity check
                        let title_len = info.title.iter().position(|&c| c == 0).unwrap_or(info.title.len());
                        let title = String::from_utf16_lossy(&info.title[..title_len]);
                        let val = ((*controller_vtable).get_param_normalized)(controller, info.id as i32);
                        let p = Parameter::new(&title, val, 0.0, 1.0);
                        b.parameters.push(p);
                        b.param_ids.push(info.id);
                    }
                }
            }
        }

        // 9. Install IComponentHandler so plugin GUI can push param changes back
        unsafe {
            if !b.editor_controller.is_null() {
                let fb = std::sync::Arc::clone(&b.param_feedback);
                let pdc = std::sync::Arc::clone(&b.pdc_recalc_needed);
                let mut handler = ComponentHandler::new(fb, pdc);
                let handler_ptr = ComponentHandler::as_com_ptr(&mut handler);
                b.component_handler = Some(handler);
                let controller_vtable = *(b.editor_controller as *mut *mut IEditControllerVtbl);
                ((*controller_vtable).set_component_handler)(b.editor_controller, handler_ptr);
            }
        }

        Ok(b)
    }

    fn process_maybach_internal(&mut self, buffer: &mut AudioBuffer, sample_rate: f64) {
        let frames = buffer.frames;
        let start = std::time::Instant::now();

        unsafe {
            let processor_vtable = *(self.processor as *mut *mut IAudioProcessorVtbl);
            
            // Sync parameters
            for (i, p) in self.parameters.iter().enumerate() {
                self.param_changes.set_parameter(self.param_ids[i], p.get_current_value());
            }

            if self.scratch.supports_f64 {
                // Prepare Input Buses
                let mut in_buses = Vec::with_capacity(self.num_input_buses as usize);
                let mut in_ptr_scratch = Vec::new();
                for b in 0..self.num_input_buses {
                    // Simple mapping: Bus 0 = channels 0,1; Bus 1 = channels 2,3 (if present)
                    let ch_offset = b * 2;
                    let mut bus_ptrs = Vec::new();
                    for c in 0..2 {
                        let ch_idx = (ch_offset as usize) + c;
                        if ch_idx < buffer.channels_data.len() {
                            bus_ptrs.push(buffer.channels_data[ch_idx].as_mut_ptr() as *mut c_void);
                        } else {
                            // Provide silence if host doesn't have enough channels for this bus
                            static mut SILENCE: [f64; 4096] = [0.0; 4096];
                            bus_ptrs.push(SILENCE.as_mut_ptr() as *mut c_void);
                        }
                    }
                    in_ptr_scratch.push(bus_ptrs);
                }
                for b in 0..self.num_input_buses as usize {
                    in_buses.push(AudioBusBuffers {
                        num_channels: 2,
                        silence_flags: 0,
                        buffers: in_ptr_scratch[b].as_mut_ptr() as *mut *mut c_void,
                    });
                }

                // Prepare Output Buses
                let mut out_buses = Vec::with_capacity(self.num_output_buses as usize);
                let mut out_ptr_scratch = Vec::new();
                for b in 0..self.num_output_buses {
                    let ch_offset = b * 2;
                    let mut bus_ptrs = Vec::new();
                    for c in 0..2 {
                        let ch_idx = (ch_offset as usize) + c;
                        if ch_idx < buffer.channels_data.len() {
                            bus_ptrs.push(buffer.channels_data[ch_idx].as_mut_ptr() as *mut c_void);
                        } else {
                            static mut DUMMY_OUT: [f64; 4096] = [0.0; 4096];
                            bus_ptrs.push(DUMMY_OUT.as_mut_ptr() as *mut c_void);
                        }
                    }
                    out_ptr_scratch.push(bus_ptrs);
                }
                for b in 0..self.num_output_buses as usize {
                    out_buses.push(AudioBusBuffers {
                        num_channels: 2,
                        silence_flags: 0,
                        buffers: out_ptr_scratch[b].as_mut_ptr() as *mut *mut c_void,
                    });
                }

                let mut data = ProcessData {
                    process_mode: 0,
                    symbolic_sample_size: kSample64,
                    num_samples: frames as i32,
                    num_inputs: self.num_input_buses,
                    num_outputs: self.num_output_buses,
                    inputs: in_buses.as_mut_ptr(),
                    outputs: out_buses.as_mut_ptr(),
                    parameter_changes: self.param_changes.as_ptr(),
                    event_list: self.event_list.as_ptr(),
                };

                ((*processor_vtable).process)(self.processor, &mut data);
            } else {
                // 32-bit fallback (simplified 1-bus stereo mapping for now)
                self.scratch.resize(frames);
                for i in 0..frames {
                    self.scratch.inputs_f32[0][i] = buffer.channels_data[0][i] as f32;
                    self.scratch.inputs_f32[1][i] = buffer.channels_data[1][i] as f32;
                }
                self.scratch.input_channels_f32[0] = self.scratch.inputs_f32[0].as_mut_ptr();
                self.scratch.input_channels_f32[1] = self.scratch.inputs_f32[1].as_mut_ptr();
                self.scratch.output_channels_f32[0] = self.scratch.outputs_f32[0].as_mut_ptr();
                self.scratch.output_channels_f32[1] = self.scratch.outputs_f32[1].as_mut_ptr();

                let mut in_bus = AudioBusBuffers {
                    num_channels: 2, silence_flags: 0, buffers: self.scratch.input_channels_f32.as_mut_ptr() as *mut *mut c_void,
                };
                let mut out_bus = AudioBusBuffers {
                    num_channels: 2, silence_flags: 0, buffers: self.scratch.output_channels_f32.as_mut_ptr() as *mut *mut c_void,
                };

                let mut data = ProcessData {
                    process_mode: 0,
                    symbolic_sample_size: kSample32,
                    num_samples: frames as i32,
                    num_inputs: 1,
                    num_outputs: 1,
                    inputs: &mut in_bus,
                    outputs: &mut out_bus,
                    parameter_changes: self.param_changes.as_ptr(),
                    event_list: self.event_list.as_ptr(),
                };

                ((*processor_vtable).process)(self.processor, &mut data);

                for i in 0..frames {
                    buffer.channels_data[0][i] = self.scratch.outputs_f32[0][i] as f64;
                    buffer.channels_data[1][i] = self.scratch.outputs_f32[1][i] as f64;
                }
            }

            // Update latency if changed
            let new_latency = ((*processor_vtable).get_latency_samples)(self.processor) as usize;
            if new_latency != self.reported_latency {
                self.reported_latency = new_latency;
                self.pdc_recalc_needed.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            // Clear block queues
            self.event_list.events.clear();
            self.param_changes.clear();
        }

        // --- CPU Performance Calculation ---
        let elapsed = start.elapsed().as_secs_f32();
        let total_time_available = frames as f32 / sample_rate as f32; 
        if total_time_available > 0.0 {
            let current_load = (elapsed / total_time_available).min(1.0);
            let old_load = f32::from_bits(self.cpu_usage.load(std::sync::atomic::Ordering::Relaxed));
            let smoothed_load = old_load * 0.9 + current_load * 0.1;
            self.cpu_usage.store(smoothed_load.to_bits(), std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Getsmoothed CPU usage percentage
    pub fn get_cpu_usage(&self) -> f32 {
        f32::from_bits(self.cpu_usage.load(std::sync::atomic::Ordering::Relaxed))
    }
}

impl AudioProcessor for Vst3Bridge {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        // 1. Drain GUI-driven parameter changes for Automation Recording
        if let Ok(mut q) = self.param_feedback.lock() {
            let changes = std::mem::take(&mut *q);
            for (vst3_id, value) in changes {
                // Find index of VST3 param_id
                let idx = self.param_ids.iter().position(|&pid| pid == vst3_id);
                if let Some(i) = idx {
                    let p = &mut self.parameters[i];
                    p.set_value(value);
                    
                    // Check if automation recording is active for this parameter
                    let curve_arc = p.curve.load();
                    if curve_arc.is_recording {
                        let mut new_curve = (**curve_arc).clone();
                        new_curve.record_value(context.playhead, value);
                        p.curve.store(std::sync::Arc::new(new_curve));
                    }
                }
            }
        }

        // 2. Perform DSP logic
        self.process_maybach_internal(buffer, context.sample_rate);
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        // VST3 instances are tied to physical DLLs and state, cannot be simply cloned.
        // Return a dummy placeholder with same metadata.
        Box::new(crate::engine::graph::DummyProcessor {
            id: self.id,
            name: self.name.clone(),
            parameters: self.parameters.clone(),
        })
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        self.parameters.iter_mut().collect()
    }

    fn latency_samples(&self) -> usize {
        self.reported_latency
    }

    fn needs_pdc_recalc(&self) -> bool {
        self.pdc_recalc_needed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn reset_pdc_recalc(&mut self) {
        self.pdc_recalc_needed.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    fn poll_editor_resize(&self) -> Option<(u32, u32)> {
        if let Ok(mut pending) = self.pending_resize.lock() {
            pending.take()
        } else {
            None
        }
    }

    fn on_midi_event(&mut self, status: u8, data1: u16, data2: u32) {
        let channel = (status & 0x0F) as u8;
        let kind    = status & 0xF0;
        let pitch   = (data1 & 0x7F) as u8;
        let vel     = (data2 & 0x7F) as u8;
        match kind {
            0x90 if vel > 0 => self.event_list.push_note_on(channel, pitch, vel, 0),
            0x80 | 0x90    => self.event_list.push_note_off(channel, pitch, vel, 0),
            _ => {}
        }
    }

    fn open_editor(&mut self, window_handle: *mut c_void) -> Option<(u32, u32)> {
        unsafe {
            if self.editor_controller.is_null() {
                // Try to get IEditController from IComponent
                let unknown = self.component as *mut *mut IUnknownVtbl;
                let mut controller: *mut c_void = std::ptr::null_mut();
                if ((*(*unknown)).query_interface)(
                    self.component,
                    &IEditController_UUID,
                    &mut controller,
                ) != kResultOk
                {
                    eprintln!("Failed to get IEditController");
                    return None;
                }
                self.editor_controller = controller;
            }

            if self.plug_view.is_null() {
                let controller_vtable = *(self.editor_controller as *mut *mut IEditControllerVtbl);
                let mut view: *mut c_void = std::ptr::null_mut();
                // Create view
                if ((*controller_vtable).create_view)(
                    self.editor_controller,
                    std::ptr::null_mut(), // type (default)
                    &mut view,
                ) == kResultOk
                {
                    self.plug_view = view;

                    // Attach
                    let view_vtable = *(self.plug_view as *mut *mut IPlugViewVtbl);
                    
                    #[cfg(target_os = "windows")]
                    let platform_type = b"HWND\0".as_ptr() as *const i8;
                    #[cfg(target_os = "macos")]
                    let platform_type = b"NSView\0".as_ptr() as *const i8;
                    #[cfg(target_os = "linux")]
                    let platform_type = b"X11EmbedWindowID\0".as_ptr() as *const i8;

                    let result =
                        ((*view_vtable).attached)(self.plug_view, window_handle, platform_type);
                    if result != kResultOk {
                        eprintln!("Failed to attach view: {}", result);
                    } else {
                        let mut rect = ViewRect { left: 0, top: 0, right: 0, bottom: 0 };
                        if ((*view_vtable).get_size)(self.plug_view, &mut rect as *mut ViewRect as *mut c_void) == kResultOk {
                            let w = (rect.right - rect.left) as u32;
                            let h = (rect.bottom - rect.top) as u32;

                            // DPI Scaling Support
                            let mut scale_support: *mut c_void = std::ptr::null_mut();
                            if ((*view_vtable).base.query_interface)(
                                self.plug_view,
                                &IPlugViewContentScaleSupport_UUID,
                                &mut scale_support,
                            ) == kResultOk
                            {
                                let scale_vtable = *(scale_support as *mut *mut IPlugViewContentScaleSupportVtbl);
                                // Set 100% scaling as default baseline; can be extended to detect OS scaling.
                                ((*scale_vtable).set_content_scale_factor)(scale_support, 1.0);
                                ((*scale_vtable).base.release)(scale_support);
                            }

                            // Set PlugFrame
                            if let Some(ref pf) = self.plug_frame {
                                ((*view_vtable).set_frame)(self.plug_view, pf.as_ref() as *const _ as *mut c_void);
                            }

                            return Some((w, h));
                        }
                    }
                } else {
                    eprintln!("Failed to create view");
                }
            }
        }
        None
    }

    fn close_editor(&mut self) {
        unsafe {
            if !self.plug_view.is_null() {
                let view_vtable = *(self.plug_view as *mut *mut IPlugViewVtbl);
                ((*view_vtable).removed)(self.plug_view);
                // We should release the view here if we owned references,
                // but typically we keep it alive for reopening or release on drop.
                // For now, minimal cleanup.
            }
        }
    }

    fn get_state(&self) -> Vec<u8> {
        let mut stream = MemoryStream::new();
        unsafe {
            let component_vtable = *(self.component as *mut *mut IComponentVtbl);
            ((*component_vtable).get_state)(self.component, &mut stream as *mut MemoryStream as *mut c_void);
        }
        stream.data.clone()
    }

    fn set_state(&mut self, data: &[u8]) {
        let mut stream = MemoryStream::from_data(data.to_vec());
        let stream_ptr = &mut stream as *mut MemoryStream as *mut c_void;
        unsafe {
            let component_vtable = *(self.component as *mut *mut IComponentVtbl);
            ((*component_vtable).set_state)(self.component, stream_ptr);

            // Also notify controller if it exists
            if !self.editor_controller.is_null() {
                // Rewind stream
                stream.cursor = 0;
                let controller_vtable = *(self.editor_controller as *mut *mut IEditControllerVtbl);
                ((*controller_vtable).set_component_state)(self.editor_controller, stream_ptr);
            }
        }
    }

    fn drain_plugin_feedback(&self) -> Vec<(String, f64)> {
        self.drain_param_feedback()
    }

    /// Getsmoothed CPU usage percentage
    fn get_cpu_usage(&self) -> f32 {
        f32::from_bits(self.cpu_usage.load(std::sync::atomic::Ordering::Relaxed))
    }

    fn get_programs(&self) -> Vec<String> {
        let mut programs = Vec::new();
        unsafe {
            let mut unit_info_ptr: *mut c_void = std::ptr::null_mut();
            let unknown = self.component as *mut *mut IUnknownVtbl;
            
            if ((*(*unknown)).query_interface)(
                self.component,
                &IUnitInfo_UUID,
                &mut unit_info_ptr,
            ) == kResultOk
            {
                let unit_info_vtable = *(unit_info_ptr as *mut *mut IUnitInfoVtbl);
                let list_count = ((*unit_info_vtable).get_program_list_count)(unit_info_ptr);
                
                if list_count > 0 {
                    let mut list_info: ProgramListInfo = std::mem::zeroed();
                    // Get first program list
                    if ((*unit_info_vtable).get_program_list_info)(unit_info_ptr, 0, &mut list_info) == kResultOk {
                        for i in 0..list_info.program_count {
                            let mut name_u16 = [0u16; 128];
                            if ((*unit_info_vtable).get_program_name)(unit_info_ptr, list_info.id, i, name_u16.as_mut_ptr()) == kResultOk {
                                let len = name_u16.iter().position(|&c| c == 0).unwrap_or(128);
                                programs.push(String::from_utf16_lossy(&name_u16[..len]));
                            }
                        }
                    }
                }
                ((*unit_info_vtable).base.release)(unit_info_ptr);
            }
        }
        programs
    }

    /// Sets the active program index for the plugin.
    fn set_program(&mut self, index: i32) {
        unsafe {
            let mut unit_info_ptr: *mut c_void = std::ptr::null_mut();
            let unknown = self.component as *mut *mut IUnknownVtbl;
            
            if ((*(*unknown)).query_interface)(
                self.component,
                &IUnitInfo_UUID,
                &mut unit_info_ptr,
            ) == kResultOk
            {
                let unit_info_vtable = *(unit_info_ptr as *mut *mut IUnitInfoVtbl);
                // Usually unit 0 is the root unit
                ((*unit_info_vtable).set_unit_program_index)(unit_info_ptr, 0, index);
                ((*unit_info_vtable).base.release)(unit_info_ptr);
            }
        }
    }
}

// --- Vst3Bridge host-API helpers ---
impl Vst3Bridge {
    /// Drain GUI-driven parameter changes queued by IComponentHandler::perform_edit.
    /// Call from the Tauri command `poll_plugin_param_changes` to sync plugin GUI → DAW UI.
    pub fn drain_param_feedback(&self) -> Vec<(String, f64)> {
        if let Ok(mut q) = self.param_feedback.lock() {
            let changes = std::mem::take(&mut *q);
            changes.into_iter().map(|(id, val)| {
                // Find index of VST3 param_id
                let idx = self.param_ids.iter().position(|&pid| pid == id);
                let param_uuid = if let Some(i) = idx {
                    self.parameters[i].id.to_string()
                } else {
                    "unknown".to_string()
                };
                (param_uuid, val)
            }).collect()
        } else {
            Vec::new()
        }
    }
}

// --- MemoryStream Implementation ---

#[repr(C)]
pub struct MemoryStream {
    vtable: *const IBStreamVtbl,
    ref_count: u32,
    data: Vec<u8>,
    cursor: usize,
}

impl MemoryStream {
    pub fn new() -> Self {
        Self {
            vtable: &MEMORY_STREAM_VTABLE,
            ref_count: 1,
            data: Vec::new(),
            cursor: 0,
        }
    }

    pub fn from_data(data: Vec<u8>) -> Self {
        Self {
            vtable: &MEMORY_STREAM_VTABLE,
            ref_count: 1,
            data,
            cursor: 0,
        }
    }
}

static MEMORY_STREAM_VTABLE: IBStreamVtbl = IBStreamVtbl {
    base: IUnknownVtbl {
        query_interface: memory_stream_query_interface,
        add_ref: memory_stream_add_ref,
        release: memory_stream_release,
    },
    read: memory_stream_read,
    write: memory_stream_write,
    seek: memory_stream_seek,
    tell: memory_stream_tell,
};

unsafe extern "system" fn memory_stream_query_interface(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> i32 {
    let iid = &*iid;
    // Check for IUnknown or IBStream
    if iid.data1 == IUnknown_UUID.data1 || iid.data1 == IBStream_UUID.data1 {
        memory_stream_add_ref(this);
        *out = this;
        return kResultOk;
    }
    kResultNoInterface
}

unsafe extern "system" fn memory_stream_add_ref(this: *mut c_void) -> u32 {
    let stream = &mut *(this as *mut MemoryStream);
    stream.ref_count += 1;
    stream.ref_count
}

unsafe extern "system" fn memory_stream_release(this: *mut c_void) -> u32 {
    let stream = &mut *(this as *mut MemoryStream);
    stream.ref_count -= 1;
    stream.ref_count
}

unsafe extern "system" fn memory_stream_read(
    this: *mut c_void,
    buffer: *mut c_void,
    num_bytes: i32,
    bytes_read: *mut i32,
) -> i32 {
    let stream = &mut *(this as *mut MemoryStream);
    let available = stream.data.len() - stream.cursor;
    let to_read = std::cmp::min(num_bytes as usize, available);

    if to_read > 0 {
        std::ptr::copy_nonoverlapping(
            stream.data.as_ptr().add(stream.cursor),
            buffer as *mut u8,
            to_read,
        );
        stream.cursor += to_read;
    }

    if !bytes_read.is_null() {
        *bytes_read = to_read as i32;
    }

    kResultOk
}

unsafe extern "system" fn memory_stream_write(
    this: *mut c_void,
    buffer: *const c_void,
    num_bytes: i32,
    bytes_written: *mut i32,
) -> i32 {
    let stream = &mut *(this as *mut MemoryStream);
    let bytes = std::slice::from_raw_parts(buffer as *const u8, num_bytes as usize);
    stream.data.extend_from_slice(bytes);
    stream.cursor += num_bytes as usize;

    if !bytes_written.is_null() {
        *bytes_written = num_bytes;
    }

    kResultOk
}

unsafe extern "system" fn memory_stream_seek(
    this: *mut c_void,
    pos: i64,
    mode: i32,
    result_pos: *mut i64,
) -> i32 {
    let stream = &mut *(this as *mut MemoryStream);
    match mode {
        0 => stream.cursor = pos as usize, // SeekSet
        1 => stream.cursor = (stream.cursor as i64 + pos) as usize, // SeekCur
        2 => stream.cursor = (stream.data.len() as i64 + pos) as usize, // SeekEnd
        _ => return -1,                    // kResultFalse
    }

    if !result_pos.is_null() {
        *result_pos = stream.cursor as i64;
    }

    kResultOk
}

unsafe extern "system" fn memory_stream_tell(this: *mut c_void, pos: *mut i64) -> i32 {
    let stream = &mut *(this as *mut MemoryStream);
    if !pos.is_null() {
        *pos = stream.cursor as i64;
    }
    kResultOk
}

// Helper constants
const kResultNoInterface: i32 = -2147467262; // E_NOINTERFACE
const IUnknown_UUID: GUID = GUID {
    data1: 0x00000000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};
