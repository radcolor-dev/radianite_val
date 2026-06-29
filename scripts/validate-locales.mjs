import { access, readFile, readdir } from "node:fs/promises"
import { constants } from "node:fs"
import path from "node:path"
import process from "node:process"

const root = process.cwd()
const registry = JSON.parse(await readFile(path.join(root, "locale-registry.json"), "utf8"))
const fallback = registry.fallbackLocale

function flatten(value, prefix = "", out = new Map()) {
  for (const [key, entry] of Object.entries(value)) {
    if (key.startsWith("$") || key === "_version") continue
    const full = prefix ? `${prefix}.${key}` : key
    if (typeof entry === "string") out.set(full, entry)
    else if (entry && typeof entry === "object" && !Array.isArray(entry)) flatten(entry, full, out)
    else throw new Error(`Invalid translation value at ${full}`)
  }
  return out
}

function placeholders(value, pattern) {
  return [...value.matchAll(pattern)].map((match) => match[1]).sort().join(",")
}

async function exists(file) {
  try {
    await access(file, constants.F_OK)
    return true
  } catch {
    return false
  }
}

async function readJson(file) {
  return JSON.parse(await readFile(file, "utf8"))
}

const uiDir = path.join(root, "src-ui/locales/ui")
const rustDir = path.join(root, "src-rs/locales")
const uiFallbackPath = path.join(uiDir, `${fallback}.json`)
const rustFallbackPath = path.join(rustDir, `${fallback}.json`)

if (!(await exists(uiFallbackPath))) throw new Error(`Missing UI fallback catalog ${fallback}.json`)
if (!(await exists(rustFallbackPath))) throw new Error(`Missing Rust fallback catalog ${fallback}.json`)

const uiFallback = flatten(await readJson(uiFallbackPath))
const rustFallback = flatten(await readJson(rustFallbackPath))
const tags = new Set()
let uiCatalogs = 0
let rustCatalogs = 0

for (const locale of registry.locales) {
  if (tags.has(locale.tag)) throw new Error(`Duplicate locale ${locale.tag}`)
  tags.add(locale.tag)
  if (!/^[a-z]{2,3}(?:-[A-Z][a-z]{3})?(?:-[A-Z]{2}|-\d{3})?$/.test(locale.tag)) throw new Error(`Invalid BCP 47 tag ${locale.tag}`)
  if (!locale.nativeName || !locale.englishName || !["ltr", "rtl"].includes(locale.direction)) throw new Error(`Incomplete metadata for ${locale.tag}`)

  const uiFile = path.join(uiDir, `${locale.tag}.json`)
  if (locale.ui && (await exists(uiFile))) {
    uiCatalogs++
    const translated = flatten(await readJson(uiFile))
    for (const [key, value] of translated) {
      if (!uiFallback.has(key)) throw new Error(`${locale.tag} UI has unknown key ${key}`)
      if (value === "") continue
      const expected = placeholders(uiFallback.get(key), /{{\s*([\w.-]+)(?:\s*,[^}]*)?\s*}}/g)
      const actual = placeholders(value, /{{\s*([\w.-]+)(?:\s*,[^}]*)?\s*}}/g)
      if (expected !== actual) throw new Error(`${locale.tag} UI placeholder mismatch at ${key}: expected [${expected}], got [${actual}]`)
    }
  }

  const rustFile = path.join(rustDir, `${locale.tag}.json`)
  if ((locale.ui || locale.rpc) && (await exists(rustFile))) {
    rustCatalogs++
    const translated = flatten(await readJson(rustFile))
    for (const [key, value] of translated) {
      if (!rustFallback.has(key)) throw new Error(`${locale.tag} Rust catalog has unknown key ${key}`)
      if (value === "") continue
      const expected = placeholders(rustFallback.get(key), /%{([\w.-]+)}/g)
      const actual = placeholders(value, /%{([\w.-]+)}/g)
      if (expected !== actual) throw new Error(`${locale.tag} Rust placeholder mismatch at ${key}: expected [${expected}], got [${actual}]`)
    }
  }
}

for (const name of await readdir(uiDir)) {
  if (name.endsWith(".json") && !name.startsWith("_") && !tags.has(name.slice(0, -5))) throw new Error(`UI catalog ${name} is missing from locale-registry.json`)
}
for (const name of await readdir(rustDir)) {
  if (name.endsWith(".json") && !name.startsWith("_") && !tags.has(name.slice(0, -5))) throw new Error(`Rust catalog ${name} is missing from locale-registry.json`)
}

console.log(`Validated ${registry.locales.length} locale(s); checked ${uiCatalogs} UI and ${rustCatalogs} Rust catalog(s); partial catalogs use ${fallback} fallback.`)
