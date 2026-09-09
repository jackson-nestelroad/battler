import autobahn from "autobahn";
import { WampSessionProvider, getWampResultString, safeJsonStringify } from "battler-wamp-client";
import {
  BatchQuery,
  BatchResult,
  ResourceData,
  ResourceLookupOptions,
  ResourceOptions,
} from "./bindings/index.js";

export * from "./bindings/index.js";
export * from "battler-types";

import type { AbilityData, ConditionData, ItemData, MoveData, SpeciesData } from "battler-types";

export class BattlerDataServiceClient {
  constructor(private provider: WampSessionProvider) {}

  private get session(): autobahn.Session {
    const s = this.provider.session;
    if (!s) throw new Error("WAMP session is not connected");
    return s;
  }

  async getMove(query: string, options?: ResourceOptions): Promise<MoveData> {
    const res = await this.session.call<unknown>("com.battler.data_service.move", [
      query,
      options ?? {},
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error(`Failed to get move response string for "${query}"`);
    return JSON.parse(json);
  }

  async getAbility(query: string, options?: ResourceOptions): Promise<AbilityData> {
    const res = await this.session.call<unknown>("com.battler.data_service.ability", [
      query,
      options ?? {},
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error(`Failed to get ability response string for "${query}"`);
    return JSON.parse(json);
  }

  async getItem(query: string, options?: ResourceOptions): Promise<ItemData> {
    const res = await this.session.call<unknown>("com.battler.data_service.item", [
      query,
      options ?? {},
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error(`Failed to get item response string for "${query}"`);
    return JSON.parse(json);
  }

  async getCondition(query: string, options?: ResourceOptions): Promise<ConditionData> {
    const res = await this.session.call<unknown>("com.battler.data_service.condition", [
      query,
      options ?? {},
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error(`Failed to get condition response string for "${query}"`);
    return JSON.parse(json);
  }

  async getSpecies(query: string, options?: ResourceOptions): Promise<SpeciesData> {
    const res = await this.session.call<unknown>("com.battler.data_service.species", [
      query,
      options ?? {},
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error(`Failed to get species response string for "${query}"`);
    return JSON.parse(json);
  }

  async getResource(
    query: string,
    options?: Partial<ResourceLookupOptions>,
  ): Promise<ResourceData> {
    const res = await this.session.call<unknown>("com.battler.data_service.resource", [
      query,
      options ?? {},
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error(`Failed to get resource response string for "${query}"`);
    return JSON.parse(json);
  }

  async batch(query: BatchQuery): Promise<BatchResult> {
    const res = await this.session.call<unknown>("com.battler.data_service.batch", [
      safeJsonStringify(query),
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error("Failed to get batch response string");
    return JSON.parse(json);
  }
}
