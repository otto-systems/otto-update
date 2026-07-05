import { status } from "./serviceShared.mjs";

export async function handle() {
  return status();
}
