# Roadmaps are free-positioned graphs with bands

Status: Accepted
Date: 2026-11-24

A Roadmap is a graph, not a row of fixed rectangles. Each node is a row with a kind (`task`, `project`, `goal`, `milestone`, `decision`, `note`, `text`, `heading`), a free `x`/`y` position and an optional `band_id`. Edges are rows with a kind (`then`, `blocks`, `depends_on`, `decides`) and a label. Nodes and edges sync like every other row, with per-field last-writer-wins, so moving a node on one device and renaming it on another both survive. Task, project, goal and milestone nodes reference an existing row; the graph never owns those rows.

A Phase becomes a **band**: a named, optionally dated region that nodes belong to by explicit `band_id`. The canvas draws a dashed band around its member nodes, and phase progress is derived from those nodes. The old `Placement` rows are kept readable and are migrated to nodes once, with stable ids, so existing roadmaps open with their phases as bands and their items as nodes. Week buckets are a view produced from dates, and there is no "Unplaced" tray: an unplaced item is a node no band covers.

## Considered options

- **Decision nodes**: purely visual, or recording the choice. Chosen: choosing a branch marks the node decided (`taken_edge_id`, `decided_at`) and hides the untaken branch. Nothing is deleted, and clearing the choice brings the branches back.
- **Bands**: manual regions (a box dragged around nodes), or explicit membership. Chosen: explicit `band_id`, because membership syncs and derives progress reliably; the drawn region follows the members.
- **Placement**: fold the List, Board and Timeline lenses onto nodes, or keep Placement for them. Chosen: the lenses read the graph, and Placement stays only as the migration source and for compatibility with older clients.
- **Layout**: `dagre` or `elkjs`, or a hand-rolled layered pass. Chosen: a hand-rolled layered pass (rank by longest path from the sources, then stack by title). It is about 40 lines, adds nothing to the PWA bundle, and only runs when the user presses **Tidy up**; it writes ordinary node positions.
- **Rendering**: canvas2d or SVG/DOM. Chosen: absolutely positioned DOM nodes with SVG edges inside one CSS-transformed layer. Nodes need real text wrapping, focus, tooltips, drag and drop and screen-reader names, which canvas2d would reimplement. Pan and zoom change only one transform, so they do not re-layout nodes. The cost is one element per node; the target is 300 to 500 nodes, which is small for the DOM.
- **Code format**: pure Mermaid, or a bespoke language. Chosen: Mermaid `flowchart` plus `%%orbit` directive lines, versioned with `%%orbit:v1`. A diagram pasted from an AI mostly works as Mermaid, the Orbit lines add positions, kinds, references and bands, and the parser rejects an unknown version instead of reinterpreting it.

## Consequences

- Roadmaps can be exported with **Copy as code** and re-imported; the round trip is tested in `orbit-core`.
- Old clients that only know Placement keep working against migrated data, but cannot edit the graph.
- Undo on the canvas covers moves; creating or deleting a node is undone from Trash or by deleting it.
