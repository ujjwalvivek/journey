import init, { render_scene } from "./pkg/journey_core.js";

async function run() {
  await init();

  const canvas = document.getElementById('bg');
  const ctx = canvas.getContext('2d');

  // Get controls
  const topColorInput = document.getElementById('topColor');
  const botColorInput = document.getElementById('botColor');
  const seedInput = document.getElementById('seed');

  const fogToggle = document.getElementById('fogToggle');
  const fogSettings = document.getElementById('fogSettings');
  const fogDensityInput = document.getElementById('fogDensity');
  const fogOpacityInput = document.getElementById('fogOpacity');
  const fogColorInput = document.getElementById('fogColor');
  const fogAnimSpeedInput = document.getElementById('fogAnimSpeed');

  fogToggle.addEventListener('change', () => {
    fogSettings.style.display = fogToggle.checked ? 'block' : 'none';
  });
  fogSettings.style.display = fogToggle.checked ? 'block' : 'none';

  function getParams() {
    return {
      topHex: topColorInput?.value || "#87ceeb",
      botHex: botColorInput?.value || "#1e3c72",
      seed: parseInt(seedInput?.value) || 0,
      fogOn: fogToggle?.checked || false,
      fogHex: fogColorInput?.value || "#b4c8dc",
      fogDensity: parseFloat(fogDensityInput?.value) || 3.0,
      fogOpacity: parseFloat(fogOpacityInput?.value) || 0.5,
      fogAnimSpeed: parseFloat(fogAnimSpeedInput?.value) || 0.05,
    };
  }

  function resizeCanvas() {
    const maxWidth = 1000;
    const maxHeight = 1000;
    canvas.width = Math.min(window.innerWidth, maxWidth);
    canvas.height = Math.min(window.innerHeight, maxHeight);
  }

  function drawScene(time) {
    resizeCanvas();
    const width = canvas.width;
    const height = canvas.height;
    const params = getParams();

    const pixels = render_scene(
      width,
      height,
      time,
      1,
      params.topHex,
      params.botHex,
      params.fogOn,
      params.fogDensity,
      params.fogOpacity,
      params.seed,
      params.fogHex,
      params.fogAnimSpeed,
    );
    const imageData = new ImageData(new Uint8ClampedArray(pixels), width, height);
    ctx.putImageData(imageData, 0, 0);
  }

  let t = 0;
  let animating = false;

  function animate() {
    if (fogToggle.checked) {
      animating = true;
      drawScene(t);
      t += 0.01;
      requestAnimationFrame(animate);
    } else {
      animating = false;
      drawScene(t);
    }
  }

  [
    topColorInput, botColorInput, fogDensityInput, fogOpacityInput, seedInput, fogColorInput, fogAnimSpeedInput, fogToggle,
  ].forEach(input => {
    if (input) input.addEventListener('input', () => {
      if (fogToggle.checked) {
        if (!animating) animate();
      } else {
        drawScene(t);
      }
    });
  });

  window.addEventListener('resize', () => drawScene(t));

  if (fogToggle.checked) {
    animate();
  } else {
    drawScene(t);
  }
}

run();