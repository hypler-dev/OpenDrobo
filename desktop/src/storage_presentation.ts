/** Display transport only where the service's reviewed 5D profile establishes it. */
export function deviceLabel(brand: string, model: string, connectionType?: number): string {
  const cleanBrand = brand.trim();
  const cleanModel = model.trim();
  const label = cleanBrand && cleanModel.toLowerCase().startsWith(cleanBrand.toLowerCase()) ? cleanModel : [cleanBrand, cleanModel].filter(Boolean).join(" ") || "Drobo";
  return connectionType === 6 && (cleanModel === "Drobo 5D" || cleanModel === "5D") ? `${label} - TB` : label;
}

export interface VolumeIdentity {
  id: string;
  provider: string;
  deviceId: string | null;
  enclosureId?: string | null;
  deviceName?: string | null;
}

/** Group only by native ancestry; equal names never establish shared hardware. */
export function groupDroboVolumes<T extends VolumeIdentity>(volumes: readonly T[]): Array<{ key: string; name: string; bound: boolean; volumes: T[] }> {
  const groups = new Map<string, { key: string; name: string; bound: boolean; volumes: T[] }>();
  for (const volume of volumes) {
    if (volume.provider !== "drobo") continue;
    const enclosure = volume.enclosureId?.trim();
    const key = enclosure ? `enclosure:${enclosure}` : `unbound:${volume.id}`;
    let group = groups.get(key);
    if (!group) {
      group = { key, name: volume.deviceName?.trim() || "Drobo enclosure", bound: Boolean(enclosure), volumes: [] };
      groups.set(key, group);
    }
    group.volumes.push(volume);
  }
  return [...groups.values()];
}

/** Keep protected allocation, OS filesystem usage and installed bytes separate. */
export function usageAccounting(total: string, used: string, free: string): { total: string; used: string; free: string } | null {
  const values = [total, used, free];
  if (values.some((value) => !/^(0|[1-9][0-9]*)$/.test(value) || value.length > 20)) return null;
  const [t, u, f] = values.map(BigInt);
  if (t === 0n || t > 18446744073709551615n || u + f !== t) return null;
  return { total, used, free };
}

/** Missing or malformed drive measurements must not become measured zero bytes. */
export function installedCapacity(values: readonly (string | null | undefined)[]): { bytes: string; populated: number } | null {
  if (values.length === 0 || values.some((value) => typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value) || value.length > 20 || BigInt(value) > 18446744073709551615n)) return null;
  const bytes = values.map((value) => BigInt(value!));
  return { bytes: bytes.reduce((sum, value) => sum + value, 0n).toString(), populated: bytes.filter((value) => value > 0n).length };
}
