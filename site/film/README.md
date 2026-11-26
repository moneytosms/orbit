# The Orbit film

The landing page's product film (`public/clips/orbit-film.{mp4,webm}`) is rendered from code, so it can be
re-shot whenever the app changes.

1. `capture.mjs` opens the real web app (demo build, fake in-browser server), seeds a made-up planner with
   `seed.js` through the app's own sync commands, and saves 2x screenshots: stills, one frame per keystroke
   for typing, before/after frames for clicks, plus the on-screen boxes the film's cursor and camera aim at
   (`assets/manifest.json`).
2. `film.html` + `film.css` + `film.js` compose those frames into a 1920x1080 motion piece. One paused GSAP
   timeline drives every camera move, caption and transition.
3. `render.mjs` seeks that timeline frame by frame in headless Chromium and pipes the frames to ffmpeg, so
   the result is a smooth 60 fps video with no dropped frames.

```bash
just wasm
(cd app && VITE_DEMO=1 pnpm exec vite --port 5199 --strictPort) &
cd site
node film/capture.mjs                 # -> film/assets/
node film/render.mjs --stills 10,40   # spot-check single frames in film/out/
node film/render.mjs                  # -> public/clips/orbit-film.{mp4,webm} + poster (about 30 min)
```

Open `film/film.html?play` through any static server rooted at `site/` to preview it in real time.
`film/assets/` and `film/out/` are build output and are not committed. The site's screenshots in
`public/shots/` are crops of the same captures.
