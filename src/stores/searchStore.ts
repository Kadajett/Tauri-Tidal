import { create } from "zustand";
import type { SearchResults } from "@/types/search";

interface SearchStoreState {
  query: string;
  results: SearchResults | null;
  suggestions: string[];
  loading: boolean;

  setQuery: (query: string) => void;
  setResults: (results: SearchResults | null) => void;
  setSuggestions: (suggestions: string[]) => void;
  setLoading: (loading: boolean) => void;
}

export const useSearchStore = create<SearchStoreState>((set) => ({
  query: "",
  results: null,
  suggestions: [],
  loading: false,

  setQuery: (query) => set({ query }),
  setResults: (results) => set({ results }),
  setSuggestions: (suggestions) => set({ suggestions }),
  setLoading: (loading) => set({ loading }),
}));
