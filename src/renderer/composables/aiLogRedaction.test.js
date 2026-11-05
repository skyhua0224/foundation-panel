import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { callLLM } from './aiClient.js'

const LOG = [
  'Error: encoder failed Authorization: Bearer abcdefghijklmnop9876',
  'sk-abcdefghijklmnopqrstuvwxyz token=supersecretvalue api_key=anothersecretvalue user@example.com 192.168.0.10 fd00:: fe80:: AA:BB:CC:DD:EE:FF 123e4567-e89b-12d3-a456-426614174000',
  'open C:\\Users\\name\\app.log and /home/name/.config',
].join('\n')

const SECRETS = [
  'abcdefghijklmnop9876', 'sk-abcdefghijklmnopqrstuvwxyz', 'supersecretvalue', 'anothersecretvalue',
  'user@example.com', '192.168.0.10', 'fd00::', 'fe80::', 'AA:BB:CC:DD:EE:FF',
  '123e4567-e89b-12d3-a456-426614174000', 'C:\\Users\\name', '/home/name',
]

function filesUnder(dir, acc = []) {
  for (const name of fs.readdirSync(dir)) {
    if (name === 'node_modules' || name === 'dist') continue
    const full = path.join(dir, name)
    if (fs.statSync(full).isDirectory()) filesUnder(full, acc)
    else if (/\.(js|vue)$/.test(name) && !name.endsWith('.test.js')) acc.push(full)
  }
  return acc
}

test('the model request body is scrubbed, including when the raw log is passed again', async (t) => {
  const bodies = []
  t.mock.method(globalThis, 'fetch', async (url, options = {}) => {
    if (String(url).includes('/api/logs')) return new Response(LOG, { status: 200 })
    bodies.push(String(options.body || ''))
    return Response.json({ choices: [{ message: { content: 'ok' } }] })
  })
  const image = 'A'.repeat(80)
  await callLLM({ model: 'm', temperature: 0.3 }, [
    { role: 'system', content: LOG },
    { role: 'user', content: [
      { type: 'text', text: LOG },
      { type: 'image_url', image_url: { url: 'data:image/png;base64,' + image } },
    ] },
  ])
  assert.equal(bodies.length, 1)
  for (const secret of SECRETS) assert.equal(bodies[0].includes(secret), false, 'body ' + secret)
  assert.ok(bodies[0].includes(image), 'image bytes stay')
})

test('log text cannot reach callLLM without the scrubber', () => {
  const root = path.resolve(new URL('..', import.meta.url).pathname)
  const actions = fs.readFileSync(new URL('./aiActions.js', import.meta.url), 'utf8')
  const client = fs.readFileSync(new URL('./aiClient.js', import.meta.url), 'utf8')
  const assistant = fs.readFileSync(new URL('./useAiAssistant.js', import.meta.url), 'utf8')
  const logsFn = actions.slice(actions.indexOf('export async function getLogsContext'), actions.indexOf('export async function getAppsContext'))
  assert.match(logsFn, /scrubText/)
  for (const name of ['callLLM', 'callOpenAI', 'callAnthropic']) {
    const at = client.indexOf('export async function ' + name)
    const next = client.indexOf('export ', at + 10)
    assert.match(client.slice(at, next), /scrubOutboundMessages/, name)
  }
  const send = assistant.slice(assistant.indexOf('async function sendMessage'), assistant.indexOf('async function retryLastMessage'))
  const retry = assistant.slice(assistant.indexOf('async function retryLastMessage'), assistant.indexOf('async function applyAction'))
  assert.doesNotMatch(send, /\/api\/logs/)
  assert.doesNotMatch(retry, /\/api\/logs/)
  for (const file of filesUnder(root)) {
    const src = fs.readFileSync(file, 'utf8')
    if (src.includes('callLLM(') && src.includes('/api/logs')) {
      assert.fail(file + ' both fetches /api/logs and calls callLLM')
    }
  }
})
