import { generatedCliCommands, runGeneratedCliCommand } from "./generated_cli/index.js";
import { generatedCommandSchemas } from "./generated_api/index.js";

export type GeneratedEntryPoint = {
  commands: typeof generatedCliCommands;
  schemas: typeof generatedCommandSchemas;
};

export function getGeneratedEntryPoint(): GeneratedEntryPoint {
  return {
    commands: generatedCliCommands,
    schemas: generatedCommandSchemas
  };
}

export async function runCommandFromGeneratedSurface(
  commandName: (typeof generatedCliCommands)[number],
  params: Record<string, unknown> = {}
): Promise<unknown> {
  return runGeneratedCliCommand(commandName, params);
}
