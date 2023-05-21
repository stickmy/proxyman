import { create } from "zustand";

export const useDetailVisible = create<{
  visible: boolean;
  setVisible: (visible: boolean) => void;
}>((set) => ({
  visible: false,
  setVisible: (visible: boolean) => set({ visible }),
}));
