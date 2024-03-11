import { create } from "zustand";
import { Connection } from "@/Store/ConnectionStore";

export const useConnActionStore = create<{
  filter?: (conn: Connection) => boolean;
  setFilter: (filter?: (conn: Connection) => boolean) => void;
  //
  detailVisible: boolean;
  setDetailVisible: (visible: boolean) => void;
}>((set) => ({
  setFilter: (filter) => set({ filter }),
  detailVisible: false,
  setDetailVisible: (detailVisible: boolean) => set({ detailVisible }),
}));
