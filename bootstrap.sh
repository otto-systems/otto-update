#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

npm run build >/dev/null

if [[ "${OTTO_DRY_RUN:-0}" == "1" ]]; then
  node --input-type=module <<'EOF'
import { bootstrap } from "./dist/main.js";

const result = await bootstrap();
console.log("OTTO_DRY_RUN=1 bootstrap completed");
console.log(JSON.stringify(result, null, 2));
EOF
else
  node --input-type=module <<'EOF'
import { bootstrap } from "./dist/main.js";

const result = await bootstrap();
console.log(JSON.stringify(result, null, 2));
EOF
fi