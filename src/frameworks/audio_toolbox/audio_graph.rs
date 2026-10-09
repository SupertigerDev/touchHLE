/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Audio graph management used by legacy AudioToolbox clients.

use std::collections::HashMap;

use crate::dyld::{export_c_func, FunctionExports};
use crate::environment::Environment;
use crate::frameworks::audio_toolbox::audio_components::{
    self, AURenderCallbackStruct, AudioComponent, AudioComponentDescription,
    AudioComponentInstance, OpaqueAudioComponent,
};
use crate::frameworks::carbon_core::{paramErr, OSStatus};
use crate::mem::{ConstPtr, MutPtr, SafeRead};

pub type AUNode = u32;

#[repr(C, packed)]
pub struct OpaqueAUGraph {
    _pad: u8,
}
unsafe impl SafeRead for OpaqueAUGraph {}

pub type AUGraph = MutPtr<OpaqueAUGraph>;

#[derive(Default)]
pub struct State {
    graphs: HashMap<AUGraph, AudioGraph>,
}

#[derive(Default)]
struct AudioGraph {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    is_open: bool,
    is_initialized: bool,
    is_running: bool,
}

struct Node {
    id: AUNode,
    description: AudioComponentDescription,
    component: AudioComponent,
    audio_unit: Option<AudioComponentInstance>,
    input_callback: Option<AURenderCallbackStruct>,
}

#[derive(Copy, Clone)]
struct Connection {
    destination_node: AUNode,
    destination_input: u32,
}

fn NewAUGraph(env: &mut Environment, out_graph: MutPtr<AUGraph>) -> OSStatus {
    if out_graph.is_null() {
        return paramErr;
    }
    let graph_ptr = env.mem.alloc_and_write(OpaqueAUGraph { _pad: 0 });
    env.framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .insert(graph_ptr, AudioGraph::default());
    env.mem.write(out_graph, graph_ptr);
    log_dbg!("NewAUGraph() -> {:?}", graph_ptr);
    0
}

fn AUGraphAddNode(
    env: &mut Environment,
    graph: AUGraph,
    description: ConstPtr<AudioComponentDescription>,
    out_node: MutPtr<AUNode>,
) -> OSStatus {
    if graph.is_null() || description.is_null() || out_node.is_null() {
        return paramErr;
    }
    let description = env.mem.read(description);
    let Some(graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
    else {
        return paramErr;
    };
    if graph.is_open {
        return paramErr;
    }
    let id = graph.nodes.iter().map(|node| node.id).max().unwrap_or(0) + 1;
    graph.nodes.push(Node {
        id,
        description,
        component: MutPtr::null(),
        audio_unit: None,
        input_callback: None,
    });
    env.mem.write(out_node, id);
    log_dbg!("AUGraphAddNode() -> {}", id);
    0
}

fn AUGraphOpen(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let Some(audio_graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
    else {
        return paramErr;
    };
    if audio_graph.is_open {
        return 0;
    }
    let node_ids: Vec<_> = audio_graph.nodes.iter().map(|node| node.id).collect();
    let instances: Vec<_> = node_ids
        .iter()
        .map(|_| {
            let component = env.mem.alloc_and_write(OpaqueAudioComponent { _pad: 0 });
            (
                component,
                audio_components::create_audio_component_instance(env),
            )
        })
        .collect();
    let audio_graph = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
        .unwrap();
    for (node, (component, audio_unit)) in audio_graph.nodes.iter_mut().zip(instances) {
        node.component = component;
        node.audio_unit = Some(audio_unit);
    }
    audio_graph.is_open = true;
    log_dbg!("AUGraphOpen({:?}) -> 0", graph);
    0
}

fn AUGraphNodeInfo(
    env: &mut Environment,
    graph: AUGraph,
    node_id: AUNode,
    out_component: MutPtr<AudioComponent>,
    out_audio_unit: MutPtr<AudioComponentInstance>,
) -> OSStatus {
    let Some(node) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
        .and_then(|graph| graph.nodes.iter().find(|node| node.id == node_id))
    else {
        return paramErr;
    };
    if !out_component.is_null() {
        env.mem.write(out_component, node.component);
    }
    if !out_audio_unit.is_null() {
        env.mem
            .write(out_audio_unit, node.audio_unit.unwrap_or_default());
    }
    log_dbg!("AUGraphNodeInfo({:?}, {})", graph, node_id);
    0
}

fn AUGraphConnectNodeInput(
    env: &mut Environment,
    graph: AUGraph,
    source_node: AUNode,
    _source_output: u32,
    destination_node: AUNode,
    destination_input: u32,
) -> OSStatus {
    let Some(audio_graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
    else {
        return paramErr;
    };
    if !audio_graph.nodes.iter().any(|node| node.id == source_node)
        || !audio_graph
            .nodes
            .iter()
            .any(|node| node.id == destination_node)
    {
        return paramErr;
    }
    let connection = Connection {
        destination_node,
        destination_input,
    };
    audio_graph.connections.retain(|existing| {
        existing.destination_node != destination_node
            || existing.destination_input != destination_input
    });
    audio_graph.connections.push(connection);
    0
}

fn AUGraphDisconnectNodeInput(
    env: &mut Environment,
    graph: AUGraph,
    destination_node: AUNode,
    destination_input: u32,
) -> OSStatus {
    let Some(audio_graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
    else {
        return paramErr;
    };
    audio_graph.connections.retain(|connection| {
        connection.destination_node != destination_node
            || connection.destination_input != destination_input
    });
    0
}

fn AUGraphSetNodeInputCallback(
    env: &mut Environment,
    graph: AUGraph,
    node_id: AUNode,
    _input_number: u32,
    callback: ConstPtr<AURenderCallbackStruct>,
) -> OSStatus {
    if callback.is_null() {
        return paramErr;
    }
    let callback = env.mem.read(callback);
    let Some(audio_unit) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
        .and_then(|graph| graph.nodes.iter().find(|node| node.id == node_id))
        .and_then(|node| node.audio_unit)
    else {
        return paramErr;
    };
    let Some(host_unit) = env
        .framework_state
        .audio_toolbox
        .audio_components
        .audio_component_instances
        .get_mut(&audio_unit)
    else {
        return paramErr;
    };
    host_unit.render_callback = Some(callback);
    if let Some(node) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .audio_graph_node_mut(graph, node_id)
    {
        node.input_callback = Some(callback);
    }
    log!(
        "AUGraphSetNodeInputCallback({:?}, node {}): callback {:?}, refcon {:?}",
        graph,
        node_id,
        callback.input_proc,
        callback.input_proc_ref_con
    );
    0
}

fn AUGraphInitialize(env: &mut Environment, graph: AUGraph) -> OSStatus {
    if AUGraphOpen(env, graph) != 0 {
        return paramErr;
    }
    let units: Vec<_> = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
        .unwrap()
        .nodes
        .iter()
        .filter_map(|node| node.audio_unit)
        .collect();
    for audio_unit in units {
        let status = super::audio_unit::AudioUnitInitialize(env, audio_unit);
        if status != 0 {
            return status;
        }
    }
    env.framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
        .unwrap()
        .is_initialized = true;
    0
}

fn AUGraphUninitialize(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let Some(audio_graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
    else {
        return paramErr;
    };
    let units: Vec<_> = audio_graph
        .nodes
        .iter()
        .filter_map(|node| node.audio_unit)
        .collect();
    for audio_unit in units {
        let _ = super::audio_unit::AudioUnitUninitialize(env, audio_unit);
    }
    env.framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
        .unwrap()
        .is_initialized = false;
    0
}

fn AUGraphStart(env: &mut Environment, graph: AUGraph) -> OSStatus {
    if AUGraphInitialize(env, graph) != 0 {
        return paramErr;
    }
    let output_units: Vec<_> = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
        .unwrap()
        .nodes
        .iter()
        .filter(|node| {
            node.description.component_type == audio_components::kAudioUnitType_Output
                && node.description.component_sub_type
                    == audio_components::kAudioUnitSubType_RemoteIO
        })
        .filter_map(|node| node.audio_unit)
        .collect();
    for audio_unit in output_units {
        let status = super::audio_unit::AudioOutputUnitStart(env, audio_unit);
        if status != 0 {
            return status;
        }
    }
    env.framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
        .unwrap()
        .is_running = true;
    0
}

fn AUGraphStop(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let Some(audio_graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
    else {
        return paramErr;
    };
    let output_units: Vec<_> = audio_graph
        .nodes
        .iter()
        .filter(|node| {
            node.description.component_type == audio_components::kAudioUnitType_Output
                && node.description.component_sub_type
                    == audio_components::kAudioUnitSubType_RemoteIO
        })
        .filter_map(|node| node.audio_unit)
        .collect();
    for audio_unit in output_units {
        let _ = super::audio_unit::AudioOutputUnitStop(env, audio_unit);
    }
    env.framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
        .unwrap()
        .is_running = false;
    0
}

fn AUGraphIsInitialized(
    env: &mut Environment,
    graph: AUGraph,
    out_initialized: MutPtr<u8>,
) -> OSStatus {
    let Some(audio_graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
    else {
        return paramErr;
    };
    if out_initialized.is_null() {
        return paramErr;
    }
    env.mem
        .write(out_initialized, audio_graph.is_initialized as u8);
    0
}

fn AUGraphIsRunning(env: &mut Environment, graph: AUGraph, out_running: MutPtr<u8>) -> OSStatus {
    let Some(audio_graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
    else {
        return paramErr;
    };
    if out_running.is_null() {
        return paramErr;
    }
    env.mem.write(out_running, audio_graph.is_running as u8);
    0
}

fn AUGraphUpdate(env: &mut Environment, graph: AUGraph, out_updated: MutPtr<u8>) -> OSStatus {
    if !env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .contains_key(&graph)
        || out_updated.is_null()
    {
        return paramErr;
    }
    env.mem.write(out_updated, 0);
    0
}

fn AUGraphClose(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let Some(audio_graph) = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get(&graph)
    else {
        return paramErr;
    };
    let units: Vec<_> = audio_graph
        .nodes
        .iter()
        .filter_map(|node| node.audio_unit)
        .collect();
    let components: Vec<_> = audio_graph
        .nodes
        .iter()
        .map(|node| node.component)
        .filter(|component| !component.is_null())
        .collect();
    for audio_unit in units {
        let _ = super::audio_unit::AudioUnitUninitialize(env, audio_unit);
        audio_components::AudioComponentInstanceDispose(env, audio_unit);
    }
    for component in components {
        env.mem.free(component.cast());
    }
    let audio_graph = env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .get_mut(&graph)
        .unwrap();
    for node in &mut audio_graph.nodes {
        node.audio_unit = None;
        node.component = MutPtr::null();
    }
    audio_graph.is_open = false;
    audio_graph.is_initialized = false;
    audio_graph.is_running = false;
    0
}

fn DisposeAUGraph(env: &mut Environment, graph: AUGraph) -> OSStatus {
    if !env
        .framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .contains_key(&graph)
    {
        return paramErr;
    }
    let _ = AUGraphStop(env, graph);
    let _ = AUGraphClose(env, graph);
    env.framework_state
        .audio_toolbox
        .audio_graph
        .graphs
        .remove(&graph);
    env.mem.free(graph.cast());
    0
}

impl State {
    fn audio_graph_node_mut(&mut self, graph: AUGraph, node_id: AUNode) -> Option<&mut Node> {
        self.graphs
            .get_mut(&graph)?
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(NewAUGraph(_)),
    export_c_func!(AUGraphAddNode(_, _, _)),
    export_c_func!(AUGraphOpen(_)),
    export_c_func!(AUGraphNodeInfo(_, _, _, _)),
    export_c_func!(AUGraphConnectNodeInput(_, _, _, _, _)),
    export_c_func!(AUGraphDisconnectNodeInput(_, _, _)),
    export_c_func!(AUGraphSetNodeInputCallback(_, _, _, _)),
    export_c_func!(AUGraphInitialize(_)),
    export_c_func!(AUGraphUninitialize(_)),
    export_c_func!(AUGraphStart(_)),
    export_c_func!(AUGraphStop(_)),
    export_c_func!(AUGraphIsInitialized(_, _)),
    export_c_func!(AUGraphIsRunning(_, _)),
    export_c_func!(AUGraphUpdate(_, _)),
    export_c_func!(AUGraphClose(_)),
    export_c_func!(DisposeAUGraph(_)),
];
