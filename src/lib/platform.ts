// Which kind of device the interface runs on. The Android build and the browser preview with a
// phone user agent both get the touch layout.

export const isMobile = typeof navigator !== 'undefined' && /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);
