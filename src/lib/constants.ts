export const ROUTES = {
  SEARCH: "/",
  ALBUM: "/album/:id",
  ARTIST: "/artist/:id",
  PLAYLIST: "/playlist/:id",
  LIBRARY: "/library",
  FAVORITES: "/favorites",
  QUEUE: "/queue",
  SIMILAR: "/similar",
} as const;

export const KEYBOARD_SHORTCUTS = {
  TOGGLE_PLAY: " ",
  SEEK_FORWARD: "ArrowRight",
  SEEK_BACK: "ArrowLeft",
  VOLUME_UP: "ArrowUp",
  VOLUME_DOWN: "ArrowDown",
  NEXT_TRACK: "n",
  PREV_TRACK: "p",
  TOGGLE_MUTE: "m",
  FOCUS_SEARCH: "/",
} as const;
