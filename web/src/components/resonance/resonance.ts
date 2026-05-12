import resonanceWasmUrl from "../../../../tools/audio/pkg/resonance_wasm_bg.wasm?url";
import "./resonance.css";

const WAVEFORMS = ["Sine", "Square", "Saw", "Triangle", "Noise"] as const;
const PATCHES = [
    { label: "Kick", index: 0 },
    { label: "Snare", index: 1 },
    { label: "HiHat", index: 2 },
    { label: "Laser", index: 3 },
    { label: "Coin", index: 4 },
    { label: "Explosion", index: 5 },
] as const;

type ResonanceState = {
    context: AudioContext | null;
    node: AudioWorkletNode | null;
    readyPromise: Promise<void> | null;
    resolveReady: (() => void) | null;
    rejectReady: ((error: Error) => void) | null;
    ready: boolean;
    running: boolean;
    waveform: number;
    peak: number;
    phase: number;
};

type ResonanceControls = {
    panel: HTMLElement;
    power: HTMLButtonElement;
    close: HTMLButtonElement;
    scope: HTMLCanvasElement;
    frequency: HTMLInputElement;
    frequencyValue: HTMLOutputElement;
    gain: HTMLInputElement;
    gainValue: HTMLOutputElement;
    attack: HTMLInputElement;
    attackValue: HTMLOutputElement;
    decay: HTMLInputElement;
    decayValue: HTMLOutputElement;
    sustain: HTMLInputElement;
    sustainValue: HTMLOutputElement;
    release: HTMLInputElement;
    releaseValue: HTMLOutputElement;
    waveforms: HTMLDivElement;
    patches: HTMLDivElement;
    status: HTMLParagraphElement;
};

export function setupResonancePanel() {
    if (document.getElementById("resonance-panel")) {
        return;
    }

    const panel = document.createElement("section");
    panel.id = "resonance-panel";
    panel.hidden = true;
    panel.setAttribute("aria-label", "Resonance synth");
    panel.innerHTML = panelMarkup();
    document.body.append(panel);

    const controls = createControls(panel);
    const state = createState();

    populateButtonGroups(controls);
    initializeSliders(controls);
    wirePanelEvents(controls);
    controls.close.addEventListener("click", () => closePanel(controls));

    window.addEventListener("journey:open-resonance", () => openPanel(controls));
    window.addEventListener("keydown", (event) => {
        if (event.key === "Escape" && !controls.panel.hidden) {
            closePanel(controls);
        }
    });

    controls.power.addEventListener("click", async () => {
        try {
            const { context, node } = await ensureAudio(controls, state);
            await context.resume();
            setRunning(controls, state, !state.running);
            node.port.postMessage({ type: state.running ? "note-on" : "note-off" });
            setStatus(controls, state.running ? "Held oscillator running" : "Held oscillator released");
        } catch (error) {
            setStatus(controls, errorMessage(error), true);
        }
    });

    for (const control of sliderInputs(controls)) {
        control.addEventListener("input", () => {
            updateLabels(controls);
            sendParams(controls, state);
        });
    }

    controls.waveforms.addEventListener("click", (event) => {
        const target = event.target as Element | null;
        const button = target?.closest<HTMLButtonElement>("button[data-waveform]");
        if (!button) {
            return;
        }

        state.waveform = Number(button.dataset.waveform);
        for (const item of controls.waveforms.querySelectorAll("button")) {
            item.classList.toggle("active", item === button);
        }
        sendParams(controls, state);
    });

    controls.patches.addEventListener("click", async (event) => {
        const target = event.target as Element | null;
        const button = target?.closest<HTMLButtonElement>("button[data-patch]");
        if (!button) {
            return;
        }

        try {
            const { context, node } = await ensureAudio(controls, state);
            await context.resume();
            node.port.postMessage({ type: "trigger-patch", patch: Number(button.dataset.patch) });
            setStatus(controls, `${button.textContent ?? "Patch"} triggered`);
        } catch (error) {
            setStatus(controls, errorMessage(error), true);
        }
    });

    updateLabels(controls);
    drawScope(controls, state);
}

function panelMarkup() {
    return `
        <div class="resonance-topbar">
            <div>
                <p class="resonance-eyebrow">resonance-wasm</p>
                <p id="resonance-status" class="resonance-status">Audio engine idle</p>
            </div>
            <button id="resonance-power" class="resonance-power" type="button" aria-pressed="false">Start</button>
            <button id="resonance-close" class="resonance-close" type="button" aria-label="Close Resonance panel">X</button>
        </div>
        <canvas id="resonance-scope" width="960" height="260" aria-label="Waveform monitor"></canvas>
        <div class="resonance-grid">
            <label class="resonance-control">
                <span>Frequency</span>
                <output id="resonance-frequency-value">440 Hz</output>
                <input id="resonance-frequency" type="range" min="20" max="20000" step="1" value="440">
            </label>
            <label class="resonance-control">
                <span>Gain</span>
                <output id="resonance-gain-value">50%</output>
                <input id="resonance-gain" type="range" min="0" max="1" step="0.01" value="0.5">
            </label>
            <div class="resonance-control">
                <span>Waveform</span>
                <div class="resonance-waveforms" id="resonance-waveforms" role="group" aria-label="Waveform"></div>
            </div>
            <div class="resonance-control">
                <span>Sounds</span>
                <div class="resonance-patch-grid" id="resonance-patches" role="group" aria-label="Procedural sounds"></div>
            </div>
        </div>
        <div class="resonance-adsr">
            <label class="resonance-control">
                <span>Attack</span>
                <output id="resonance-attack-value">10 ms</output>
                <input id="resonance-attack" type="range" min="0" max="2000" step="1" value="10">
            </label>
            <label class="resonance-control">
                <span>Decay</span>
                <output id="resonance-decay-value">50 ms</output>
                <input id="resonance-decay" type="range" min="0" max="2000" step="1" value="50">
            </label>
            <label class="resonance-control">
                <span>Sustain</span>
                <output id="resonance-sustain-value">0.70</output>
                <input id="resonance-sustain" type="range" min="0" max="1" step="0.01" value="0.7">
            </label>
            <label class="resonance-control">
                <span>Release</span>
                <output id="resonance-release-value">200 ms</output>
                <input id="resonance-release" type="range" min="0" max="3000" step="1" value="200">
            </label>
        </div>
    `;
}

function createControls(panel: HTMLElement): ResonanceControls {
    return {
        panel,
        power: requireElement("#resonance-power", panel),
        close: requireElement("#resonance-close", panel),
        scope: requireElement("#resonance-scope", panel),
        frequency: requireElement("#resonance-frequency", panel),
        frequencyValue: requireElement("#resonance-frequency-value", panel),
        gain: requireElement("#resonance-gain", panel),
        gainValue: requireElement("#resonance-gain-value", panel),
        attack: requireElement("#resonance-attack", panel),
        attackValue: requireElement("#resonance-attack-value", panel),
        decay: requireElement("#resonance-decay", panel),
        decayValue: requireElement("#resonance-decay-value", panel),
        sustain: requireElement("#resonance-sustain", panel),
        sustainValue: requireElement("#resonance-sustain-value", panel),
        release: requireElement("#resonance-release", panel),
        releaseValue: requireElement("#resonance-release-value", panel),
        waveforms: requireElement("#resonance-waveforms", panel),
        patches: requireElement("#resonance-patches", panel),
        status: requireElement("#resonance-status", panel),
    };
}

function createState(): ResonanceState {
    return {
        context: null,
        node: null,
        readyPromise: null,
        resolveReady: null,
        rejectReady: null,
        ready: false,
        running: false,
        waveform: 0,
        peak: 0,
        phase: 0,
    };
}

function populateButtonGroups(controls: ResonanceControls) {
    for (const label of WAVEFORMS) {
        const index = controls.waveforms.children.length;
        const button = document.createElement("button");
        button.type = "button";
        button.dataset.waveform = String(index);
        button.textContent = label;
        if (index === 0) {
            button.classList.add("active");
        }
        controls.waveforms.append(button);
    }

    for (const { label, index } of PATCHES) {
        const button = document.createElement("button");
        button.type = "button";
        button.dataset.patch = String(index);
        button.textContent = label;
        controls.patches.append(button);
    }
}

function initializeSliders(controls: ResonanceControls) {
    for (const input of sliderInputs(controls)) {
        addSliderChrome(input);
    }
}

function wirePanelEvents(controls: ResonanceControls) {
    for (const eventName of ["pointerdown", "pointerup", "click", "keydown", "keyup", "wheel"]) {
        controls.panel.addEventListener(eventName, (event) => event.stopPropagation());
    }
}

function openPanel(controls: ResonanceControls) {
    controls.panel.hidden = false;
    controls.power.focus();
}

function closePanel(controls: ResonanceControls) {
    controls.panel.hidden = true;
}

function setStatus(controls: ResonanceControls, message: string, isError = false) {
    controls.status.textContent = message;
    controls.status.classList.toggle("error", isError);
}

function params(controls: ResonanceControls, state: ResonanceState) {
    return {
        frequency: Number(controls.frequency.value),
        waveform: state.waveform,
        attackMs: Number(controls.attack.value),
        decayMs: Number(controls.decay.value),
        sustain: Number(controls.sustain.value),
        releaseMs: Number(controls.release.value),
        gain: Number(controls.gain.value),
    };
}

async function ensureAudio(controls: ResonanceControls, state: ResonanceState) {
    if (state.context && state.node) {
        if (state.readyPromise) {
            await state.readyPromise;
        }
        return { context: state.context, node: state.node };
    }

    const webkitWindow = window as Window & { webkitAudioContext?: typeof AudioContext };
    const AudioContextType = window.AudioContext ?? webkitWindow.webkitAudioContext;
    if (!AudioContextType) {
        throw new Error("Web Audio is not available");
    }

    const context = new AudioContextType();
    state.context = context;
    setStatus(controls, "Loading audio worklet");
    await context.audioWorklet.addModule("/resonance_worklet.js?v=2");

    const node = new AudioWorkletNode(context, "resonance-processor", {
        numberOfInputs: 0,
        numberOfOutputs: 1,
        outputChannelCount: [2],
    });
    state.node = node;

    state.readyPromise = new Promise((resolve, reject) => {
        state.resolveReady = resolve;
        state.rejectReady = reject;
    });

    node.onprocessorerror = () => {
        const error = new Error("Audio processor crashed");
        setStatus(controls, error.message, true);
        state.rejectReady?.(error);
    };

    node.port.onmessage = (event: MessageEvent) => {
        const message = event.data as { type?: string; peak?: number; message?: string };
        if (message.type === "ready") {
            state.ready = true;
            setStatus(controls, "Audio engine ready");
            sendParams(controls, state);
            if (state.running) {
                node.port.postMessage({ type: "note-on" });
            }
            state.resolveReady?.();
        } else if (message.type === "meter") {
            state.peak = Number(message.peak ?? 0);
        } else if (message.type === "error") {
            const error = new Error(message.message || "Audio worklet error");
            setStatus(controls, error.message, true);
            state.rejectReady?.(error);
        }
    };

    node.connect(context.destination);
    const response = await fetch(resonanceWasmUrl);
    if (!response.ok) {
        throw new Error(`Failed to load WASM: ${response.status}`);
    }
    const wasmBytes = await response.arrayBuffer();
    setStatus(controls, "Loading WASM synth");
    node.port.postMessage(
        {
            type: "init",
            wasmBytes,
            sampleRate: context.sampleRate,
        },
        [wasmBytes],
    );

    if (state.readyPromise) {
        await state.readyPromise;
    }
    return { context, node };
}

function sendParams(controls: ResonanceControls, state: ResonanceState) {
    state.node?.port.postMessage({ type: "set", params: params(controls, state) });
}

function updateLabels(controls: ResonanceControls) {
    controls.frequencyValue.textContent = `${Number(controls.frequency.value).toFixed(0)} Hz`;
    controls.gainValue.textContent = `${Math.round(Number(controls.gain.value) * 100)}%`;
    controls.attackValue.textContent = `${Number(controls.attack.value).toFixed(0)} ms`;
    controls.decayValue.textContent = `${Number(controls.decay.value).toFixed(0)} ms`;
    controls.sustainValue.textContent = Number(controls.sustain.value).toFixed(2);
    controls.releaseValue.textContent = `${Number(controls.release.value).toFixed(0)} ms`;
}

function setRunning(controls: ResonanceControls, state: ResonanceState, running: boolean) {
    state.running = running;
    controls.power.classList.toggle("running", running);
    controls.power.textContent = running ? "Stop" : "Start";
    controls.power.setAttribute("aria-pressed", String(running));
}

function addSliderChrome(input: HTMLInputElement) {
    const wrapper = document.createElement("div");
    wrapper.className = "resonance-slider";

    const track = document.createElement("div");
    track.className = "resonance-slider-track";

    const thumb = document.createElement("div");
    thumb.className = "resonance-slider-thumb";

    wrapper.append(track, thumb);
    input.after(wrapper);

    let dragging = false;

    const updateSliderPosition = () => {
        const min = Number(input.min) || 0;
        const max = Number(input.max) || 100;
        const value = Number(input.value) || 0;
        const fill = ((value - min) / (max - min)) * 100;
        const position = `${fill}%`;
        wrapper.style.setProperty("--position", position);
        wrapper.style.setProperty("--fill", position);
    };

    const handleInput = (clientX: number) => {
        const rect = wrapper.getBoundingClientRect();
        const x = clientX - rect.left;
        const fill = Math.max(0, Math.min(1, x / rect.width));
        const min = Number(input.min) || 0;
        const max = Number(input.max) || 100;
        input.value = String(min + fill * (max - min));
        input.dispatchEvent(new Event("input", { bubbles: true }));
        updateSliderPosition();
    };

    wrapper.addEventListener("pointerdown", (event) => {
        dragging = true;
        thumb.classList.add("dragging");
        wrapper.setPointerCapture(event.pointerId);
        handleInput(event.clientX);
    });

    wrapper.addEventListener("pointermove", (event) => {
        if (dragging) {
            handleInput(event.clientX);
        }
    });

    const stopDragging = (event: PointerEvent) => {
        if (!dragging) {
            return;
        }
        dragging = false;
        thumb.classList.remove("dragging");
        if (wrapper.hasPointerCapture(event.pointerId)) {
            wrapper.releasePointerCapture(event.pointerId);
        }
    };

    wrapper.addEventListener("pointerup", stopDragging);
    wrapper.addEventListener("pointercancel", stopDragging);
    input.addEventListener("input", updateSliderPosition);
    updateSliderPosition();
}

function waveformSample(t: number, waveform: number, phase: number) {
    if (waveform === 1) {
        return t < 0.5 ? 1 : -1;
    }
    if (waveform === 2) {
        return 2 * t - 1;
    }
    if (waveform === 3) {
        return 1 - 4 * Math.abs(Math.round(t - 0.25) - (t - 0.25));
    }
    if (waveform === 4) {
        return Math.sin((t * 97.13 + phase) * Math.PI * 2) * Math.sin(t * 311.7);
    }
    return Math.sin(t * Math.PI * 2);
}

function drawScope(controls: ResonanceControls, state: ResonanceState) {
    const canvas = controls.scope;
    const dpr = window.devicePixelRatio || 1;
    const width = Math.floor(canvas.clientWidth * dpr);
    const height = Math.floor(canvas.clientHeight * dpr);

    if (width <= 0 || height <= 0) {
        requestAnimationFrame(() => drawScope(controls, state));
        return;
    }

    if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) {
        requestAnimationFrame(() => drawScope(controls, state));
        return;
    }

    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = "#0b0d0f";
    ctx.fillRect(0, 0, width, height);

    ctx.strokeStyle = "#1f2a31";
    ctx.lineWidth = dpr;
    for (let i = 1; i < 8; i += 1) {
        const x = (width / 8) * i;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
    }
    for (let i = 1; i < 4; i += 1) {
        const y = (height / 4) * i;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
    }

    const amplitude = Math.max(0.08, state.running ? state.peak : 0.12);
    const cycles = Math.max(1, Math.min(8, Number(controls.frequency.value) / 220));

    ctx.strokeStyle = state.running ? "#44d9c5" : "#52616a";
    ctx.lineWidth = 3 * dpr;
    ctx.beginPath();
    for (let x = 0; x < width; x += 1) {
        const t = (x / width) * cycles + state.phase;
        const sample = waveformSample(t - Math.floor(t), state.waveform, state.phase) * amplitude;
        const y = height * 0.5 - sample * height * 0.42;
        if (x === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();

    state.phase = (state.phase + 0.004) % 1;
    state.peak *= 0.94;
    requestAnimationFrame(() => drawScope(controls, state));
}

function sliderInputs(controls: ResonanceControls) {
    return [
        controls.frequency,
        controls.gain,
        controls.attack,
        controls.decay,
        controls.sustain,
        controls.release,
    ];
}

function requireElement<T extends Element>(selector: string, root: ParentNode = document): T {
    const element = root.querySelector<T>(selector);
    if (!element) {
        throw new Error(`Missing element: ${selector}`);
    }
    return element;
}

function errorMessage(error: unknown) {
    return error instanceof Error ? error.message : String(error);
}
