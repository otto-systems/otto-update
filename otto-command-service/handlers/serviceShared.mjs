import { spawnSync } from "node:child_process";

const SERVICE_NAME = "OttoUpdate";

function unsupported(action) {
  return {
    ok: false,
    message: `service.${action} is only supported on Windows`
  };
}

function invokeSc(args, action) {
  if (process.platform !== "win32") {
    return unsupported(action);
  }

  const result = spawnSync("sc", args, { stdio: "pipe", encoding: "utf8" });
  if (result.status === 0) {
    return {
      ok: true,
      message: (result.stdout || `${action} succeeded`).trim()
    };
  }

  const detail = (result.stderr || result.stdout || "unknown sc failure").trim();
  return {
    ok: false,
    message: `service.${action} failed: ${detail}`
  };
}

export function install() {
  return invokeSc([
    "create",
    SERVICE_NAME,
    "binPath=",
    "\"C:\\Program Files\\OttoUpdate\\ottoupdate-server.exe\""
  ], "install");
}

export function start() {
  return invokeSc(["start", SERVICE_NAME], "start");
}

export function stop() {
  return invokeSc(["stop", SERVICE_NAME], "stop");
}

export function status() {
  return invokeSc(["query", SERVICE_NAME], "status");
}

export function uninstall() {
  return invokeSc(["delete", SERVICE_NAME], "uninstall");
}
