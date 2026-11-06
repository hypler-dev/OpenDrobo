(() => {
  const overlay = document.getElementById("opendrobo-startup");
  const status = document.getElementById("opendrobo-startup-status");
  const dismissButton = document.getElementById("opendrobo-startup-dismiss");
  const errorShell = document.getElementById("opendrobo-startup-error");
  let dismissed = false;
  let ready = false;
  let fallbackDismissTimer = 0;

  const dismiss = () => {
    if (dismissed || !overlay) return;
    dismissed = true;
    overlay.classList.add("is-dismissed");
    if (overlay.contains(document.activeElement)) {
      const destination = errorShell?.hidden === false
        ? errorShell : document.querySelector("#app main");
      destination?.focus({ preventScroll: true });
    }
  };

  const showFallback = () => {
    if (ready) return;
    if (status) status.textContent = "The dashboard is still starting. You can open it now.";
    if (errorShell) errorShell.hidden = false;
    if (dismissed) {
      errorShell?.focus({ preventScroll: true });
      return;
    }
    dismissButton?.focus({ preventScroll: true });
    window.clearTimeout(fallbackDismissTimer);
    fallbackDismissTimer = window.setTimeout(dismiss, 1200);
  };

  const fallbackTimer = window.setTimeout(showFallback, 3000);
  const dismissWhenReady = () => {
    ready = true;
    window.clearTimeout(fallbackTimer);
    window.clearTimeout(fallbackDismissTimer);
    window.removeEventListener("error", showFallback);
    window.removeEventListener("unhandledrejection", showFallback);
    if (errorShell) errorShell.hidden = true;
    // Allow the branded overlay to paint before starting its visible interval.
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => window.setTimeout(dismiss, 700));
    });
  };

  window.OpenDroboStartup = { dismiss, dismissWhenReady };
  dismissButton?.addEventListener("click", dismiss);
  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape") dismiss();
  });
  window.addEventListener("error", showFallback);
  window.addEventListener("unhandledrejection", showFallback);
})();
