import { create } from "zustand";

import { getProcessorPacks, ProcessorPackTransfer } from "@/Commands/Commands";
import { useEffect, useState } from "react";

export const usePackStore = create<{
  currentPack?: string;
  packs: ProcessorPackTransfer[];
  addPack: (packName: string) => void;
  setPacks: (packs: ProcessorPackTransfer[]) => void;
  updatePackStatus: (packName: string, status: boolean) => void;
  setCurrentPack: (packName: string) => void;
}>((set) => ({
  packs: [],
  setPacks: (packs: ProcessorPackTransfer[]) => set((state) => ({ packs })),
  addPack: (packName: string) =>
    set((state) => ({
      packs: [
        ...state.packs,
        {
          packName,
          enable: false,
        },
      ],
    })),
  updatePackStatus: (packName: string, status: boolean) =>
    set((state) => ({
      packs: state.packs.map((pack) =>
        pack.packName === packName
          ? {
              packName,
              enable: status,
            }
          : pack
      ),
    })),
  setCurrentPack: (packName: string) => set(state => ({ currentPack: packName }))
}));

getProcessorPacks().then((packs) => usePackStore.setState({ packs }));
