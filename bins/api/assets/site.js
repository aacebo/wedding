(() => {
    const path = window.location.pathname === "/" ? "/welcome" : window.location.pathname;
    document.querySelectorAll("[data-nav]").forEach((link) => {
        const href = link.getAttribute("href");
        if (path === href || path.startsWith(`${href}/`)) {
            link.setAttribute("aria-current", "page");
        }
    });

    const revealTargets = document.querySelectorAll("[data-reveal]");
    if ("IntersectionObserver" in window && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
        const observer = new IntersectionObserver((entries) => {
            entries.forEach((entry) => {
                if (!entry.isIntersecting) return;
                entry.target.classList.add("is-visible");
                observer.unobserve(entry.target);
            });
        }, { threshold: 0.18, rootMargin: "0px 0px -6%" });
        revealTargets.forEach((target) => observer.observe(target));
    } else {
        revealTargets.forEach((target) => target.classList.add("is-visible"));
    }

    const inviteDialog = document.querySelector("[data-invite-dialog]");
    const inviteZoom = document.querySelector("[data-invite-zoom]");
    const inviteClose = document.querySelector("[data-invite-close]");
    let inviteReturnFocus = null;

    if (inviteDialog && inviteZoom && inviteClose) {
        inviteZoom.addEventListener("click", () => {
            inviteReturnFocus = inviteZoom;
            inviteDialog.showModal();
            document.body.classList.add("dialog-open");
            inviteClose.focus();
        });
        inviteClose.addEventListener("click", () => inviteDialog.close());
        inviteDialog.addEventListener("click", (event) => {
            if (event.target === inviteDialog) inviteDialog.close();
        });
        document.addEventListener("keydown", (event) => {
            if (event.key === "Escape" && inviteDialog.open) inviteDialog.close();
        });
        inviteDialog.addEventListener("close", () => {
            document.body.classList.remove("dialog-open");
            inviteReturnFocus?.focus();
        });
    }
})();
