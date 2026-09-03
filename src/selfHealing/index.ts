/**
 * Self-Healing Framework for OttoUpdate
 * 
 * Public API for programs to register artifacts and enable self-healing
 */

export * from "./types.js";
export {
  SelfHealingRegistry,
  getGlobalSelfHealingRegistry,
  resetGlobalRegistry,
} from "./registry.js";
export {
  PreUpdateValidator,
  createPreUpdateValidator,
} from "./preUpdateValidator.js";
