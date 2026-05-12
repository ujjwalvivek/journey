class ResonanceProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.exports = null;
    this.ready = false;
    this.reportCountdown = 0;
    this.params = {
      frequency: 440,
      waveform: 0,
      attackMs: 10,
      decayMs: 50,
      sustain: 0.7,
      releaseMs: 200,
      gain: 0.5,
    };

    this.port.onmessage = (event) => {
      this.handleMessage(event.data).catch((error) => {
        this.port.postMessage({
          type: "error",
          message: error && error.message ? error.message : String(error),
        });
      });
    };
  }

  async handleMessage(message) {
    if (message.type === "init") {
      const imports = {
        "./resonance_wasm_bg.js": {
          __wbindgen_init_externref_table: () => {
            const table = this.exports.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
          },
        },
      };
      const module = await WebAssembly.instantiate(message.wasmBytes, imports);
      this.exports = module.instance.exports;
      this.exports.__wbindgen_start();
      this.exports.resonance_init(Math.round(message.sampleRate || sampleRate));
      this.applyParams(this.params);
      this.ready = true;
      this.port.postMessage({ type: "ready" });
      return;
    }

    if (message.type === "set") {
      this.params = { ...this.params, ...message.params };
      if (this.ready) {
        this.applyParams(this.params);
      }
      return;
    }

    if (!this.ready) {
      return;
    }

    if (message.type === "note-on") {
      this.exports.resonance_note_on();
    } else if (message.type === "note-off") {
      this.exports.resonance_note_off();
    } else if (message.type === "trigger-patch") {
      this.exports.resonance_trigger_patch(message.patch);
    }
  }

  applyParams(params) {
    this.exports.resonance_set_frequency(params.frequency);
    this.exports.resonance_set_waveform(params.waveform);
    this.exports.resonance_set_adsr(params.attackMs, params.decayMs, params.sustain, params.releaseMs);
    this.exports.resonance_set_master_gain(params.gain);
  }

  process(_inputs, outputs) {
    const output = outputs[0];
    if (!output || output.length === 0) {
      return true;
    }

    const frames = output[0].length;
    let peak = 0;

    for (let i = 0; i < frames; i += 1) {
      const sample = this.ready ? this.exports.resonance_next_sample() : 0;
      const abs = Math.abs(sample);
      if (abs > peak) {
        peak = abs;
      }

      for (let channel = 0; channel < output.length; channel += 1) {
        output[channel][i] = sample;
      }
    }

    this.reportCountdown -= 1;
    if (this.ready && this.reportCountdown <= 0) {
      this.reportCountdown = 12;
      this.port.postMessage({ type: "meter", peak, stage: this.exports.resonance_stage() });
    }

    return true;
  }
}

registerProcessor("resonance-processor", ResonanceProcessor);
