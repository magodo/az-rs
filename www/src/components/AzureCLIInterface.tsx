import React, { useState, useEffect } from 'react';
import { useAzureAuth } from '../hooks/useAzureAuth';
import { loadWasmModule, type WasmModule } from '../utils/wasmLoader';

export const AzureCLIInterface: React.FC = () => {
  const [command, setCommand] = useState('');
  const [output, setOutput] = useState('Azure CLI WASM interface ready...');
  const [loading, setLoading] = useState(false);
  const [wasmReady, setWasmReady] = useState(false);
  const [initializingWasm, setInitializingWasm] = useState(false);
  const [wasmModule, setWasmModule] = useState<WasmModule | null>(null);

  const { account, getAzureManagementToken } = useAzureAuth();

  // Initialize WASM module
  useEffect(() => {
    const initializeWasm = async () => {
      if (wasmReady || initializingWasm) return;

      setInitializingWasm(true);
      setOutput('Initializing Azure CLI WebAssembly module...');

      try {
        const module = await loadWasmModule();
        setWasmModule(module);
        setWasmReady(true);
        setOutput('Azure CLI WebAssembly module initialized successfully. Ready to execute commands.');
      } catch (error) {
        console.error('Failed to initialize WASM module:', error);
        setOutput(`Failed to initialize WASM module: ${error instanceof Error ? error.message : 'Unknown error'}`);
      } finally {
        setInitializingWasm(false);
      }
    };

    initializeWasm();
  }, [wasmReady, initializingWasm]);

  const parseCliCommand = (command: string): string[] => {
    // Remove leading/trailing whitespace and split by spaces
    // Handle quoted arguments properly
    const args: string[] = [];
    let current = '';
    let inQuotes = false;
    let quoteChar = '';

    for (let i = 0; i < command.length; i++) {
      const char = command[i];

      if ((char === '"' || char === "'") && !inQuotes) {
        inQuotes = true;
        quoteChar = char;
      } else if (char === quoteChar && inQuotes) {
        inQuotes = false;
        quoteChar = '';
      } else if (char === ' ' && !inQuotes) {
        if (current.trim()) {
          args.push(current.trim());
          current = '';
        }
      } else {
        current += char;
      }
    }

    if (current.trim()) {
      args.push(current.trim());
    }

    // Remove 'az' prefix if present and ensure 'azure' is the first element
    if (args.length > 0 && args[0] === 'azure') {
      args.shift();
    }

    // Always ensure 'azure' is the first element
    args.unshift('azure');

    return args;
  };

  const executeCommand = async () => {
    if (!wasmReady || !wasmModule || !account) {
      setOutput('WASM module not ready or user not authenticated');
      return;
    }

    if (!command.trim()) {
      setOutput('Please enter a command');
      return;
    }

    setLoading(true);
    setOutput('Executing Azure CLI command...');

    try {
      // Get access token for Azure Management API
      const accessToken = await getAzureManagementToken();
      if (!accessToken) {
        throw new Error('Failed to acquire access token');
      }

      const args = parseCliCommand(command);
      console.log('Executing command with args:', args);


      // Note: For MSAL authentication, we don't have client_id and secret in the traditional sense
      // The WASM module might need to be updated to handle access tokens instead
      // For now, we'll pass the access token as the secret parameter
      const result = await wasmModule.run_cli(
        args,

        accessToken // Using access token instead of client secret
      );

      // Try to format JSON if the result is valid JSON
      let formattedResult = result;
      try {
        const parsed = JSON.parse(result);
        formattedResult = JSON.stringify(parsed, null, 2);
      } catch {
        // Not valid JSON, use original result
        formattedResult = result;
      }

      setOutput(formattedResult);
    } catch (error) {
      let errorMessage = 'CLI execution failed';

      if (error instanceof Error) {
        errorMessage = error.message;
      } else if (typeof error === 'string') {
        try {
          // Try to parse JSON error from WASM
          const parsedError = JSON.parse(error);
          errorMessage = parsedError;
        } catch {
          errorMessage = error;
        }
      }

      setOutput(`${errorMessage}`);
    } finally {
      setLoading(false);
    }
  };

  const clearOutput = () => {
    setOutput('Output cleared. Ready for next command.');
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && e.ctrlKey) {
      executeCommand();
    }
  };

  return (
    <div className="cli-interface">
      <h2>Azure CLI WebAssembly Interface</h2>

      <div className="status-indicators">
        <div className={`status-item ${wasmReady ? 'ready' : 'not-ready'}`}>
          WASM Module: {wasmReady ? 'Ready' : initializingWasm ? 'Initializing...' : 'Not Ready'}
        </div>
        <div className={`status-item ${account ? 'ready' : 'not-ready'}`}>
          Authentication: {account ? 'Authenticated' : 'Not Authenticated'}
        </div>
      </div>

      <div className="command-input-section">
        <label htmlFor="cli-command">Azure CLI Command:</label>
        <div className="input-group">
          <span className="command-prefix">azure</span>
          <input
            type="text"
            id="cli-command"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            onKeyDown={handleKeyPress}
            placeholder="-h"
            disabled={!wasmReady || loading}
            className="cli-input"
          />
        </div>
        <div className="input-help">
          Press Ctrl+Enter to execute
        </div>
      </div>

      <div className="button-group">
        <button
          onClick={executeCommand}
          disabled={!wasmReady || loading || !account}
          className="execute-btn"
        >
          {loading ? 'Executing...' : 'Execute Command'}
        </button>
        <button
          onClick={clearOutput}
          className="clear-btn"
          disabled={loading}
        >
          Clear Output
        </button>
      </div>

      <div className="output-section">
        <h3>Output:</h3>
        <pre className={`cli-output ${loading ? 'loading' : ''}`}>
          {output}
        </pre>
      </div>
    </div>
  );
};
