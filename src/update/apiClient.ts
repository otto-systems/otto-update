export type OttoFetch = (
  input: string | URL,
  init?: RequestInit
) => Promise<Pick<Response, "ok" | "status" | "json">>;

export type OttoUpdateClientOptions = {
  baseUrl: string;
  token?: string;
  fetchImpl?: OttoFetch;
};

export type HealthResponse = {
  status: "ok" | "degraded" | "error";
  version: string;
  uptime_seconds: number;
};

export type StateResponse = {
  device_state: Record<string, unknown>;
  update_state: Record<string, unknown>;
  active_manifest: Record<string, unknown> | null;
};

export type PolicyDecision = {
  decision: "approve" | "defer" | "block" | "require_approval";
  reason?: string | null;
  until?: string | null;
  group?: string | null;
};

export type UpdateConfig = Record<string, unknown>;

export type BackupRecord = {
  id: string;
  version: string;
  created_at: string;
  path: string;
  file_count: number;
  sha256: string;
};

export class OttoUpdateApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly payload?: unknown
  ) {
    super(message);
    this.name = "OttoUpdateApiError";
  }
}

export class OttoUpdateApiClient {
  private readonly baseUrl: string;
  private readonly token?: string;
  private readonly fetchImpl: OttoFetch;

  constructor(options: OttoUpdateClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
    this.token = options.token;
    this.fetchImpl = options.fetchImpl ?? ((globalThis.fetch as OttoFetch | undefined) as OttoFetch);

    if (!this.fetchImpl) {
      throw new Error("fetch implementation not available");
    }
  }

  health(): Promise<HealthResponse> {
    return this.request<HealthResponse>("GET", "/health");
  }

  state(): Promise<StateResponse> {
    return this.request<StateResponse>("GET", "/v1/state");
  }

  triggerCheck(): Promise<{ check_id: string; triggered_at: string }> {
    return this.request("POST", "/v1/check");
  }

  manifest(): Promise<Record<string, unknown>> {
    return this.request("GET", "/v1/manifest");
  }

  policy(): Promise<PolicyDecision> {
    return this.request("GET", "/v1/policy");
  }

  approve(checkId: string): Promise<{ status: "approved" }> {
    return this.request("POST", "/v1/approve", { check_id: checkId });
  }

  defer(checkId: string, deferSeconds: number): Promise<{ until: string }> {
    return this.request("POST", "/v1/defer", {
      check_id: checkId,
      defer_seconds: deferSeconds
    });
  }

  progress(): Promise<Record<string, unknown> | null> {
    return this.request("GET", "/v1/progress", undefined, [200, 204]);
  }

  history(limit = 50, offset = 0): Promise<Record<string, unknown>> {
    return this.request(
      "GET",
      `/v1/history?limit=${encodeURIComponent(String(limit))}&offset=${encodeURIComponent(String(offset))}`
    );
  }

  config(): Promise<UpdateConfig> {
    return this.request("GET", "/v1/config");
  }

  setConfig(patch: Record<string, unknown>): Promise<UpdateConfig> {
    return this.request("PUT", "/v1/config", patch);
  }

  rollback(): Promise<{ rollback_id: string; triggered_at: string }> {
    return this.request("POST", "/v1/rollback", undefined, [202]);
  }

  backups(): Promise<{ items: BackupRecord[] }> {
    return this.request("GET", "/v1/backups");
  }

  private async request<T>(
    method: "GET" | "POST" | "PUT",
    path: string,
    body?: unknown,
    acceptedStatuses: number[] = [200]
  ): Promise<T> {
    const headers: Record<string, string> = {
      Accept: "application/json"
    };

    if (body !== undefined) {
      headers["Content-Type"] = "application/json";
    }

    if (this.token) {
      headers.Authorization = `Bearer ${this.token}`;
    }

    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined
    });

    let payload: unknown;
    try {
      payload = await response.json();
    } catch {
      payload = undefined;
    }

    if (!acceptedStatuses.includes(response.status)) {
      throw new OttoUpdateApiError(`request failed: ${method} ${path}`, response.status, payload);
    }

    if (response.status === 204) {
      return null as T;
    }

    return payload as T;
  }
}
