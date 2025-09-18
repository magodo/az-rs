#!/usr/bin/env node


import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SOURCE_JS = path.join(__dirname, '../public/pkg/azure_rs.js');
const SOURCE_DTS = path.join(__dirname, '../public/pkg/azure_rs.d.ts');
const SOURCE_WASM = path.join(__dirname, '../public/pkg/azure_rs_bg.wasm');
const TARGET_DIR = path.join(__dirname, '../src/wasm');
const TARGET_JS = path.join(TARGET_DIR, 'azure_rs.js');
const TARGET_DTS = path.join(TARGET_DIR, 'azure_rs.d.ts');
const PUBLIC_PKG_DIR = path.join(__dirname, '../public/pkg');

function getViteBasePath() {
  try {
    const viteConfigPath = path.join(__dirname, '../vite.config.ts');
    if (!fs.existsSync(viteConfigPath)) {
      console.log('⚠️  vite.config.ts not found, using default base path');
      return '/';
    }
    
    const configContent = fs.readFileSync(viteConfigPath, 'utf8');
    
    // Simple regex to extract base path from vite config
    const baseMatch = configContent.match(/base:\s*['"`]([^'"`]+)['"`]/);
    if (baseMatch) {
      const basePath = baseMatch[1];
      console.log(`📁 Found Vite base path: ${basePath}`);
      return basePath;
    }
    
    console.log('📁 No base path found in vite.config.ts, using default');
    return '/';
  } catch (error) {
    console.log(`⚠️  Error reading vite.config.ts: ${error.message}, using default base path`);
    return '/';
  }
}

function ensureDirectoryExists(dirPath) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`✅ Created directory: ${dirPath}`);
  }
}

function copyAndModifyJsFile() {
  try {
    // Read the original file
    const content = fs.readFileSync(SOURCE_JS, 'utf8');
    
    // Get the Vite base path
    const basePath = getViteBasePath();
    
    // Construct the WASM path with base path
    const wasmFileName = 'azure_bg.wasm';
    const wasmPath = path.posix.join(basePath, 'pkg', wasmFileName);

    console.log(`📝 Using WASM path: ${wasmPath}`);
    
    // Multiple patterns to handle different wasm-pack outputs
    let modifiedContent = content;
    let replacementsMade = 0;
    
    // Pattern 1: new URL('azure_bg.wasm', import.meta.url)
    const pattern1 = /module_or_path = new URL\('azure_bg\.wasm', import\.meta\.url\);/g;
    if (pattern1.test(modifiedContent)) {
      modifiedContent = modifiedContent.replace(pattern1, `module_or_path = '${wasmPath}';`);
      replacementsMade++;
    }
    
    // Pattern 2: new URL("azure_bg.wasm", import.meta.url)
    const pattern2 = /module_or_path = new URL\("azure_bg\.wasm", import\.meta\.url\);/g;
    if (pattern2.test(modifiedContent)) {
      modifiedContent = modifiedContent.replace(pattern2, `module_or_path = '${wasmPath}';`);
      replacementsMade++;
    }
    
    // Pattern 3: Any other similar pattern with azure_bg.wasm
    const pattern3 = /new URL\(['"]azure_bg\.wasm['"], import\.meta\.url\)/g;
    if (pattern3.test(modifiedContent)) {
      modifiedContent = modifiedContent.replace(pattern3, `'${wasmPath}'`);
      replacementsMade++;
    }
    
    // Write the modified file
    fs.writeFileSync(TARGET_JS, modifiedContent, 'utf8');
    console.log(`✅ Copied and modified: ${SOURCE_JS} -> ${TARGET_JS}`);
    
    // Check if any replacements were made
    if (replacementsMade === 0) {
      console.log(`⚠️  Warning: No WASM path replacements were made.`);
      console.log(`   The file might have a different pattern. Please check manually.`);
      
      // Show lines that might contain the WASM reference
      const lines = content.split('\n');
      const wasmLines = lines.filter(line => 
        line.includes('azure_bg.wasm') || 
        line.includes('import.meta.url')
      );
      
      if (wasmLines.length > 0) {
        console.log(`   Found potential WASM references:`);
        wasmLines.forEach(line => console.log(`   -> ${line.trim()}`));
      }
    } else {
      console.log(`✅ Fixed ${replacementsMade} WASM path(s) to point to ${wasmPath}`);
    }
    
  } catch (error) {
    console.error(`❌ Error processing JS file: ${error.message}`);
    process.exit(1);
  }
}

function copyTypeDefinitions() {
  try {
    if (fs.existsSync(SOURCE_DTS)) {
      fs.copyFileSync(SOURCE_DTS, TARGET_DTS);
      console.log(`✅ Copied type definitions: ${SOURCE_DTS} -> ${TARGET_DTS}`);
    } else {
      console.log(`⚠️  Type definitions not found: ${SOURCE_DTS}`);
    }
  } catch (error) {
    console.error(`❌ Error copying type definitions: ${error.message}`);
  }
}

function main() {
  console.log('🚀 Preparing WASM module for Vite...\n');
  
  // Check if source files exist
  if (!fs.existsSync(SOURCE_JS)) {
    console.log(`⚠️  WASM files not found in: ${SOURCE_JS}`);
    console.log('This is normal for first-time setup.');
    console.log('Run your WASM compilation first, then this script will prepare the files.');
    console.log('\nSkipping WASM preparation...');
    return; // Exit gracefully instead of with error
  }
  
  // Ensure target directory exists
  ensureDirectoryExists(TARGET_DIR);
  
  // Ensure public/pkg directory exists
  ensureDirectoryExists(PUBLIC_PKG_DIR);
  
  // Copy and modify files
  copyAndModifyJsFile();
  copyTypeDefinitions();
  
  console.log('\n✨ WASM module is ready for Vite!');
  console.log('You can now import it with: import("../wasm/azure_rs.js")');
}

// Run main function
main();