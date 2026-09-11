import autobahn from "autobahn";
import { WampSessionProvider, getWampResultString, safeJsonStringify } from "battler-wamp-client";
import {
  BatchQuery,
  BatchResult,
  DescriptionData,
  ResourceData,
  ResourceLookupOptions,
  ResourceOptions,
} from "./bindings/index.js";

export * from "./bindings/index.js";
export * from "battler-types";

import type { AbilityData, ConditionData, ItemData, MoveData, SpeciesData, TypeChartData } from "battler-types";

export interface ResourceWithDescription<T> {
  data: T;
  description?: DescriptionData | null;
}

function parseResourceOutput<T>(res: unknown): ResourceWithDescription<T> {
  let json: string | null = null;
  let description: DescriptionData | null = null;

  if (Array.isArray(res)) {
    json = getWampResultString(res[0]);
    if (res.length > 1 && res[1] && typeof res[1] === "object") {
      description = res[1] as DescriptionData;
    }
  } else if (res && typeof res === "object" && "args" in res && Array.isArray((res as any).args)) {
    const args = (res as any).args;
    json = getWampResultString(args[0]);
    if (args.length > 1 && args[1] && typeof args[1] === "object") {
      description = args[1] as DescriptionData;
    }
  } else {
    json = getWampResultString(res);
  }

  if (!json) throw new Error("Failed to parse resource response string");
  return {
    data: JSON.parse(json),
    description,
  };
}

export class BattlerDataServiceClient {
  constructor(private provider: WampSessionProvider) {}

  private get session(): autobahn.Session {
    const s = this.provider.session;
    if (!s) throw new Error("WAMP session is not connected");
    return s;
  }

  async getMove(query: string, options?: ResourceOptions): Promise<ResourceWithDescription<MoveData>> {
    const res = await this.session.call<unknown>("com.battler.data_service.move", [
      query,
      options ?? {},
    ]);
    return parseResourceOutput<MoveData>(res);
  }

  async getAbility(query: string, options?: ResourceOptions): Promise<ResourceWithDescription<AbilityData>> {
    const res = await this.session.call<unknown>("com.battler.data_service.ability", [
      query,
      options ?? {},
    ]);
    return parseResourceOutput<AbilityData>(res);
  }

  async getItem(query: string, options?: ResourceOptions): Promise<ResourceWithDescription<ItemData>> {
    const res = await this.session.call<unknown>("com.battler.data_service.item", [
      query,
      options ?? {},
    ]);
    return parseResourceOutput<ItemData>(res);
  }

  async getCondition(query: string, options?: ResourceOptions): Promise<ResourceWithDescription<ConditionData>> {
    const res = await this.session.call<unknown>("com.battler.data_service.condition", [
      query,
      options ?? {},
    ]);
    return parseResourceOutput<ConditionData>(res);
  }

  async getSpecies(query: string, options?: ResourceOptions): Promise<ResourceWithDescription<SpeciesData>> {
    const res = await this.session.call<unknown>("com.battler.data_service.species", [
      query,
      options ?? {},
    ]);
    return parseResourceOutput<SpeciesData>(res);
  }

  async getResource(
    query: string,
    options?: Partial<ResourceLookupOptions>,
  ): Promise<ResourceWithDescription<ResourceData>> {
    const res = await this.session.call<unknown>("com.battler.data_service.resource", [
      query,
      options ?? {},
    ]);
    return parseResourceOutput<ResourceData>(res);
  }

  async batch(query: BatchQuery): Promise<BatchResult> {
    const res = await this.session.call<unknown>("com.battler.data_service.batch", [
      safeJsonStringify(query),
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error("Failed to get batch response string");
    return JSON.parse(json);
  }

  async getTypeChart(): Promise<TypeChartData> {
    const res = await this.session.call<unknown>("com.battler.data_service.type_chart", []);
    let json: string | null = null;
    if (Array.isArray(res)) {
      json = getWampResultString(res[0]);
    } else if (res && typeof res === "object" && "args" in res && Array.isArray((res as any).args)) {
      json = getWampResultString((res as any).args[0]);
    } else {
      json = getWampResultString(res);
    }
    if (!json) throw new Error("Failed to parse type chart response string");
    return JSON.parse(json);
  }
}

