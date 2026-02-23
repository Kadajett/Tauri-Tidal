import { useCallback, useEffect, useRef } from "react";
import { useSearchStore } from "@/stores/searchStore";
import * as tauri from "@/lib/tauri";

export function useSearch() {
  const { query, setQuery, setResults, setSuggestions, setLoading } =
    useSearchStore();
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();

  const performSearch = useCallback(
    async (q: string) => {
      if (!q.trim()) {
        setResults(null);
        setSuggestions([]);
        return;
      }

      setLoading(true);
      try {
        const results = await tauri.searchTidal(q, 20);
        setResults(results);
      } catch (err) {
        console.error("Search failed:", err);
      } finally {
        setLoading(false);
      }
    },
    [setResults, setSuggestions, setLoading],
  );

  const fetchSuggestions = useCallback(
    async (q: string) => {
      if (!q.trim()) {
        setSuggestions([]);
        return;
      }
      try {
        const suggestions = await tauri.searchSuggestions(q);
        setSuggestions(suggestions);
      } catch {
        // Silently fail for suggestions
      }
    },
    [setSuggestions],
  );

  const updateQuery = useCallback(
    (q: string) => {
      setQuery(q);
      clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        fetchSuggestions(q);
      }, 200);
    },
    [setQuery, fetchSuggestions],
  );

  const submitSearch = useCallback(
    (q?: string) => {
      const searchQuery = q ?? query;
      clearTimeout(debounceRef.current);
      setSuggestions([]);
      performSearch(searchQuery);
    },
    [query, performSearch, setSuggestions],
  );

  useEffect(() => {
    return () => clearTimeout(debounceRef.current);
  }, []);

  return { query, updateQuery, submitSearch };
}
