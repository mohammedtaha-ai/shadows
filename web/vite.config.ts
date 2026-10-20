/// <reference types="vitest/config" />
import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig(({mode})=>({
  base:mode==="desktop"?"./":"/",
  plugins: [react(), tailwindcss(),{
    name:'desktop-module-inventory',
    generateBundle(){
      if(mode==='desktop')this.emitFile({type:'asset',fileName:'desktop-modules.json',source:JSON.stringify([...this.getModuleIds()].map(id=>id.replaceAll('\\','/')))})
    },
  }],
  // A test waits with `until` (10 s); under a busy CPU the default 5 s per test
  // cut tests short that would have passed.
  test: { testTimeout: 30_000 },
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
}))
