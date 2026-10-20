import {defineConfig} from 'vite';
export default defineConfig(()=>({
  base:'./',
  build:{
    outDir:'companion-dist',
    rollupOptions:{input:{companion:'companion.html'}},
  },
}));
