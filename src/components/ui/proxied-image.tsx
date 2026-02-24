import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const cache = new Map<string, string>();

export function ProxiedImage({
  src,
  alt,
  className,
  fallbackClassName,
}: {
  src: string;
  alt: string;
  className?: string;
  fallbackClassName?: string;
}) {
  const [dataUri, setDataUri] = useState<string | null>(() => cache.get(src) ?? null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setFailed(false);

    const cached = cache.get(src);
    if (cached) {
      setDataUri(cached);
      return;
    }

    setDataUri(null);
    let cancelled = false;

    invoke<string>("proxy_image", { url: src })
      .then((uri) => {
        if (!cancelled) {
          cache.set(src, uri);
          setDataUri(uri);
        }
      })
      .catch((err) => {
        console.error("[ProxiedImage] proxy_image failed for", src, err);
        if (!cancelled) setFailed(true);
      });

    return () => {
      cancelled = true;
    };
  }, [src]);

  if (failed || (!dataUri && !src)) {
    return <div className={fallbackClassName ?? className ?? "size-12 rounded-xs bg-muted"} />;
  }

  if (!dataUri) {
    return <div className={fallbackClassName ?? className ?? "size-12 rounded-xs bg-muted animate-pulse"} />;
  }

  return (
    <img
      src={dataUri}
      alt={alt}
      className={className}
      onError={() => setFailed(true)}
    />
  );
}
