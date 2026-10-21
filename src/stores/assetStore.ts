import { create } from "zustand";
import { assetApi } from "@/services";
import type { Asset, AssetDetail, AssetType } from "@/types";

export type GalleryMode = "grid" | "compact" | "list";

interface AssetState {
  assets: Asset[];
  loading: boolean;
  loadedForProject: string | null;
  search: string;
  typeFilter: AssetType | "all";
  providerFilter: string;
  mode: GalleryMode;
  sortDesc: boolean;
  detail: AssetDetail | null;
  load: (projectId: string) => Promise<void>;
  setFilters: (patch: Partial<Pick<AssetState, "search" | "typeFilter" | "providerFilter" | "mode" | "sortDesc">>) => void;
  openDetail: (assetId: string) => Promise<void>;
  closeDetail: () => void;
  clear: () => void;
}

export const useAssetStore = create<AssetState>((set) => ({
  assets: [],
  loading: false,
  loadedForProject: null,
  search: "",
  typeFilter: "all",
  providerFilter: "all",
  mode: "grid",
  sortDesc: true,
  detail: null,

  load: async (projectId) => {
    set({ loading: true });
    try {
      const assets = await assetApi.list(projectId);
      set({ assets, loadedForProject: projectId });
    } finally {
      set({ loading: false });
    }
  },

  setFilters: (patch) => set(patch),

  openDetail: async (assetId) => {
    const detail = await assetApi.detail(assetId);
    set({ detail });
  },

  closeDetail: () => set({ detail: null }),

  clear: () => set({ assets: [], loadedForProject: null, detail: null }),
}));

/** 过滤输入（与 store 解耦，便于测试与复用） */
export interface AssetFilterInput {
  assets: Asset[];
  search: string;
  typeFilter: AssetType | "all";
  providerFilter: string;
  sortDesc: boolean;
}

/** 过滤 + 排序：纯函数，避免状态冗余 */
export function filterAssets(input: AssetFilterInput): Asset[] {
  const kw = input.search.trim().toLowerCase();
  const list = input.assets.filter((a) => {
    if (input.typeFilter !== "all" && a.type !== input.typeFilter) return false;
    if (input.providerFilter !== "all" && a.providerId !== input.providerFilter) return false;
    if (kw) {
      const hay = `${a.name} ${a.prompt} ${a.tags.join(" ")}`.toLowerCase();
      if (!hay.includes(kw)) return false;
    }
    return true;
  });
  return list.sort((a, b) =>
    input.sortDesc ? b.createdAt.localeCompare(a.createdAt) : a.createdAt.localeCompare(b.createdAt)
  );
}
