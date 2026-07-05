import { generatedCliCommands, runGeneratedCliCommand } from "./generated_cli/index.js";
import { executeGeneratedCommand, generatedCommandSchemas } from "./generated_api/index.js";

export type BootstrapStageResult = {
  stage: string;
  command: string;
  ok: boolean;
  detail: unknown;
};

export type BootstrapResult = {
  stages: BootstrapStageResult[];
};

async function runStage(stage: string, command: string, params: Record<string, unknown> = {}): Promise<BootstrapStageResult> {
  const detail = await executeGeneratedCommand(command, params);
  const ok = typeof detail === "object" && detail !== null && "ok" in detail ? Boolean((detail as { ok: unknown }).ok) : true;
  return { stage, command, ok, detail };
}

export async function splashScreen(): Promise<BootstrapStageResult> {
  return runStage("splash-screen", "config.show");
}

export async function selfUpdate(): Promise<BootstrapStageResult> {
  return runStage("self-update", "service.status");
}

export async function payloadReader(): Promise<BootstrapStageResult> {
  return runStage("payload-reader", "config.show");
}

export async function installerOrchestration(): Promise<BootstrapStageResult> {
  return runStage("installer-orchestration", "service.install");
}

export async function telemetryHooks(): Promise<BootstrapStageResult> {
  return runStage("telemetry-hooks", "service.status");
}

export async function kernelBridge(): Promise<BootstrapStageResult> {
  return runStage("kernel-bridge", "service.start");
}

export async function extensionLoader(): Promise<BootstrapStageResult> {
  return runStage("extension-loader", "service.status");
}

export async function bootstrap(): Promise<BootstrapResult> {
  const stages = [
    await splashScreen(),
    await selfUpdate(),
    await payloadReader(),
    await installerOrchestration(),
    await telemetryHooks(),
    await kernelBridge(),
    await extensionLoader()
  ];

  return { stages };
}

export async function runCli(command: (typeof generatedCliCommands)[number], params: Record<string, unknown> = {}): Promise<unknown> {
  return runGeneratedCliCommand(command, params);
}

export { generatedCliCommands, generatedCommandSchemas };
