/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! AVAudioSession

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::ns_string;
use crate::mem::{GuestUSize, MutPtr};
use crate::objc::{id, objc_classes, ClassExports, TrivialHostObject};
use crate::todo_objc_setter;

type AVAudioSessionCategory = id; // NSString *
type AVAudioSessionMode = id; // NSString *

#[derive(Default)]
pub struct State {
    /// [AVAudioSession sharedInstance]
    shared_instance: Option<id>,
    category: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// This is a singleton.
@implementation AVAudioSession: NSObject

+ (id)sharedInstance {
    if let Some(audio_session) =
        env.framework_state.avfoundation.av_audio_session.shared_instance {
        audio_session
    } else {
        let new = env.objc.alloc_static_object(
            this,
            Box::new(TrivialHostObject),
            &mut env.mem
        );
        env.framework_state.avfoundation.av_audio_session.shared_instance = Some(new);
        new
    }
}

- (bool)isInputAvailable {
    false
}

- (bool)isInputGainSettable {
    false
}

- (id)category {
    env.framework_state
        .avfoundation
        .av_audio_session
        .category
        .unwrap_or_else(|| ns_string::get_static_str(env, AVAudioSessionCategorySoloAmbient))
}

- (id)retain { this }
- (())release {}
- (id)autorelease { this }

- (())setDelegate:(id)delegate {
    todo_objc_setter!(this, delegate);
}

- (bool)setCategory:(AVAudioSessionCategory)category
              error:(MutPtr<id>)error { // NSError **
    env.framework_state.avfoundation.av_audio_session.category = Some(category);
    log!(
        "TODO: [(AVAudioSession *){:?} setCategory:'{}' error:{:?}] -> true",
        this,
        ns_string::to_rust_string(env, category),
        error
    );
    true
}

- (bool)setCategory:(AVAudioSessionCategory)category
        withOptions:(GuestUSize)options
              error:(MutPtr<id>)error { // NSError **
    env.framework_state.avfoundation.av_audio_session.category = Some(category);
    log!(
        "TODO: [(AVAudioSession *){:?} setCategory:'{}' withOptions:{:#x} error:{:?}] -> true",
        this,
        ns_string::to_rust_string(env, category),
        options,
        error
    );
    true
}

- (bool)setMode:(AVAudioSessionMode)mode
          error:(MutPtr<id>)error { // NSError **
    log!(
        "TODO: [(AVAudioSession *){:?} setMode:'{}' error:{:?}] -> true",
        this,
        ns_string::to_rust_string(env, mode),
        error
    );
    true
}

- (bool)setActive:(bool)active
            error:(MutPtr<id>)error { // NSError **
    log!(
        "TODO: [(AVAudioSession *){:?} setActive:{} error:{:?}] -> true",
        this,
        active,
        error
    );
    true
}

- (bool)setPreferredHardwareSampleRate:(f64)sample_rate
                                 error:(MutPtr<id>)error { // NSError **
    env.framework_state
        .audio_toolbox
        .set_current_hardware_sample_rate(sample_rate);
    log!(
        "TODO: [(AVAudioSession *){:?} setPreferredHardwareSampleRate:{} error:{:?}] -> true",
        this,
        sample_rate,
        error
    );
    true
}

- (bool)setPreferredSampleRate:(f64)sample_rate
                         error:(MutPtr<id>)error { // NSError **
    env.framework_state
        .audio_toolbox
        .set_current_hardware_sample_rate(sample_rate);
    log!(
        "TODO: [(AVAudioSession *){:?} setPreferredSampleRate:{} error:{:?}] -> true",
        this,
        sample_rate,
        error
    );
    true
}

- (f64)preferredSampleRate {
    env.framework_state.audio_toolbox.current_hardware_sample_rate()
}

- (bool)setPreferredIOBufferDuration:(f64)duration
                               error:(MutPtr<id>)error { // NSError **
    env.framework_state
        .audio_toolbox
        .set_current_hardware_io_buffer_duration(duration);
    log!(
        "TODO: [(AVAudioSession *){:?} setPreferredIOBufferDuration:{} error:{:?}] -> true",
        this,
        duration,
        error
    );
    true
}

- (f64)preferredIOBufferDuration {
    env.framework_state
        .audio_toolbox
        .current_hardware_io_buffer_duration()
}

@end

};

// Values might not be correct, but as these are linked symbol constants, it
// shouldn't matter.
const AVAudioSessionCategoryAmbient: &str = "AVAudioSessionCategoryAmbient";
const AVAudioSessionCategoryMultiRoute: &str = "AVAudioSessionCategoryMultiRoute";
const AVAudioSessionCategoryPlayAndRecord: &str = "AVAudioSessionCategoryPlayAndRecord";
const AVAudioSessionCategoryPlayback: &str = "AVAudioSessionCategoryPlayback";
const AVAudioSessionCategoryRecord: &str = "AVAudioSessionCategoryRecord";
const AVAudioSessionCategorySoloAmbient: &str = "AVAudioSessionCategorySoloAmbient";
const AVAudioSessionCategoryAudioProcessing: &str = "AVAudioSessionCategoryAudioProcessing";
const AVAudioSessionInterruptionNotification: &str = "AVAudioSessionInterruptionNotification";
const AVAudioSessionInterruptionOptionKey: &str = "AVAudioSessionInterruptionOptionKey";
const AVAudioSessionInterruptionTypeKey: &str = "AVAudioSessionInterruptionTypeKey";
const AVAudioSessionMediaServicesWereResetNotification: &str =
    "AVAudioSessionMediaServicesWereResetNotification";
const AVAudioSessionModeDefault: &str = "AVAudioSessionModeDefault";
const AVAudioSessionPortAirPlay: &str = "AVAudioSessionPortAirPlay";
const AVAudioSessionPortBluetoothA2DP: &str = "AVAudioSessionPortBluetoothA2DP";
const AVAudioSessionPortBluetoothLE: &str = "AVAudioSessionPortBluetoothLE";
const AVAudioSessionPortBuiltInMic: &str = "AVAudioSessionPortBuiltInMic";
const AVAudioSessionPortBuiltInReceiver: &str = "AVAudioSessionPortBuiltInReceiver";
const AVAudioSessionPortBuiltInSpeaker: &str = "AVAudioSessionPortBuiltInSpeaker";
const AVAudioSessionPortHDMI: &str = "AVAudioSessionPortHDMI";
const AVAudioSessionPortHeadphones: &str = "AVAudioSessionPortHeadphones";
const AVAudioSessionPortLineOut: &str = "AVAudioSessionPortLineOut";
const AVAudioSessionRouteChangeNotification: &str = "AVAudioSessionRouteChangeNotification";
const AVAudioSessionRouteChangePreviousRouteKey: &str = "AVAudioSessionRouteChangePreviousRouteKey";
const AVAudioSessionRouteChangeReasonKey: &str = "AVAudioSessionRouteChangeReasonKey";

/// `AVAudioSession` string constants
pub const CONSTANTS: ConstantExports = &[
    (
        "_AVAudioSessionCategoryAmbient",
        HostConstant::NSString(AVAudioSessionCategoryAmbient),
    ),
    (
        "_AVAudioSessionCategoryMultiRoute",
        HostConstant::NSString(AVAudioSessionCategoryMultiRoute),
    ),
    (
        "_AVAudioSessionCategoryPlayAndRecord",
        HostConstant::NSString(AVAudioSessionCategoryPlayAndRecord),
    ),
    (
        "_AVAudioSessionCategoryPlayback",
        HostConstant::NSString(AVAudioSessionCategoryPlayback),
    ),
    (
        "_AVAudioSessionCategoryRecord",
        HostConstant::NSString(AVAudioSessionCategoryRecord),
    ),
    (
        "_AVAudioSessionCategorySoloAmbient",
        HostConstant::NSString(AVAudioSessionCategorySoloAmbient),
    ),
    (
        "_AVAudioSessionCategoryAudioProcessing",
        HostConstant::NSString(AVAudioSessionCategoryAudioProcessing),
    ),
    (
        "_AVAudioSessionInterruptionNotification",
        HostConstant::NSString(AVAudioSessionInterruptionNotification),
    ),
    (
        "_AVAudioSessionInterruptionOptionKey",
        HostConstant::NSString(AVAudioSessionInterruptionOptionKey),
    ),
    (
        "_AVAudioSessionInterruptionTypeKey",
        HostConstant::NSString(AVAudioSessionInterruptionTypeKey),
    ),
    (
        "_AVAudioSessionMediaServicesWereResetNotification",
        HostConstant::NSString(AVAudioSessionMediaServicesWereResetNotification),
    ),
    (
        "_AVAudioSessionModeDefault",
        HostConstant::NSString(AVAudioSessionModeDefault),
    ),
    (
        "_AVAudioSessionPortAirPlay",
        HostConstant::NSString(AVAudioSessionPortAirPlay),
    ),
    (
        "_AVAudioSessionPortBluetoothA2DP",
        HostConstant::NSString(AVAudioSessionPortBluetoothA2DP),
    ),
    (
        "_AVAudioSessionPortBluetoothLE",
        HostConstant::NSString(AVAudioSessionPortBluetoothLE),
    ),
    (
        "_AVAudioSessionPortBuiltInMic",
        HostConstant::NSString(AVAudioSessionPortBuiltInMic),
    ),
    (
        "_AVAudioSessionPortBuiltInReceiver",
        HostConstant::NSString(AVAudioSessionPortBuiltInReceiver),
    ),
    (
        "_AVAudioSessionPortBuiltInSpeaker",
        HostConstant::NSString(AVAudioSessionPortBuiltInSpeaker),
    ),
    (
        "_AVAudioSessionPortHDMI",
        HostConstant::NSString(AVAudioSessionPortHDMI),
    ),
    (
        "_AVAudioSessionPortHeadphones",
        HostConstant::NSString(AVAudioSessionPortHeadphones),
    ),
    (
        "_AVAudioSessionPortLineOut",
        HostConstant::NSString(AVAudioSessionPortLineOut),
    ),
    (
        "_AVAudioSessionRouteChangeNotification",
        HostConstant::NSString(AVAudioSessionRouteChangeNotification),
    ),
    (
        "_AVAudioSessionRouteChangePreviousRouteKey",
        HostConstant::NSString(AVAudioSessionRouteChangePreviousRouteKey),
    ),
    (
        "_AVAudioSessionRouteChangeReasonKey",
        HostConstant::NSString(AVAudioSessionRouteChangeReasonKey),
    ),
];
