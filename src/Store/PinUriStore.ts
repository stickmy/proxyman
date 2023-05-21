import { create } from "zustand";

export const usePinUriStore = create<{
  pins: string[];
  pinUri: (uri: string) => void;
  unpinUri: (uri: string) => void;
  clearPins: () => void;

  currentPin: string | undefined;
  setCurrentPin: (uri: string | undefined) => void;
}>((set) => ({
  pins: [],
  pinUri: (uri: string) => set((state) => ({ pins: [...state.pins, uri] })),
  unpinUri: (uri: string) =>
    set((state) => ({
      pins: state.pins.filter((x) => x !== uri),
      currentPin: state.currentPin === uri ? undefined : state.currentPin,
    })),
  clearPins: () => set({ pins: [] }),

  currentPin: undefined,
  setCurrentPin: (uri: string | undefined) => set({ currentPin: uri }),
}));
