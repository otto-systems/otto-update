import { describe, expect, it, vi } from "vitest";

import { OttoUpdateApiClient, OttoUpdateApiError } from "./apiClient.js";

describe("OttoUpdateApiClient", () => {
  it("sends auth header and parses successful JSON payload", async () => {
    const fetchImpl = vi.fn(async () => ({
      ok: true,
      status: 200,
      json: async () => ({ status: "ok", version: "1.0.0", uptime_seconds: 42 })
    }));

    const client = new OttoUpdateApiClient({
      baseUrl: "http://localhost:7430",
      token: "abc123",
      fetchImpl
    });

    const health = await client.health();
    expect(health.status).toBe("ok");
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(fetchImpl.mock.calls[0]?.[0]).toBe("http://localhost:7430/health");

    const init = fetchImpl.mock.calls[0]?.[1];
    expect(init?.method).toBe("GET");
    expect((init?.headers as Record<string, string>).Authorization).toBe("Bearer abc123");
  });

  it("sends POST body for approval endpoint", async () => {
    const fetchImpl = vi.fn(async () => ({
      ok: true,
      status: 200,
      json: async () => ({ status: "approved" })
    }));

    const client = new OttoUpdateApiClient({ baseUrl: "http://localhost:7430", fetchImpl });
    const result = await client.approve("check-1");

    expect(result.status).toBe("approved");
    const init = fetchImpl.mock.calls[0]?.[1];
    expect(init?.method).toBe("POST");
    expect(init?.body).toBe(JSON.stringify({ check_id: "check-1" }));
  });

  it("maps 204 progress response to null", async () => {
    const fetchImpl = vi.fn(async () => ({
      ok: true,
      status: 204,
      json: async () => {
        throw new Error("no content");
      }
    }));

    const client = new OttoUpdateApiClient({ baseUrl: "http://localhost:7430", fetchImpl });
    const progress = await client.progress();

    expect(progress).toBeNull();
  });

  it("throws OttoUpdateApiError on non-accepted status", async () => {
    const fetchImpl = vi.fn(async () => ({
      ok: false,
      status: 409,
      json: async () => ({ error: "already_checking" })
    }));

    const client = new OttoUpdateApiClient({ baseUrl: "http://localhost:7430", fetchImpl });

    await expect(client.triggerCheck()).rejects.toBeInstanceOf(OttoUpdateApiError);
  });
});
