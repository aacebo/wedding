(() => {
    const path = window.location.pathname === "/" ? "/welcome" : window.location.pathname;
    document.querySelectorAll("[data-nav]").forEach((link) => {
        const href = link.getAttribute("href");
        if (path === href || path.startsWith(`${href}/`)) {
            link.setAttribute("aria-current", "page");
        }
    });

    const revealTargets = document.querySelectorAll("[data-reveal], [data-pet-cameo]");
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

    const petTracks = [...document.querySelectorAll("[data-pet-scroll]")];
    if (!petTracks.length || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    let scheduled = false;
    const updatePetTracks = () => {
        petTracks.forEach((track) => {
            const pet = track.querySelector("[data-pet-walker]");
            if (!pet) return;
            const rect = track.getBoundingClientRect();
            const progress = Math.max(0, Math.min(1, (window.innerHeight - rect.top) / (window.innerHeight + rect.height)));
            const travel = Math.max(0, track.clientWidth - pet.offsetWidth - 24);
            track.style.setProperty("--pet-x", `${Math.round(travel * progress)}px`);
        });
        scheduled = false;
    };
    const scheduleUpdate = () => {
        if (scheduled) return;
        scheduled = true;
        window.requestAnimationFrame(updatePetTracks);
    };
    updatePetTracks();
    window.addEventListener("scroll", scheduleUpdate, { passive: true });
    window.addEventListener("resize", scheduleUpdate);
})();
