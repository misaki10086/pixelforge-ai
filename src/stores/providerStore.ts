import { create } from "zustand";
import { providerApi } from "@/services";
import type { ProviderView } from "@/types";

interface ProviderState {
  providers: ProviderView[];
  loading: boolean;
  loaded: boolean;
  load: () => Promise<void>;
  clear: () => void;
}

export const useProviderStore = create<ProviderState>((set) => ({
  providers: [],
  loading: false,
  loaded: false,

  load: async () => {
    set({ loading: true });
    try {
      const providers = await providerApi.list();
      set({ providers, loaded: true });
    } finally {
      set({ loading: false });
    }
  },

  clear: () => set({ providers: [], loaded: false }),
}));
