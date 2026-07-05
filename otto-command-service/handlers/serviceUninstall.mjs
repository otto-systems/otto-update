import { uninstall } from "./serviceShared.mjs";

export async function handle() {
  return uninstall();
}
