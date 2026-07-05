import fs from "node:fs/promises";
import path from "node:path";
import { handle as showConfig } from "./configShow.mjs";

function toToml(config) {
  const tokenValue = config.bearer_token === null ? "null" : `\"${config.bearer_token}\"`;
  return `bind = \"${config.bind}\"\nbearer_token = ${tokenValue}\n`;
}

export async function handle(params = {}) {
  const targetPath = params.path ?? "./config/server.toml";
  const cfg = await showConfig({ path: targetPath });

  if (typeof params.bind === "string") {
    cfg.bind = params.bind;
  }
  if (Object.prototype.hasOwnProperty.call(params, "bearer_token")) {
    cfg.bearer_token = params.bearer_token;
  }

  await fs.mkdir(path.dirname(targetPath), { recursive: true });
  await fs.writeFile(targetPath, toToml(cfg), "utf8");

  return {
    updatedPath: targetPath,
    bind: cfg.bind,
    bearer_token: cfg.bearer_token
  };
}
