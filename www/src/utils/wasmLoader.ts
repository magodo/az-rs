// WASM module types
export interface WasmModule {
  run_cli: (args: string[],  token: string) => Promise<string>;
}

// Simple cache to avoid re-initializing
let wasmModuleCache: WasmModule | null = null;
let initPromise: Promise<WasmModule> | null = null;

/**
 * Load and initialize the Azure CLI WASM module
 * Much simpler approach: JS in src, WASM in public
 */
export const loadWasmModule = async (): Promise<WasmModule> => {
  // Return cached module if already loaded
  if (wasmModuleCache) {
    return wasmModuleCache;
  }

  // Return existing promise if already loading
  if (initPromise) {
    return initPromise;
  }

  // Create initialization promise
  initPromise = (async () => {
    try {
      // Direct import now works since azure.js is in src/
      const wasmModule = await import('../wasm/azure.js');
      
      // Initialize the WASM module
      await wasmModule.default();
      
      // Cache and return the module
      wasmModuleCache = {
        run_cli: wasmModule.run_cli
      };
      
      return wasmModuleCache;
    } catch (error) {
      // Reset promise on error so we can retry
      initPromise = null;
      throw new Error(`WASM initialization failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  })();

  return initPromise;
};