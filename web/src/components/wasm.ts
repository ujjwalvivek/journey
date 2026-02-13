// 1. Import the WASM module. 
import init, { render_scene } from "../../pkg/engine";

// 2. Define the types for your input parameters
interface SceneParams {
  topHex: string;
  botHex: string;
  seed: number;
  fogOn: boolean;
  fogHex: string;
  fogDensity: number;
  fogOpacity: number;
  fogAnimSpeed: number;
}

async function run() {
  // Initialize the WASM memory
  await init();

  const canvas = document.getElementById('bg') as HTMLCanvasElement | null;
  if (!canvas) throw new Error("Canvas element 'bg' not found");

  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error("Could not get 2D context");

  // 3. Get controls with explicit HTML types
  const topColorInput = document.getElementById('topColor') as HTMLInputElement | null;
  const botColorInput = document.getElementById('botColor') as HTMLInputElement | null;
  const seedInput = document.getElementById('seed') as HTMLInputElement | null;

  const fogToggle = document.getElementById('fogToggle') as HTMLInputElement | null;
  const fogSettings = document.getElementById('fogSettings') as HTMLDivElement | null;

  const fogDensityInput = document.getElementById('fogDensity') as HTMLInputElement | null;
  const fogOpacityInput = document.getElementById('fogOpacity') as HTMLInputElement | null;
  const fogColorInput = document.getElementById('fogColor') as HTMLInputElement | null;
  const fogAnimSpeedInput = document.getElementById('fogAnimSpeed') as HTMLInputElement | null;

  // Toggle visibility logic
  fogToggle?.addEventListener('change', () => {
    if (fogSettings) {
      fogSettings.style.display = fogToggle.checked ? 'block' : 'none';
    }
  });

  // Set initial state
  if (fogSettings && fogToggle) {
    fogSettings.style.display = fogToggle.checked ? 'block' : 'none';
  }

  function getParams(): SceneParams {
    return {
      topHex: topColorInput?.value || "#87ceeb",
      botHex: botColorInput?.value || "#1e3c72",
      seed: parseInt(seedInput?.value || "0"),
      fogOn: fogToggle?.checked || false,
      fogHex: fogColorInput?.value || "#b4c8dc",
      fogDensity: parseFloat(fogDensityInput?.value || "3.0"),
      fogOpacity: parseFloat(fogOpacityInput?.value || "0.5"),
      fogAnimSpeed: parseFloat(fogAnimSpeedInput?.value || "0.05"),
    };
  }

  function resizeCanvas() {
    const maxWidth = 1000;
    const maxHeight = 1000;
    canvas!.width = Math.min(window.innerWidth, maxWidth);
    canvas!.height = Math.min(window.innerHeight, maxHeight);
  }

  function drawScene(time: number) {
    if (!canvas || !ctx) return;

    resizeCanvas();
    const width = canvas.width;
    const height = canvas.height;
    const params = getParams();

    // Call the WASM function
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

    // Create ImageData from the Uint8ClampedArray returned by Rust
    const imageData = new ImageData(new Uint8ClampedArray(pixels), width, height);
    ctx.putImageData(imageData, 0, 0);
  }

  let t = 0;
  let animating = false;
  let animationFrameId: number;

  function animate() {
    if (fogToggle?.checked) {
      animating = true;
      drawScene(t);
      t += 0.01;
      animationFrameId = requestAnimationFrame(animate);
    } else {
      animating = false;
      drawScene(t);
      cancelAnimationFrame(animationFrameId);
    }
  }

  // Event Listeners
  const inputs = [
    topColorInput, botColorInput, fogDensityInput, fogOpacityInput,
    seedInput, fogColorInput, fogAnimSpeedInput, fogToggle
  ];

  inputs.forEach(input => {
    input?.addEventListener('input', () => {
      if (fogToggle?.checked) {
        if (!animating) animate();
      } else {
        drawScene(t);
      }
    });
  });

  window.addEventListener('resize', () => drawScene(t));

  // Kickoff
  if (fogToggle?.checked) {
    animate();
  } else {
    drawScene(t);
  }
}

run().catch(console.error);