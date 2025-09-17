#!/usr/bin/env node

/**
 * Script to move azure.js from public/pkg to src/wasm and fix the WASM path
 * Also compresses WASM files with Brotli for better performance
 * Run this after compiling the WASM module to prepare it for Vite
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { compress } from 'brotli';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SOURCE_JS = path.join(__dirname, '../public/pkg/azure.js');
const SOURCE_DTS = path.join(__dirname, '../public/pkg/azure.d.ts');
const SOURCE_WASM = path.join(__dirname, '../public/pkg/azure_bg.wasm');
const TARGET_DIR = path.join(__dirname, '../src/wasm');
const TARGET_JS = path.join(TARGET_DIR, 'azure.js');
const TARGET_DTS = path.join(TARGET_DIR, 'azure.d.ts');
const PUBLIC_PKG_DIR = path.join(__dirname, '../public/pkg');

function ensureDirectoryExists(dirPath) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`✅ Created directory: ${dirPath}`);
  }
}

function compressWasmFile() {
  try {
    if (!fs.existsSync(SOURCE_WASM)) {
      console.log(`⚠️  WASM file not found: ${SOURCE_WASM}`);
      return false;
    }

    const wasmBuffer = fs.readFileSync(SOURCE_WASM);
    const originalSize = wasmBuffer.length;
    
    console.log(`📦 Compressing WASM file (${(originalSize / 1024 / 1024).toFixed(2)} MB)...`);
    
    // Compress with Brotli (quality 6 provides good balance of compression ratio and speed)
    const compressedBuffer = compress(wasmBuffer, {
      quality: 6,
      lgwin: 22
    });
    
    if (!compressedBuffer) {
      throw new Error('Brotli compression failed');
    }
    
    const compressedSize = compressedBuffer.length;
    const compressionRatio = ((originalSize - compressedSize) / originalSize * 100).toFixed(1);
    
    // Write compressed file to public/pkg directory
    const compressedPath = `${SOURCE_WASM}.br`;
    fs.writeFileSync(compressedPath, compressedBuffer);
    
    console.log(`✅ WASM compressed: ${(compressedSize / 1024 / 1024).toFixed(2)} MB (${compressionRatio}% reduction)`);
    console.log(`   Saved: ${compressedPath}`);
    
    return true;
  } catch (error) {
    console.error(`❌ Error compressing WASM: ${error.message}`);
    console.log('   Continuing with uncompressed WASM...');
    return false;
  }
}

function copyAndModifyJsFile() {
  try {
    // Read the original file
    const content = fs.readFileSync(SOURCE_JS, 'utf8');
    
    // Check if compressed WASM file exists
    const compressedWasmExists = fs.existsSync(`${SOURCE_WASM}.br`);
    const wasmPath = compressedWasmExists ? '/pkg/azure_bg.wasm.br' : '/pkg/azure_bg.wasm';
    
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
      if (compressedWasmExists) {
        console.log(`🗜️  Using compressed WASM file for better performance`);
      }
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
  
  // Compress WASM file
  const compressionSuccess = compressWasmFile();
  
  // Copy and modify files
  copyAndModifyJsFile();
  copyTypeDefinitions();
  
  console.log('\n✨ WASM module is ready for Vite!');
  console.log('You can now import it with: import("../wasm/azure.js")');
  
  if (compressionSuccess) {
    console.log('🗜️  Brotli compressed WASM available for production builds');
    console.log('   Configure your web server to serve .br files with proper Content-Encoding');
  }
}

// Run main function
main();