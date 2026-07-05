import fs from "node:fs/promises";

const DEFAULT_CONFIG = {
  bind: "127.0.0.1:7430",
  bearer_token: null
};

function parseToml(raw) {
  const out = { ...DEFAULT_CONFIG };
  for (const line of raw.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }
    const idx = trimmed.indexOf("=");
    if (idx === -1) {
      continue;
    }
    const key = trimmed.slice(0, idx).trim();
    const val = trimmed.slice(idx + 1).trim();
    if (key === "bind") {
      out.bind = val.replace(/^"|"$/g, "");
    }
    if (key === "bearer_token") {
      out.bearer_token = val === "null" ? null : val.replace(/^"|"$/g, "");
    }
  }
  return out;
}

export async function handle(params = {}) {
  const path = params.path ?? "./config/server.toml";
  try {
    const raw = await fs.readFile(path, "utf8");
    return parseToml(raw);
  } catch {
    return { ...DEFAULT_CONFIG };
  }
}
