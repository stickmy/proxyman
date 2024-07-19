import { create } from "zustand";

import {
  type ProcessorPackTransfer,
  getProcessorPacks,
} from "@/Commands/Commands";

export const usePackStore = create<{
  packs: ProcessorPackTransfer[];
  addPack: (packName: string, enable?: boolean) => void;
  setPacks: (packs: ProcessorPackTransfer[]) => void;
  updatePackStatus: (packName: string, status: boolean) => void;
}>(set => ({
  packs: [],
  setPacks: (packs: ProcessorPackTransfer[]) => set(state => ({ packs })),
  addPack: (packName: string, enable?: boolean) =>
    set(state => ({
      packs: [
        {
          packName,
          enable: enable ?? false,
        },
        ...state.packs,
      ],
    })),
  updatePackStatus: (packName: string, status: boolean) =>
    set(state => ({
      packs: state.packs.map(pack =>
        pack.packName === packName
          ? {
              packName,
              enable: status,
            }
          : pack,
      ),
    })),
}));

getProcessorPacks().then(packs => usePackStore.setState({ packs }));
