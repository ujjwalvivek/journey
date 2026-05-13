import resonanceWasmUrl from "../../../tools/audio/pkg/resonance_wasm_bg.wasm?url";

export type SharedAudio = {
    context: AudioContext;
    node: AudioWorkletNode;
};

let sharedAudio: SharedAudio | null = null;
let initPromise: Promise<SharedAudio> | null = null;

export async function ensureAudio(): Promise<SharedAudio> {
    if (sharedAudio) {
        return sharedAudio;
    }
    if (initPromise) {
        return initPromise;
    }

    initPromise = createAudio();
    sharedAudio = await initPromise;
    return sharedAudio;
}

async function createAudio(): Promise<SharedAudio> {
    const webkitWindow = window as Window & { webkitAudioContext?: typeof AudioContext };
    const AudioContextType = window.AudioContext ?? webkitWindow.webkitAudioContext;
    if (!AudioContextType) {
        throw new Error("Web Audio is not available");
    }

    const context = new AudioContextType();
    await context.audioWorklet.addModule("/resonance_worklet.js?v=3");

    const node = new AudioWorkletNode(context, "resonance-processor", {
        numberOfInputs: 0,
        numberOfOutputs: 1,
        outputChannelCount: [2],
    });

    node.connect(context.destination);

    const response = await fetch(resonanceWasmUrl);
    if (!response.ok) {
        throw new Error(`Failed to load WASM: ${response.status}`);
    }
    const wasmBytes = await response.arrayBuffer();

    const ready = new Promise<void>((resolve, reject) => {
        node.port.onmessage = (event: MessageEvent) => {
            const msg = event.data as { type?: string; message?: string };
            if (msg.type === "ready") {
                node.port.onmessage = null;
                resolve();
            } else if (msg.type === "error") {
                node.port.onmessage = null;
                reject(new Error(msg.message || "Worklet error"));
            }
        };
    });

    node.port.postMessage(
        { type: "init", wasmBytes, sampleRate: context.sampleRate },
        [wasmBytes],
    );

    await ready;

    const audio: SharedAudio = { context, node };

    window.dispatchEvent(new CustomEvent("audio:ready", { detail: audio }));

    return audio;
}

export function getSharedAudio(): SharedAudio | null {
    return sharedAudio;
}
