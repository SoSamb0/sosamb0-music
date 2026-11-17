// Which kind of device the interface runs on. The Android app says so itself (an init script from
// Rust, see lib.rs); the browser preview with a phone user agent gets the touch layout too.

declare global {
	interface Window {
		__LUMEN_MOBILE__?: boolean;
	}
}

export const isMobile =
	typeof window !== 'undefined' && (window.__LUMEN_MOBILE__ === true || /Android|iPhone|iPad|iPod/i.test(navigator.userAgent));
