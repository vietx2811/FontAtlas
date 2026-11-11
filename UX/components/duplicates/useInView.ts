import { useEffect, useRef, useState } from 'react';

/** Đã từng vào khung nhìn chưa (giữ true sau đó): chỉ nạp font cho thẻ người dùng thực sự cuộn tới. Không có IntersectionObserver (test, WebView cũ) → luôn true. */
export function useSeen<T extends Element>(): [React.RefObject<T | null>, boolean] {
  const ref = useRef<T | null>(null);
  const [seen, setSeen] = useState(() => typeof IntersectionObserver === 'undefined');
  useEffect(() => {
    if (seen || !ref.current) return;
    const io = new IntersectionObserver(es => { if (es.some(e => e.isIntersecting)) { setSeen(true); io.disconnect(); } }, { rootMargin: '300px' });
    io.observe(ref.current);
    return () => io.disconnect();
  }, [seen]);
  return [ref, seen];
}
