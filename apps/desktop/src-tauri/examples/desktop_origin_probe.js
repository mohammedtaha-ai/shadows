(() => {
  const collector = __COLLECTOR__, core = __CORE__, label = __LABEL__;
  const violations = [];
  document.addEventListener('securitypolicyviolation', e => violations.push(e.violatedDirective));
  window.addEventListener('DOMContentLoaded', async () => {
    const result = {label, locationOrigin: location.origin, violations};
    try {
      await fetch(collector + '/origin/' + label, {method:'POST'});
      if (label === 'main') {
        let config;
        for (let n = 0; n < 60; n++) {
          const r = await fetch(collector + '/ready');
          if (r.ok) { config = await r.json(); break; }
          await new Promise(resolve => setTimeout(resolve, 500));
        }
        if (!config) throw new Error('Core readiness timeout');
        result.get = (await fetch(core + '/api/projects')).status;
        result.jsonMutation = (await fetch(core + '/api/projects', {
          method:'POST', headers:{'Content-Type':'application/json'},
          body:JSON.stringify({command_id:'00000000-0000-4000-8000-000000000101',
            slug:'origin-probe',name:'Origin probe',directory:config.directory})
        })).status;
        result.sseCaughtUp = await new Promise((resolve, reject) => {
          const source = new EventSource(core + '/api/subscribe?thread_id=00000000-0000-4000-8000-000000000102&after=0');
          const timer = setTimeout(() => {source.close(); reject(new Error('SSE timeout'));},10000);
          source.addEventListener('caught-up', () => {clearTimeout(timer);source.close();resolve(true);});
        });
        await document.fonts.ready;
        result.fonts = document.fonts.status;
        result.rootWidth = document.getElementById('root')?.getBoundingClientRect().width;
      }
      try {
        await window.__TAURI_INTERNALS__.invoke('hide_main');
        result.ipcAllowed = true;
      } catch { result.ipcAllowed = false; }
    } catch (e) { result.error = e.message; }
    await fetch(collector + '/result/' + label, {method:'POST',
      headers:{'Content-Type':'text/plain'},body:JSON.stringify(result)});
  });
})();
