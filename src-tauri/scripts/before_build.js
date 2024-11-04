import fs from "fs"
import path from "path"
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const tauriConf = path.resolve(__dirname, "../tauri.conf.json")

const json = JSON.parse(fs.readFileSync(tauriConf, "utf-8"))
const [curr, inBundle] = Object.entries(json.bundle.resources).find(e => e[0].includes("obs.dll")) ?? []

if (curr) {
    json.bundle.resources[curr] = "./obs.dll"
    fs.writeFileSync(tauriConf, JSON.stringify(json, null, 2))
}