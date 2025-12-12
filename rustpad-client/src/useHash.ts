import { useEffect, useState } from "react";

function getHash() {
  if (!window.location.hash) {
    const id = crypto.randomUUID();
    window.history.replaceState(null, "", "#" + id);
  }
  return window.location.hash.slice(1);
}

function useHash() {
  const [hash, setHash] = useState(getHash);

  useEffect(() => {
    const handler = () => setHash(getHash());
    window.addEventListener("hashchange", handler);
    return () => window.removeEventListener("hashchange", handler);
  }, []);

  return hash;
}

export default useHash;
