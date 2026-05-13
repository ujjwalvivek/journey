import { setupResonancePanel } from "./components/resonance/resonance.ts";
import { setupCadencePanel } from "./components/cadence/cadence.ts";

type GameInit = () => Promise<unknown>;

async function run() {
    const bootTitle = document.getElementById("title");
    const bootFill = document.getElementById("progress-fill") as HTMLDivElement | null;
    const bootStatus = document.getElementById("progress-status");
    const bootHint = document.getElementById("hint");

    const setText = (title: string) => { if (bootTitle) bootTitle.textContent = title; };
    const setStatus = (status: string) => { if (bootStatus) bootStatus.textContent = status; };
    const setHint = (hint: string) => { if (bootHint) bootHint.textContent = hint; };

    const trickleProgress = (ceiling: number, durationS: number) => {
        if (!bootFill) return;
        void bootFill.offsetWidth;
        bootFill.style.transition = `transform ${durationS}s linear`;
        bootFill.style.transform = `scaleX(${ceiling.toFixed(4)})`;
    };

    document.body.classList.remove("ready");
    setText("Loading engine...");
    setStatus("downloading");
    setHint("");
    bootFill?.classList.remove("done");

    const firstFrameReady = new Promise<void>((resolve) => {
        window.addEventListener("journey:first-frame", () => resolve(), { once: true });
    });

    const urlParams = new URLSearchParams(window.location.search);
    const version = urlParams.get("v");

    try {
        let init: GameInit;

        if (version) {
            const cdnUrl = `https://cdn.jsdelivr.net/npm/@ujjwalvivek/journey-engine@${version}/game.js`;
            const module = await import(/* @vite-ignore */ cdnUrl);
            init = module.default as GameInit;
        } else {
            const module = await import("../../game/pkg/game.js");
            init = module.default as GameInit;
        }

        setText("Initializing Engine");
        setStatus("The first load may take a while!");
        setHint("Starting");

        await init();

        setText("Preparing world");
        setStatus("The first load may take a while!");
        setHint("Almost there");
        trickleProgress(0.95, 10);

        await firstFrameReady;
        setupResonancePanel();
        setupCadencePanel();

        if (bootFill) {
            bootFill.style.transition = "none";
            bootFill.style.transform = "scaleX(1)";
            void bootFill.offsetWidth;
            bootFill.classList.add("done");
        }
        setText("Ready");
        setStatus("Initialized");

        window.setTimeout(() => {
            document.body.classList.add("ready");
        }, 400);
    } catch (error) {
        if (bootFill) bootFill.style.transition = "none";
        setText("Startup failed");
        setStatus("error");
        setHint("check console");
        console.error("Journey boot failed:", error);
    }
}

run();
