import { useEffect, useState } from "react";
import * as Toast from "@radix-ui/react-toast";

function isMobileUserAgent(ua: string): boolean {
  return /Android|iPhone|iPad|iPod/i.test(ua);
}

function isMetaMaskBrowser(ua: string): boolean {
  return /MetaMaskMobile/i.test(ua);
}

function buildDeepLink(ua: string): string {
  const { hostname, pathname, search } = window.location;
  const dappPath = `${hostname}${pathname}${search}`;

  if (/Android/i.test(ua)) {
    const fallback = encodeURIComponent(
      "https://play.google.com/store/apps/details?id=io.metamask",
    );
    return `intent://dapp/${dappPath}#Intent;scheme=metamask;package=io.metamask;S.browser_fallback_url=${fallback};end`;
  }

  return `metamask://dapp/${dappPath}`;
}

export default function MetaMaskMobileToast() {
  const [open, setOpen] = useState(false);
  const [deepLink, setDeepLink] = useState("");

  useEffect(() => {
    const ua = navigator.userAgent;
    if (!isMobileUserAgent(ua)) return;
    if (isMetaMaskBrowser(ua)) return;

    setDeepLink(buildDeepLink(ua));
    setOpen(true);
  }, []);

  return (
    <Toast.Provider duration={10_000}>
      <Toast.Root
        open={open}
        onOpenChange={setOpen}
        className="bg-white border border-zinc-200 rounded-xl shadow-lg p-4 flex items-start gap-3 data-[state=open]:animate-in data-[state=closed]:animate-out data-[swipe=end]:animate-out data-[state=closed]:fade-out-80 data-[state=open]:slide-in-from-top-full"
      >
        <div className="flex-1 min-w-0">
          <Toast.Title className="text-sm font-semibold text-zinc-900">
            Open in MetaMask
          </Toast.Title>
          <Toast.Description className="text-xs text-zinc-500 mt-0.5">
            For a seamless experience, open this page in the MetaMask in-app browser.
          </Toast.Description>
        </div>
        <Toast.Action asChild altText="Open in MetaMask">
          <a
            href={deepLink}
            className="shrink-0 text-xs font-medium text-[#192b25] bg-[#e5fff6] border border-[#a7f3d0] px-3 py-1.5 rounded-lg hover:bg-[#d1fae5] transition-colors"
          >
            Open
          </a>
        </Toast.Action>
      </Toast.Root>
      <Toast.Viewport className="fixed top-4 left-1/2 -translate-x-1/2 w-[calc(100%-2rem)] sm:w-96 z-50 flex flex-col gap-2" />
    </Toast.Provider>
  );
}
