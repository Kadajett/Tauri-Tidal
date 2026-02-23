import { useNavigate, useLocation } from "react-router";
import {
  Search,
  Heart,
  Library,
  ListMusic,
  LayoutList,
  LogIn,
  User,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { useLibraryStore } from "@/stores/libraryStore";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";
import { cn } from "@/lib/utils";

const NAV_ITEMS = [
  { icon: Search, label: "Search", path: "/" },
  { icon: Heart, label: "Favorites", path: "/favorites" },
  { icon: Library, label: "Library", path: "/library" },
  { icon: LayoutList, label: "Queue", path: "/queue" },
];

export function Sidebar() {
  const navigate = useNavigate();
  const location = useLocation();
  const playlists = useLibraryStore((s) => s.playlists);
  const { authenticated, userId, checking } = useAuthStore();
  const { startLogin } = useAuth();

  return (
    <div className="flex h-full w-56 flex-col border-r border-border bg-card">
      <div className="p-4">
        <h1 className="text-lg/6 font-bold">MacTidal</h1>
      </div>

      {!checking && (
        <div className="px-2 pb-2">
          {authenticated ? (
            <div className="flex items-center gap-2 rounded-sm px-3 py-2 text-sm/5 text-muted-foreground">
              <User className="size-4" />
              <span className="truncate">{userId ?? "Logged in"}</span>
            </div>
          ) : (
            <Button
              variant="outline"
              size="sm"
              className="w-full justify-start gap-2"
              onClick={startLogin}
            >
              <LogIn className="size-4" />
              Login to Tidal
            </Button>
          )}
        </div>
      )}

      <nav className="flex flex-col gap-1 px-2">
        {NAV_ITEMS.map((item) => (
          <Button
            key={item.path}
            variant="ghost"
            className={cn(
              "justify-start gap-3",
              location.pathname === item.path && "bg-accent",
            )}
            onClick={() => navigate(item.path)}
          >
            <item.icon className="size-4" />
            {item.label}
          </Button>
        ))}
      </nav>

      <Separator className="mx-2 my-3" />

      <div className="px-4 pb-2">
        <span className="text-xs/4 font-semibold uppercase text-muted-foreground">
          Playlists
        </span>
      </div>

      <ScrollArea className="flex-1 px-2">
        <div className="flex flex-col gap-1">
          {playlists.map((playlist) => (
            <Button
              key={playlist.id}
              variant="ghost"
              className={cn(
                "justify-start gap-3 text-sm",
                location.pathname === `/playlist/${playlist.id}` &&
                  "bg-accent",
              )}
              onClick={() => navigate(`/playlist/${playlist.id}`)}
            >
              <ListMusic className="size-4 shrink-0" />
              <span className="truncate">{playlist.name}</span>
            </Button>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
