ALTER TABLE aylul_provider ADD COLUMN preset TEXT;
ALTER TABLE aylul_provider ADD COLUMN label TEXT;
UPDATE aylul_provider SET preset = 'minimax' WHERE base_url = 'https://api.minimax.io/anthropic';
UPDATE aylul_provider SET preset = 'minimax-china' WHERE base_url = 'https://api.minimaxi.com/anthropic';
UPDATE aylul_provider SET preset = 'anthropic' WHERE base_url = 'https://api.anthropic.com';
UPDATE aylul_provider SET preset = 'deepseek' WHERE base_url = 'https://api.deepseek.com/anthropic';
UPDATE aylul_provider SET preset = 'openrouter' WHERE base_url = 'https://openrouter.ai/api';
UPDATE aylul_provider SET preset = 'moonshot' WHERE base_url = 'https://api.moonshot.ai/anthropic';
UPDATE aylul_provider SET preset = 'moonshot-china' WHERE base_url = 'https://api.moonshot.cn/anthropic';
