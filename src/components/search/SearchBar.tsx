import { Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import { useSearch } from "@/hooks/useSearch";
import { useSearchStore } from "@/stores/searchStore";
import { useCallback, useRef, useState } from "react";

export function SearchBar() {
  const { query, updateQuery, submitSearch } = useSearch();
  const suggestions = useSearchStore((s) => s.suggestions);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        setShowSuggestions(false);
        submitSearch();
      }
      if (e.key === "Escape") {
        setShowSuggestions(false);
        inputRef.current?.blur();
      }
    },
    [submitSearch],
  );

  return (
    <div className="relative">
      <div className="relative">
        <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          ref={inputRef}
          data-search-input
          value={query}
          onChange={(e) => {
            updateQuery(e.target.value);
            setShowSuggestions(true);
          }}
          onKeyDown={handleKeyDown}
          onFocus={() => setShowSuggestions(true)}
          onBlur={() => setTimeout(() => setShowSuggestions(false), 200)}
          placeholder="Search tracks, albums, artists..."
          className="pl-10"
        />
      </div>
      {showSuggestions && suggestions.length > 0 && (
        <div className="absolute top-full z-50 mt-1 w-full rounded-sm border border-border bg-popover p-1 shadow-sm">
          {suggestions.map((suggestion, i) => (
            <button
              key={i}
              className="flex w-full items-center rounded-xs px-3 py-2 text-sm/5 hover:bg-accent"
              onMouseDown={() => {
                updateQuery(suggestion);
                submitSearch(suggestion);
                setShowSuggestions(false);
              }}
            >
              {suggestion}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
