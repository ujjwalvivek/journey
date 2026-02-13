import './style.css'
import '../components/wasm.ts'

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
  <div>
    <h1>Journey Engine</h1>
    <div class="controls">
        <fieldset>
            <legend>Sky Settings</legend>
            <label>Top Color: <input type="color" id="topColor" value="#87ceeb"></label>
            <label>Bottom Color: <input type="color" id="botColor" value="#1e3c72"></label>
            <label>Seed: <input type="number" id="seed" min="0" max="99999" value="0"></label>
        </fieldset>
        <fieldset>
            <legend>
                <label><input type="checkbox" id="fogToggle" checked> Fog</label>
            </legend>
            <div id="fogSettings">
                <label>Fog Color: <input type="color" id="fogColor" value="#b4c8dc"></label>
                <label>Fog Density: <input type="range" id="fogDensity" min="0.5" max="10" step="0.1" value="3"></label>
                <label>Fog Opacity: <input type="range" id="fogOpacity" min="0" max="1" step="0.01" value="0.5"></label>
                <label>Fog Animation Speed: <input type="range" id="fogAnimSpeed" min="0.01" max="0.5" step="0.01"
                        value="0.05"></label>
            </div>
        </fieldset>
    </div>
    <canvas id="bg" width="100%" height="100%"></canvas>
  </div>
`;