/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioUnit.h` (Audio Unit Services)
//!
//! [Audio Unit Programming Guide](https://developer.apple.com/library/archive/documentation/MusicAudio/Conceptual/AudioUnitProgrammingGuide/TheAudioUnit/TheAudioUnit.html)

use std::time::Instant;

use crate::audio::openal::al_types::{ALuint, ALvoid};
use crate::audio::openal::{AL_BUFFERS_PROCESSED, AL_PLAYING, AL_SOURCE_STATE};

use super::audio_components::{AURenderCallbackStruct, AudioComponentInstance};
use super::audio_queue::decode_buffer;
use super::audio_session;
use crate::abi::CallFromHost;
use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::audio_toolbox::audio_components;
use crate::frameworks::audio_toolbox::audio_queue::{
    is_supported_audio_format, log_if_broken_audio_format,
};
use crate::frameworks::carbon_core::{paramErr, OSStatus};
use crate::frameworks::core_audio_types::{AudioStreamBasicDescription, AudioTimeStamp, SMPTETime};
use crate::frameworks::core_foundation::cf_run_loop::CFRunLoopGetMain;
use crate::frameworks::foundation::ns_run_loop;
use crate::mem::{guest_size_of, ConstVoidPtr, MutPtr, MutVoidPtr, SafeRead};

pub type AudioUnit = AudioComponentInstance;
type AudioUnitPropertyID = u32;
type AudioUnitScope = u32;
type AudioUnitElement = u32;
type AudioUnitParameterID = u32;

#[repr(C, packed)]
pub struct AudioBufferList<const COUNT: usize> {
    pub number_buffers: u32,
    pub buffers: [AudioBuffer; COUNT],
}
unsafe impl SafeRead for AudioBufferList<1> {}
unsafe impl SafeRead for AudioBufferList<2> {}

#[repr(C, packed)]
pub struct AudioBuffer {
    pub number_channels: u32,
    pub data_byte_size: u32,
    pub data: MutVoidPtr,
}

// TODO: Other scopes
const kAudioUnitScope_Global: AudioUnitScope = 0;
const kAudioUnitScope_Input: AudioUnitScope = 1;
const kAudioUnitScope_Output: AudioUnitScope = 2;

const kAudioUnitProperty_SampleRate: AudioUnitPropertyID = 2;
const kAudioUnitProperty_SetRenderCallback: AudioUnitPropertyID = 23;
const kAudioUnitProperty_MaximumFramesPerSlice: AudioUnitPropertyID = 14;
const kAudioUnitProperty_StreamFormat: AudioUnitPropertyID = 8;
const kAudioUnitProperty_ElementCount: AudioUnitPropertyID = 11;
const kAudioUnitProperty_FactoryPresets: AudioUnitPropertyID = 24;
const kAudioUnitErr_PropertyNotInUse: OSStatus = -10850;

const kAudioOutputUnitProperty_EnableIO: AudioUnitPropertyID = 2003;
const kAudioUnitRenderAction_PreRender: u32 = 1 << 2;
const kAudioUnitRenderAction_PostRender: u32 = 1 << 3;
const kAudioTimeStampSampleTimeValid: u32 = 1;

fn AudioUnitAddRenderNotify(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_proc: audio_components::AURenderCallback,
    in_ref_con: ConstVoidPtr,
) -> OSStatus {
    let Some(audio_unit) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
    else {
        return paramErr;
    };

    audio_unit.render_notifications.push((in_proc, in_ref_con));
    log!(
        "AudioUnitAddRenderNotify({:?}, {:?}, {:?}) -> 0",
        in_unit,
        in_proc,
        in_ref_con
    );
    0
}

fn AudioUnitRemoveRenderNotify(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_proc: audio_components::AURenderCallback,
    in_ref_con: ConstVoidPtr,
) -> OSStatus {
    let Some(audio_unit) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
    else {
        return paramErr;
    };
    let Some(index) = audio_unit
        .render_notifications
        .iter()
        .position(|&(proc, ref_con)| {
            proc.addr_with_thumb_bit() == in_proc.addr_with_thumb_bit() && ref_con == in_ref_con
        })
    else {
        return paramErr;
    };

    audio_unit.render_notifications.remove(index);
    log!(
        "AudioUnitRemoveRenderNotify({:?}, {:?}, {:?}) -> 0",
        in_unit,
        in_proc,
        in_ref_con
    );
    0
}

pub(super) fn AudioUnitInitialize(env: &mut Environment, in_unit: AudioUnit) -> OSStatus {
    let run_loop = CFRunLoopGetMain(env);
    ns_run_loop::add_audio_unit(env, run_loop, in_unit);
    0 // success
}

pub(super) fn AudioUnitUninitialize(env: &mut Environment, in_unit: AudioUnit) -> OSStatus {
    let run_loop = CFRunLoopGetMain(env);
    match ns_run_loop::remove_audio_unit(env, run_loop, in_unit) {
        Ok(_) => 0,
        Err(_) => paramErr, // TODO: handle different errors
    }
}

fn AudioUnitSetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    in_data: ConstVoidPtr,
    in_data_size: u32,
) -> OSStatus {
    let host_object = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
        .unwrap();

    let result;
    match in_id {
        kAudioUnitProperty_SetRenderCallback => {
            assert_eq!(in_scope, kAudioUnitScope_Global);
            assert_eq!(in_data_size, guest_size_of::<AURenderCallbackStruct>());
            let render_callback = env.mem.read(in_data.cast::<AURenderCallbackStruct>());
            host_object.render_callback = Some(render_callback);
            let AURenderCallbackStruct {
                input_proc,
                input_proc_ref_con,
            } = render_callback;
            result = 0;
            log!("AudioUnitSetProperty({:?}, SetRenderCallback, scope {:?}, element {:?}, callback {:?}, refcon {:?}) -> 0", in_unit, in_scope, in_element, input_proc, input_proc_ref_con);
        }
        kAudioUnitProperty_StreamFormat => {
            assert_eq!(in_data_size, guest_size_of::<AudioStreamBasicDescription>());
            let stream_format = env.mem.read(in_data.cast::<AudioStreamBasicDescription>());
            log_if_broken_audio_format(&stream_format);
            match in_scope {
                kAudioUnitScope_Global => host_object.global_stream_format = stream_format,
                kAudioUnitScope_Output => host_object.output_stream_format = Some(stream_format),
                kAudioUnitScope_Input => {
                    host_object.input_stream_format = Some(stream_format);
                    host_object
                        .input_stream_formats
                        .insert(in_element, stream_format);
                }
                _ => unimplemented!("in_scope {}", in_scope),
            };
            result = 0;
            log!(
                "AudioUnitSetProperty({:?}, StreamFormat, scope {}, element {}): {:?}",
                in_unit,
                in_scope,
                in_element,
                stream_format
            );
        }
        kAudioUnitProperty_SampleRate => {
            assert_eq!(in_data_size, guest_size_of::<f64>());
            let sample_rate = env.mem.read(in_data.cast::<f64>());
            match in_scope {
                kAudioUnitScope_Global => {
                    host_object.global_stream_format.sample_rate = sample_rate;
                }
                kAudioUnitScope_Input => {
                    let mut format = host_object
                        .input_stream_formats
                        .get(&in_element)
                        .copied()
                        .or(host_object.input_stream_format)
                        .unwrap_or(host_object.global_stream_format);
                    format.sample_rate = sample_rate;
                    host_object.input_stream_format = Some(format);
                    host_object.input_stream_formats.insert(in_element, format);
                }
                kAudioUnitScope_Output => {
                    let mut format = host_object
                        .output_stream_format
                        .unwrap_or(host_object.global_stream_format);
                    format.sample_rate = sample_rate;
                    host_object.output_stream_format = Some(format);
                }
                _ => return paramErr,
            }
            result = 0;
        }
        kAudioOutputUnitProperty_EnableIO => {
            assert_eq!(in_data_size, guest_size_of::<u32>());
            let enabled = env.mem.read(in_data.cast::<u32>());
            match (in_scope, in_element) {
                (kAudioUnitScope_Output, 0) => host_object.output_enabled = enabled != 0,
                (kAudioUnitScope_Input, 1) => host_object.input_enabled = enabled != 0,
                _ => return paramErr,
            }
            result = 0;
            log_dbg!("AudioUnitSetProperty({:?}, kAudioOutputUnitProperty_EnableIO, {:?}, {:?}, {:?}, {:?}) -> {:?}", in_unit, in_scope, in_element, enabled, in_data_size, result);
        }
        kAudioUnitProperty_ElementCount => {
            assert_eq!(in_data_size, guest_size_of::<u32>());
            let count = env.mem.read(in_data.cast::<u32>());
            match in_scope {
                kAudioUnitScope_Input => host_object.input_element_count = count,
                kAudioUnitScope_Output => host_object.output_element_count = count,
                _ => return paramErr,
            }
            result = 0;
        }
        kAudioUnitProperty_MaximumFramesPerSlice => {
            assert_eq!(in_scope, kAudioUnitScope_Global);
            assert_eq!(in_data_size, guest_size_of::<u32>());
            host_object.maximum_frames_per_slice = env.mem.read(in_data.cast::<u32>());
            result = 0;
        }
        _ => unimplemented!(
            "AudioUnitSetProperty id {} scope {} element {} size {}",
            in_id,
            in_scope,
            in_element,
            in_data_size
        ),
    };

    result
}

fn AudioUnitGetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    out_data: MutVoidPtr,
    io_data_size: MutPtr<u32>,
) -> OSStatus {
    let host_object = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
        .unwrap();

    match in_id {
        kAudioUnitProperty_MaximumFramesPerSlice => {
            assert_eq!(env.mem.read(io_data_size), guest_size_of::<u32>());
            let max_frames: u32 = host_object.maximum_frames_per_slice;
            env.mem.write(out_data.cast(), max_frames);
            env.mem.write(io_data_size.cast(), guest_size_of::<u32>());
        }
        kAudioUnitProperty_ElementCount => {
            assert_eq!(env.mem.read(io_data_size), guest_size_of::<u32>());
            let count = match in_scope {
                kAudioUnitScope_Input => host_object.input_element_count,
                kAudioUnitScope_Output => host_object.output_element_count,
                _ => return paramErr,
            };
            env.mem.write(out_data.cast(), count);
            env.mem.write(io_data_size.cast(), guest_size_of::<u32>());
        }
        kAudioUnitProperty_StreamFormat => {
            assert_eq!(
                env.mem.read(io_data_size),
                guest_size_of::<AudioStreamBasicDescription>()
            );
            let stream_format = match in_scope {
                kAudioUnitScope_Global => host_object.global_stream_format,
                kAudioUnitScope_Output => host_object.output_stream_format.unwrap(),
                kAudioUnitScope_Input => host_object
                    .input_stream_formats
                    .get(&in_element)
                    .copied()
                    .or(host_object.input_stream_format)
                    .unwrap(),
                _ => unimplemented!(),
            };
            env.mem.write(out_data.cast(), stream_format);
            env.mem.write(
                io_data_size.cast(),
                guest_size_of::<AudioStreamBasicDescription>(),
            );
        }
        kAudioUnitProperty_SampleRate => {
            assert_eq!(env.mem.read(io_data_size), guest_size_of::<f64>());
            let sample_rate = match in_scope {
                kAudioUnitScope_Global => host_object.global_stream_format.sample_rate,
                kAudioUnitScope_Output => {
                    host_object
                        .output_stream_format
                        .unwrap_or(host_object.global_stream_format)
                        .sample_rate
                }
                kAudioUnitScope_Input => {
                    host_object
                        .input_stream_formats
                        .get(&in_element)
                        .copied()
                        .or(host_object.input_stream_format)
                        .unwrap_or(host_object.global_stream_format)
                        .sample_rate
                }
                _ => unimplemented!(),
            };
            env.mem.write(out_data.cast(), sample_rate);
            env.mem.write(io_data_size.cast(), guest_size_of::<f64>());
        }
        kAudioUnitProperty_FactoryPresets => return kAudioUnitErr_PropertyNotInUse,
        _ => unimplemented!("in_id {}", in_id),
    };
    0 // success
}

pub(super) fn AudioUnitSetParameter(
    env: &mut Environment,
    unit: AudioUnit,
    parameter_id: AudioUnitParameterID,
    scope: AudioUnitScope,
    element: AudioUnitElement,
    value: f32,
    _buffer_offset: u32,
) -> OSStatus {
    let Some(host_object) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&unit)
    else {
        return paramErr;
    };
    host_object
        .parameter_values
        .insert((parameter_id, scope, element), value);
    log!(
        "AudioUnitSetParameter({:?}, parameter {}, scope {}, element {}) = {}",
        unit,
        parameter_id,
        scope,
        element,
        value
    );
    0
}

pub(super) fn AudioOutputUnitStart(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let context = env
        .framework_state
        .audio_toolbox
        .make_al_context_current(env.openal_manager.as_mut());

    let mut source: ALuint = 0;
    unsafe {
        context.GenSources(1, &mut source);
        context.SourcePlay(source);
        assert_eq!(context.GetError(), 0);
    }

    let audio_components_state = audio_components::State::get(&mut env.framework_state);
    let audio_unit_state = audio_components_state
        .audio_component_instances
        .get_mut(&ci)
        .unwrap();
    audio_unit_state.al_source = Some(source);
    audio_unit_state.last_render_time = Some(Instant::now());
    audio_unit_state.started = true;
    audio_unit_state.has_logged_first_render = false;
    audio_unit_state.has_logged_nonzero_render = false;
    audio_unit_state.sample_time = 0.0;

    let result = 0; // Success
    log!(
        "AudioOutputUnitStart({:?}) -> {:?}; render callback configured: {}, notifications: {}, stream format: {:?}",
        ci,
        result,
        audio_unit_state.render_callback.is_some(),
        audio_unit_state.render_notifications.len(),
        audio_unit_state.global_stream_format
    );
    result
}

pub(super) fn AudioOutputUnitStop(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let at_state = &mut env.framework_state.audio_toolbox;
    let context = at_state
        .al_context
        .make_al_context_current(env.openal_manager.as_mut());

    let audio_components_state = &mut at_state.audio_components;

    let result = if let Some(audio_unit_state) = audio_components_state
        .audio_component_instances
        .get_mut(&ci)
    {
        audio_unit_state.started = false;
        audio_unit_state.last_render_time = None;

        if let Some(al_source) = audio_unit_state.al_source {
            unsafe {
                context.DeleteSources(1, &al_source);
                assert_eq!(context.GetError(), 0);
            }
        }
        audio_unit_state.al_source = None;
        0 // success
    } else {
        -1
    };
    log_dbg!("AudioOutputUnitStop({:?}) -> {:?}", ci, result);
    result
}

pub fn read_normalized_sample(sample: &[u8], format: &AudioStreamBasicDescription) -> f32 {
    use crate::frameworks::core_audio_types::{
        kAudioFormatFlagIsFloat, kAudioFormatFlagIsSignedInteger,
    };

    let format_flags = format.format_flags;
    let bits_per_channel = format.bits_per_channel;
    if format_flags & kAudioFormatFlagIsFloat != 0 {
        let value = f32::from_le_bytes(sample.try_into().unwrap());
        return if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            0.0
        };
    }

    let signed = format_flags & kAudioFormatFlagIsSignedInteger != 0;
    match (bits_per_channel, signed) {
        (8, true) => i8::from_le_bytes(sample.try_into().unwrap()) as f32 / 128.0,
        (8, false) => (f32::from(sample[0]) - 128.0) / 128.0,
        (16, true) => i16::from_le_bytes(sample.try_into().unwrap()) as f32 / 32768.0,
        (16, false) => {
            (f32::from(u16::from_le_bytes(sample.try_into().unwrap())) - 32768.0) / 32768.0
        }
        (32, true) => (i32::from_le_bytes(sample.try_into().unwrap()) as f64 / 2147483648.0) as f32,
        (32, false) => {
            ((u32::from_le_bytes(sample.try_into().unwrap()) as f64 - 2147483648.0) / 2147483648.0)
                as f32
        }
        _ => unimplemented!("unsupported PCM sample depth {}", bits_per_channel),
    }
}

fn write_normalized_sample(sample: f32, output: &mut [u8], format: &AudioStreamBasicDescription) {
    use crate::frameworks::core_audio_types::{
        kAudioFormatFlagIsFloat, kAudioFormatFlagIsSignedInteger,
    };

    let format_flags = format.format_flags;
    let bits_per_channel = format.bits_per_channel;
    let sample = sample.clamp(-1.0, 1.0);
    if format_flags & kAudioFormatFlagIsFloat != 0 {
        output.copy_from_slice(&sample.to_le_bytes());
        return;
    }

    let signed = format_flags & kAudioFormatFlagIsSignedInteger != 0;
    match (bits_per_channel, signed) {
        (8, true) => output.copy_from_slice(&((sample * 128.0).round() as i8).to_le_bytes()),
        (8, false) => output[0] = (sample * 128.0 + 128.0).round().clamp(0.0, 255.0) as u8,
        (16, true) => output.copy_from_slice(
            &((sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16).to_le_bytes(),
        ),
        (16, false) => output.copy_from_slice(
            &((sample * 32768.0 + 32768.0).round().clamp(0.0, 65535.0) as u16).to_le_bytes(),
        ),
        (32, true) => output.copy_from_slice(
            &((sample as f64 * 2147483648.0)
                .round()
                .clamp(-2147483648.0, 2147483647.0) as i32)
                .to_le_bytes(),
        ),
        (32, false) => output.copy_from_slice(
            &((sample as f64 * 2147483648.0 + 2147483648.0)
                .round()
                .clamp(0.0, 4294967295.0) as u32)
                .to_le_bytes(),
        ),
        _ => unimplemented!("unsupported PCM sample depth {}", bits_per_channel),
    }
}

fn mix_input_into_output(
    input: &[u8],
    input_format: &AudioStreamBasicDescription,
    output: &mut [u8],
    output_format: &AudioStreamBasicDescription,
    number_frames: u32,
    gain: f32,
) {
    let input_bytes_per_sample = (input_format.bits_per_channel / 8) as usize;
    let output_bytes_per_sample = (output_format.bits_per_channel / 8) as usize;
    let input_channels = input_format.channels_per_frame as usize;
    let output_channels = output_format.channels_per_frame as usize;

    for frame in 0..number_frames as usize {
        for output_channel in 0..output_channels {
            let input_channel = if input_channels == 1 {
                0
            } else {
                output_channel.min(input_channels - 1)
            };
            let input_offset = frame * input_format.bytes_per_frame as usize
                + input_channel * input_bytes_per_sample;
            let output_offset = frame * output_format.bytes_per_frame as usize
                + output_channel * output_bytes_per_sample;
            let input_sample = read_normalized_sample(
                &input[input_offset..input_offset + input_bytes_per_sample],
                input_format,
            );
            let output_sample = read_normalized_sample(
                &output[output_offset..output_offset + output_bytes_per_sample],
                output_format,
            );
            write_normalized_sample(
                output_sample + input_sample * gain,
                &mut output[output_offset..output_offset + output_bytes_per_sample],
                output_format,
            );
        }
    }
}

pub fn render_audio_unit(env: &mut Environment, audio_unit: AudioUnit) {
    if env.bundle.bundle_identifier().starts_with("com.ea.simcity") {
        // If enabled, we have some random crashes inside AURenderCallback ;(
        log_dbg!("Applying game-specific hack for SimCity: skipping rendering of audio units");
        return;
    }

    let at_state = &mut env.framework_state.audio_toolbox;
    let context = at_state
        .al_context
        .make_al_context_current(env.openal_manager.as_mut());

    let audio_session::State {
        current_hardware_sample_rate,
        ..
    } = at_state.audio_session;

    let audio_components_state = &mut at_state.audio_components;
    let audio_unit_host_object = audio_components_state
        .audio_component_instances
        .get_mut(&audio_unit)
        .unwrap();

    if !audio_unit_host_object.started {
        return;
    }

    if audio_unit_host_object.is_running_handler {
        return;
    }

    let render_callback = audio_unit_host_object.render_callback;
    let graph_input_callbacks = audio_unit_host_object.graph_input_callbacks.clone();
    if render_callback.is_none() && graph_input_callbacks.is_empty() {
        log_dbg!(
            "AudioUnit {:?} is started without a render callback; skipping this render",
            audio_unit
        );
        return;
    }
    let render_notifications = audio_unit_host_object.render_notifications.clone();
    let has_logged_first_render = audio_unit_host_object.has_logged_first_render;
    let has_logged_nonzero_render = audio_unit_host_object.has_logged_nonzero_render;
    let sample_time = audio_unit_host_object.sample_time;

    audio_unit_host_object.is_running_handler = true;

    let input_stream_format = audio_unit_host_object.input_stream_format;
    let output_stream_format = audio_unit_host_object.output_stream_format;
    let stream_format = if input_stream_format.is_some()
        && output_stream_format.is_some()
        && input_stream_format != output_stream_format
    {
        unimplemented!("AudioUnit {:?} has non default and different input {:?} and output {:?} stream formats, conversion is needed", audio_unit, input_stream_format, output_stream_format);
    } else {
        // For purposes, the only important part is that format is supported
        // and playable by OpenAL. Thus, it doesn't really matter if input or
        // output format is defined by the application.
        // (but not both at the same time, see the check above)
        input_stream_format
            .unwrap_or(output_stream_format.unwrap_or(audio_unit_host_object.global_stream_format))
    };
    let sample_rate = if let Some(input_stream_format) = input_stream_format {
        input_stream_format.sample_rate
    } else if output_stream_format.is_some() {
        // Output units use the current hardware rate when the app configures
        // an output stream format. If it does not, use the configured global
        // stream format instead.
        current_hardware_sample_rate
    } else {
        stream_format.sample_rate
    };

    assert!(is_supported_audio_format(&stream_format));

    let al_source = audio_unit_host_object.al_source.unwrap();
    let mut al_buffers = Vec::new();
    unsafe {
        let mut buffers_processed = 0;
        context.GetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut buffers_processed);
        while buffers_processed > 0 {
            let mut al_buffer = 0;
            context.SourceUnqueueBuffers(al_source, 1, &mut al_buffer);
            al_buffers.push(al_buffer);
            context.GetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut buffers_processed);
        }
        assert_eq!(context.GetError(), 0);
    }

    let now = Instant::now();

    // Calculate number of frames by checking how much time passed since
    // the last render. Limit to 100ms to prevent delay from adding up
    // if it's been too long since the last render.
    // Ace Combat Xi relies on it being 2048 frames (at 48000Hz, 42ms) or under
    // If it's higher, flawed game logic causes it to call memset in a loop for
    // every frame over 2048 until it reaches the provided frame number.
    // TODO: Verify if this behavior is right
    let elapsed_time = now.duration_since(audio_unit_host_object.last_render_time.unwrap());
    let number_frames = ((elapsed_time.as_secs_f64() * sample_rate) as u32).min(2048);

    let bytes_per_channel = stream_format.bits_per_channel / 8;
    let actual_bytes_per_frame = stream_format.channels_per_frame * bytes_per_channel;

    let buffer_size = number_frames * actual_bytes_per_frame;

    // Alloc callback arguments
    let action_flags = env.mem.alloc_and_write(0);
    let time_stamp = env.mem.alloc_and_write(AudioTimeStamp {
        sample_time,
        host_time: 0,
        rate_scalar: 1.0,
        world_clock_type: 0,
        SMPTE_time: SMPTETime {
            subframes: 0,
            subframe_divisor: 0,
            counter: 0,
            type_: 0,
            flags: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
            frames: 0,
        },
        flags: kAudioTimeStampSampleTimeValid,
        _reserved: 0,
    });

    let (audio_buffer_list, buffer1Data, buffer2Data): (
        MutVoidPtr,
        MutVoidPtr,
        Option<MutVoidPtr>,
    ) = if input_stream_format.is_some()
        || !graph_input_callbacks.is_empty()
        || env.bundle.bundle_identifier() == "com.upasani.iTablaPro"
    {
        let bufferData = env.mem.alloc(buffer_size);
        let audio_buffer_list: AudioBufferList<1> = AudioBufferList {
            number_buffers: 1,
            buffers: [AudioBuffer {
                number_channels: stream_format.channels_per_frame,
                data_byte_size: buffer_size,
                data: bufferData,
            }],
        };
        (
            env.mem.alloc_and_write(audio_buffer_list).cast(),
            bufferData,
            None,
        )
    } else {
        // Resident Evil 4 expects 2 buffers
        // though it copies the same data to both
        let buffer1Data = env.mem.alloc(buffer_size);
        let buffer2Data = env.mem.alloc(buffer_size);
        let audio_buffer_list: AudioBufferList<2> = AudioBufferList {
            number_buffers: 2,
            buffers: [
                AudioBuffer {
                    number_channels: stream_format.channels_per_frame,
                    data_byte_size: buffer_size,
                    data: buffer1Data,
                },
                AudioBuffer {
                    number_channels: stream_format.channels_per_frame,
                    data_byte_size: buffer_size,
                    data: buffer2Data,
                },
            ],
        };
        (
            env.mem.alloc_and_write(audio_buffer_list).cast(),
            buffer1Data,
            Some(buffer2Data),
        )
    };

    // Run render notifications and callback.
    env.mem
        .write(action_flags, kAudioUnitRenderAction_PreRender);
    for &(notify, ref_con) in &render_notifications {
        let _: OSStatus = notify.call_from_host(
            env,
            (
                ref_con,
                action_flags,
                time_stamp.cast_const(),
                0u32,
                number_frames,
                audio_buffer_list,
            ),
        );
    }

    env.mem.write(action_flags, 0);
    let mut callback_statuses = Vec::new();
    let mut callback_audio_stats = Vec::new();
    if graph_input_callbacks.is_empty() {
        let AURenderCallbackStruct {
            input_proc,
            input_proc_ref_con,
        } = render_callback.unwrap();
        let callback_status: OSStatus = input_proc.call_from_host(
            env,
            (
                input_proc_ref_con,
                action_flags,
                time_stamp.cast_const(),
                0u32,
                number_frames,
                audio_buffer_list,
            ),
        );
        callback_statuses.push((0, callback_status));
    } else {
        env.mem
            .bytes_at_mut(buffer1Data.cast(), buffer_size)
            .fill(0);
        for (source_audio_unit, bus, callback, input_format) in &graph_input_callbacks {
            let parameters = &audio_components::State::get(&mut env.framework_state)
                .audio_component_instances
                .get(source_audio_unit)
                .unwrap()
                .parameter_values;
            let gain_db = parameters
                .get(&(3, kAudioUnitScope_Input, *bus))
                .copied()
                .unwrap_or(0.0);
            let gain = 10.0f32.powf(gain_db / 20.0);
            env.mem.write(action_flags, 0);
            let input_buffer_size = number_frames * input_format.bytes_per_frame;
            let scratch_buffer = env.mem.alloc(input_buffer_size);
            let audio_buffer_list_for_bus = env.mem.alloc_and_write(AudioBufferList::<1> {
                number_buffers: 1,
                buffers: [AudioBuffer {
                    number_channels: input_format.channels_per_frame,
                    data_byte_size: input_buffer_size,
                    data: scratch_buffer,
                }],
            });
            let AURenderCallbackStruct {
                input_proc,
                input_proc_ref_con,
            } = *callback;
            let callback_status: OSStatus = input_proc.call_from_host(
                env,
                (
                    input_proc_ref_con,
                    action_flags,
                    time_stamp.cast_const(),
                    *bus,
                    number_frames,
                    audio_buffer_list_for_bus.cast::<MutVoidPtr>(),
                ),
            );
            callback_statuses.push((*bus, callback_status));

            let input = env
                .mem
                .bytes_at(scratch_buffer.cast(), input_buffer_size)
                .to_vec();
            let nonzero_bytes = input.iter().filter(|&&byte| byte != 0).count();
            if nonzero_bytes > 0 {
                callback_audio_stats.push((*bus, nonzero_bytes, *input_format));
            }
            let output = env.mem.bytes_at_mut(buffer1Data.cast(), buffer_size);
            mix_input_into_output(
                &input,
                input_format,
                output,
                &stream_format,
                number_frames,
                gain,
            );
            env.mem.free(scratch_buffer.cast_void());
            env.mem.free(audio_buffer_list_for_bus.cast());
        }
    }

    if !has_logged_first_render && number_frames > 0 {
        let nonzero_bytes = env
            .mem
            .bytes_at(buffer1Data.cast(), buffer_size)
            .iter()
            .filter(|&&byte| byte != 0)
            .count();
        log!(
            "AudioUnit {:?} first render: {} frames at {} Hz, callback statuses {:?}, {} of {} output bytes nonzero",
            audio_unit,
            number_frames,
            sample_rate,
            callback_statuses,
            nonzero_bytes,
            buffer_size
        );
        audio_components::State::get(&mut env.framework_state)
            .audio_component_instances
            .get_mut(&audio_unit)
            .unwrap()
            .has_logged_first_render = true;
        if nonzero_bytes > 0 {
            audio_components::State::get(&mut env.framework_state)
                .audio_component_instances
                .get_mut(&audio_unit)
                .unwrap()
                .has_logged_nonzero_render = true;
        }
    } else if !has_logged_nonzero_render && number_frames > 0 {
        let nonzero_bytes = env
            .mem
            .bytes_at(buffer1Data.cast(), buffer_size)
            .iter()
            .filter(|&&byte| byte != 0)
            .count();
        if nonzero_bytes > 0 {
            log!(
                "AudioUnit {:?} produced nonzero audio: {} of {} bytes, callback statuses {:?}, per-bus (bus, nonzero bytes, stream format) {:?}",
                audio_unit,
                nonzero_bytes,
                buffer_size,
                callback_statuses,
                callback_audio_stats
            );
            audio_components::State::get(&mut env.framework_state)
                .audio_component_instances
                .get_mut(&audio_unit)
                .unwrap()
                .has_logged_nonzero_render = true;
        }
    }

    env.mem
        .write(action_flags, kAudioUnitRenderAction_PostRender);
    for &(notify, ref_con) in &render_notifications {
        let _: OSStatus = notify.call_from_host(
            env,
            (
                ref_con,
                action_flags,
                time_stamp.cast_const(),
                0u32,
                number_frames,
                audio_buffer_list,
            ),
        );
    }

    let at_state = &mut env.framework_state.audio_toolbox;
    let context = at_state
        .al_context
        .make_al_context_current(env.openal_manager.as_mut());

    let (al_format, _sample_rate, processed_data) =
        decode_buffer(&env.mem, &stream_format, buffer1Data.cast(), buffer_size);

    unsafe {
        // Get an unqueued buffer or create a new one
        let al_buffer = al_buffers.pop().unwrap_or_else(|| {
            let mut al_buffer = 0;
            context.GenBuffers(1, &mut al_buffer);
            al_buffer
        });

        context.BufferData(
            al_buffer,
            al_format,
            processed_data.as_ptr() as *const ALvoid,
            processed_data.len().try_into().unwrap(),
            sample_rate as i32,
        );
        context.SourceQueueBuffers(al_source, 1, &al_buffer);

        let mut al_source_state = 0;
        context.GetSourcei(al_source, AL_SOURCE_STATE, &mut al_source_state);
        if al_source_state != AL_PLAYING {
            context.SourcePlay(al_source);
        }

        // TODO: Play buffer 2 (In RE4 its the same as buffer 1 though)

        // Clear unused buffers
        if !al_buffers.is_empty() {
            context.DeleteBuffers(al_buffers.len() as i32, al_buffers.as_ptr());
        }

        assert_eq!(context.GetError(), 0);
    }

    // TODO: Do something with the action flags?
    env.mem.free(action_flags.cast_void());
    env.mem.free(time_stamp.cast_void());

    env.mem.free(buffer1Data.cast_void());
    if let Some(buffer2Data) = buffer2Data {
        env.mem.free(buffer2Data.cast_void());
    }

    env.mem.free(audio_buffer_list.cast_void());

    let audio_unit_host_object = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&audio_unit)
        .unwrap();
    // Reborrow as mutable to update the last render time

    audio_unit_host_object.last_render_time = Some(now);
    audio_unit_host_object.sample_time += f64::from(number_frames);
    audio_unit_host_object.is_running_handler = false;
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioUnitInitialize(_)),
    export_c_func!(AudioUnitUninitialize(_)),
    export_c_func!(AudioUnitSetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitSetParameter(_, _, _, _, _, _)),
    export_c_func!(AudioOutputUnitStart(_)),
    export_c_func!(AudioOutputUnitStop(_)),
    export_c_func!(AudioUnitAddRenderNotify(_, _, _)),
    export_c_func!(AudioUnitRemoveRenderNotify(_, _, _)),
];
