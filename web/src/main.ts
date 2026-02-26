async function run() {
    const urlParams = new URLSearchParams(window.location.search);
    const version = urlParams.get('v');

    let init;

    if (version) {
        console.log(`Loading Engine v${version}...`);
        const cdnUrl = `https://cdn.jsdelivr.net/npm/@ujjwalvivek/journey@${version}/game.js`;

        const module = await import(/* @vite-ignore */ cdnUrl); //* Initialize the WASM module and start the game
        init = module.default;
    } else {
        console.log("Loading Engine -local...");
        const module = await import('../../game/pkg/game.js');
        init = module.default;
    }

    await init();
    console.log("Journey booted successfully!");
}

run();