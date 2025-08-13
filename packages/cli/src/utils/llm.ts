import fs from 'node:fs';

export interface LlmOptions {
	provider?: 'openai' | 'anthropic';
	model?: string;
	maxTokens?: number;
}

export async function generateText(prompt: string, options: LlmOptions = {}): Promise<string> {
	const provider = options.provider || inferProvider();
	if (provider === 'openai') {
		const apiKey = getEnvVar('OPENAI_API_KEY');
		if (!apiKey) throw new Error('OPENAI_API_KEY is not set');
		const model = options.model || getEnvVar('OPENAI_MODEL') || 'gpt-4o-mini';
		const maxTokens = options.maxTokens || 1500;
		const res = await fetch('https://api.openai.com/v1/chat/completions', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Authorization: `Bearer ${apiKey}`
			},
			body: JSON.stringify({
				model,
				messages: [
					{ role: 'system', content: 'You are an expert code reviewer and technical evaluator. Be concise and structured.' },
					{ role: 'user', content: prompt }
				],
				max_tokens: maxTokens,
				temperature: 0.2
			})
		});
		if (!res.ok) {
			const text = await safeText(res);
			throw new Error(`OpenAI API error: ${res.status} ${text}`);
		}
		const json: any = await res.json();
		return json.choices?.[0]?.message?.content?.trim() || '';
	}

	if (provider === 'anthropic') {
		const apiKey = getEnvVar('ANTHROPIC_API_KEY');
		if (!apiKey) throw new Error('ANTHROPIC_API_KEY is not set');
		const model = options.model || getEnvVar('ANTHROPIC_MODEL') || 'claude-3-5-sonnet-latest';
		const maxTokens = options.maxTokens || 1500;
		const res = await fetch('https://api.anthropic.com/v1/messages', {
			method: 'POST',
			headers: {
				'content-type': 'application/json',
				'x-api-key': apiKey,
				'anthropic-version': '2023-06-01'
			},
			body: JSON.stringify({
				model,
				max_tokens: maxTokens,
				messages: [
					{ role: 'user', content: prompt }
				]
			})
		});
		if (!res.ok) {
			const text = await safeText(res);
			throw new Error(`Anthropic API error: ${res.status} ${text}`);
		}
		const json: any = await res.json();
		const content = json.content?.[0]?.text || '';
		return String(content).trim();
	}

	throw new Error('No supported LLM provider configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY.');
}

function inferProvider(): 'openai' | 'anthropic' {
	const prov = getEnvVar('LLM_PROVIDER');
	if (prov === 'anthropic') return 'anthropic';
	if (prov === 'openai') return 'openai';
	if (getEnvVar('OPENAI_API_KEY')) return 'openai';
	if (getEnvVar('ANTHROPIC_API_KEY')) return 'anthropic';
	return 'openai';
}

async function safeText(res: Response): Promise<string> {
	try {
		return await res.text();
	} catch {
		return '';
	}
}

function getEnvVar(name: string): string | undefined {
	const raw = (process.env as any)[name] as unknown as string | undefined;
	if (raw == null) return undefined;
	const v = String(raw).trim();
	if (v === '' || v.toLowerCase() === 'undefined' || v.toLowerCase() === 'null') return undefined;
	return v;
}


