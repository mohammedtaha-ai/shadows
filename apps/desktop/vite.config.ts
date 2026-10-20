import {defineConfig} from 'vite';
export default defineConfig(({mode})=>({
  base:'./',
  build:{
    outDir:mode==='companion'?'companion-dist':'dist',
    rollupOptions:{input:mode==='companion'?{companion:'companion.html'}:{main:'index.html',companion:'companion.html'}},
  },
}));
