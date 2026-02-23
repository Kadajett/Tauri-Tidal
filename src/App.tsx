import { BrowserRouter, Routes, Route } from "react-router";
import { AppLayout } from "@/components/layout/AppLayout";
import { SearchPage } from "@/pages/SearchPage";
import { AlbumPage } from "@/pages/AlbumPage";
import { ArtistPage } from "@/pages/ArtistPage";
import { PlaylistPage } from "@/pages/PlaylistPage";
import { LibraryPage } from "@/pages/LibraryPage";
import { FavoritesPage } from "@/pages/FavoritesPage";
import { QueuePage } from "@/pages/QueuePage";
import { SimilarTracksPage } from "@/pages/SimilarTracksPage";

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route path="/" element={<SearchPage />} />
          <Route path="/search" element={<SearchPage />} />
          <Route path="/album/:id" element={<AlbumPage />} />
          <Route path="/artist/:id" element={<ArtistPage />} />
          <Route path="/playlist/:id" element={<PlaylistPage />} />
          <Route path="/library" element={<LibraryPage />} />
          <Route path="/favorites" element={<FavoritesPage />} />
          <Route path="/queue" element={<QueuePage />} />
          <Route path="/similar" element={<SimilarTracksPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

export default App;
