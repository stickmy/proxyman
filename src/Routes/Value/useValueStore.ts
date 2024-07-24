import { getValueList } from "@/Commands/Commands";
import { create } from "zustand";

export const useValueStore = create<{
  values: string[];
  addValue: (name: string) => void;
  removeValue: (name: string) => void;
}>((set) => ({
  values: [],
  addValue: (name: string) =>
    set((state) => ({ values: [...state.values, name] })),
  removeValue: (name: string) =>
    set((state) => ({ values: state.values.filter((x) => x !== name) })),
}));

getValueList().then((values) => useValueStore.setState({ values }));
