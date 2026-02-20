import { useCallback, useEffect, useRef, useState } from "react";

export function useAutoScroll<T extends HTMLElement>(dependency: unknown) {
  const ref = useRef<T>(null);
  const [isAutoScrolling, setIsAutoScrolling] = useState(true);

  const handleScroll = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50;
    setIsAutoScrolling(atBottom);
  }, []);

  useEffect(() => {
    if (isAutoScrolling && ref.current) {
      ref.current.scrollTop = ref.current.scrollHeight;
    }
  }, [dependency, isAutoScrolling]);

  const scrollToBottom = useCallback(() => {
    if (ref.current) {
      ref.current.scrollTop = ref.current.scrollHeight;
      setIsAutoScrolling(true);
    }
  }, []);

  return { ref, isAutoScrolling, handleScroll, scrollToBottom };
}
