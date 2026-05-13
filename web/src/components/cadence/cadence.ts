import "./cadence.css";
import { ensureAudio, type SharedAudio } from "../shared-audio";

const SCENES = ["Drums", "Melodic", "Full"] as const;

type CadenceState = {
    audio: SharedAudio | null;
    active: boolean;
    bpm: number;
    scene: number;
    step: number;
};

type CadenceControls = {
    panel: HTMLElement;
    power: HTMLButtonElement;
    close: HTMLButtonElement;
    bpmDisplay: HTMLElement;
    sceneButtons: HTMLButtonElement[];
    stepCells: HTMLElement[];
    status: HTMLElement;
};

export function setupCadencePanel() {
    if (document.getElementById("cadence-panel")) {
        return;
    }

    const panel = document.createElement("section");
    panel.id = "cadence-panel";
    panel.hidden = true;
    panel.setAttribute("aria-label", "Cadence sequencer");
    panel.innerHTML = panelMarkup();
    document.body.append(panel);

    const controls = createControls(panel);
    const state = createState();

    wirePanelEvents(controls);
    controls.close.addEventListener("click", () => closePanel(controls));

    window.addEventListener("journey:open-cadence", () => openPanel(controls));
    window.addEventListener("keydown", (event) => {
        if (event.key === "Escape" && !controls.panel.hidden) {
            closePanel(controls);
        }
    });

    controls.power.addEventListener("click", async () => {
        try {
            if (!state.audio) {
                setStatus(controls, "Loading audio engine...");
                state.audio = await ensureAudio();
                await state.audio.context.resume();

                state.audio.node.port.start();
                state.audio.node.port.addEventListener("message", (event: MessageEvent) => {
                    const msg = event.data as { type?: string; cadenceStep?: number; cadenceBpm?: number; cadenceActive?: boolean };
                    if (msg.type === "meter" && msg.cadenceActive) {
                        updateStepGrid(controls, msg.cadenceStep ?? 0);
                    }
                });

                setStatus(controls, "Audio engine ready");
            }

            await state.audio.context.resume();
            state.active = !state.active;
            controls.power.classList.toggle("running", state.active);
            controls.power.textContent = state.active ? "Stop" : "Start";
            state.audio.node.port.postMessage({ type: "cadence-active", active: state.active });
            setStatus(controls, state.active ? "Sequencer running" : "Sequencer stopped");
        } catch (error) {
            setStatus(controls, error instanceof Error ? error.message : String(error));
        }
    });

    for (const btn of panel.querySelectorAll<HTMLButtonElement>("[data-bpm-delta]")) {
        btn.addEventListener("click", () => {
            const delta = Number(btn.dataset.bpmDelta);
            state.bpm = Math.max(40, Math.min(300, state.bpm + delta));
            controls.bpmDisplay.textContent = `${state.bpm} BPM`;
            state.audio?.node.port.postMessage({ type: "cadence-bpm", bpm: state.bpm });
        });
    }

    // Scene buttons
    for (const btn of controls.sceneButtons) {
        btn.addEventListener("click", () => {
            state.scene = Number(btn.dataset.scene);
            for (const b of controls.sceneButtons) {
                b.classList.toggle("active", b === btn);
            }
            state.audio?.node.port.postMessage({ type: "cadence-scene", scene: state.scene });
        });
    }
}

function panelMarkup(): string {
    const sceneBtns = SCENES.map((name, i) =>
        `<button type="button" data-scene="${i}" class="${i === 0 ? "active" : ""}">${name}</button>`
    ).join("");

    const steps = Array.from({ length: 16 }, (_, i) =>
        `<div class="cadence-step" data-step="${i}">${i + 1}</div>`
    ).join("");

    return `
        <div class="cadence-topbar">
            <div>
                <p class="cadence-eyebrow">cadence</p>
                <p id="cadence-status" class="cadence-status">Sequencer idle</p>
            </div>
            <button id="cadence-power" class="cadence-power" type="button">Start</button>
            <button id="cadence-close" class="cadence-close" type="button" aria-label="Close Cadence panel">X</button>
        </div>
        <div class="cadence-controls">
            <div class="cadence-section">
                <div class="cadence-section-label">Tempo</div>
                <div class="cadence-bpm-display" id="cadence-bpm">120 BPM</div>
                <div class="cadence-bpm-controls">
                    <button type="button" data-bpm-delta="-10">-10</button>
                    <button type="button" data-bpm-delta="-1">-1</button>
                    <button type="button" data-bpm-delta="1">+1</button>
                    <button type="button" data-bpm-delta="10">+10</button>
                </div>
            </div>
            <div class="cadence-section">
                <div class="cadence-section-label">Scene</div>
                <div class="cadence-scenes">${sceneBtns}</div>
            </div>
        </div>
        <div class="cadence-grid-wrap">
            <div class="cadence-grid-label">Step Sequencer</div>
            <div class="cadence-step-grid">${steps}</div>
        </div>
    `;
}

function createControls(panel: HTMLElement): CadenceControls {
    return {
        panel,
        power: requireElement("#cadence-power", panel),
        close: requireElement("#cadence-close", panel),
        bpmDisplay: requireElement("#cadence-bpm", panel),
        sceneButtons: Array.from(panel.querySelectorAll<HTMLButtonElement>("[data-scene]")),
        stepCells: Array.from(panel.querySelectorAll<HTMLElement>("[data-step]")),
        status: requireElement("#cadence-status", panel),
    };
}

function createState(): CadenceState {
    return {
        audio: null,
        active: false,
        bpm: 120,
        scene: 0,
        step: 0,
    };
}

function wirePanelEvents(controls: CadenceControls) {
    for (const eventName of ["pointerdown", "pointerup", "click", "keydown", "keyup", "wheel"]) {
        controls.panel.addEventListener(eventName, (event) => event.stopPropagation());
    }
}

function openPanel(controls: CadenceControls) {
    controls.panel.hidden = false;
    controls.power.focus();
}

function closePanel(controls: CadenceControls) {
    controls.panel.hidden = true;
}

function setStatus(controls: CadenceControls, message: string) {
    controls.status.textContent = message;
}

function updateStepGrid(controls: CadenceControls, step: number) {
    for (let i = 0; i < controls.stepCells.length; i++) {
        controls.stepCells[i].classList.toggle("active", i === step);
    }
}

function requireElement<T extends Element>(selector: string, root: ParentNode = document): T {
    const element = root.querySelector<T>(selector);
    if (!element) {
        throw new Error(`Missing element: ${selector}`);
    }
    return element;
}
