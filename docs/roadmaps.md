# Roadmaps

For daily users who want to plan a goal as a graph: tasks, decisions and notes you can place anywhere, connect and hand to an AI as text. A Roadmap is owned by an Area, Project or Goal and has no done-condition of its own.

## Before you start

- An owner for the Roadmap: an Area, Project or Goal.
- Some tasks to reference (optional). The canvas can also create new ones.

## Steps

### Build it on the canvas

1. Create a Roadmap from the Roadmaps screen or the **+** menu, then open it on the **Canvas** tab.
2. Use the tool dock at the bottom of the canvas, like a drawing board. Pick a tool (Select V, Hand H, Task T, Decision D, Note N, Heading G, Connector C; digits 1 to 7 also work), then click empty space to place it. Esc returns to Select. Click an active tool again to drop one at the centre of the view, next to the selected node and connected to it. Double-click empty space to add a task there. **Band** (B) frames the selected nodes as a stage, and **Add existing** (X) places a Goal, Milestone, Project or existing task. You can also drag an open task from **Task source** onto the canvas (dropping it on a node connects the two). The **?** at the end of the dock explains every tool.
3. Drag nodes anywhere. Hold Space or use the Hand tool or middle mouse button to pan; scroll or pinch to zoom. **Snap** aligns them to a 24 px grid and guides appear when edges line up. Hold Shift and drag the background to select several nodes. Scroll to pan; Ctrl plus scroll or the zoom buttons zoom; **Map** shows a minimap.
4. Connect nodes with the Connector tool (drag from one node to another), by dragging from the dot on a node's right edge, or with **Connect to...**. Arrows leave and enter from any side and may point in any direction; they always touch the node border. Select a connection to set its kind (then, blocks, depends on, decides), give it a label such as `yes`, drag its endpoints to reroute it, or delete it.
5. A Decision is a diamond-shaped node with labelled branches. Select it and choose the branch you took: the node is marked decided, the untaken branch is hidden, and nothing is deleted.
6. Add **Bands** for stages or time periods (for example "Week 1" or "Phase: Research") and assign nodes to them in the selection panel, where you can also rename the band and set its dates. A band is a frame drawn around its nodes in any layout, and drives the progress of its phase.
7. **Tidy up** lays the graph out in layers from its connections, in the direction you pick: left to right, top to bottom, right to left or bottom to top. Nothing is auto-arranged unless you ask. Keyboard: Tab moves through nodes, arrows nudge, Enter opens the row, Delete removes, Ctrl+Z undoes the last move.
8. Select a node and use **Convert to** (or right-click it) to change it between task, decision, note and heading. A task turned into a note stays in your task lists; a note turned into a task creates a real task. Linked goals, projects and milestones cannot become tasks. **Expand** shows the canvas full screen; Esc exits.
9. The **List**, **Board** and **Timeline** tabs show the same graph by phase and date.

### Copy it to or from an AI

1. Choose **Import** on the canvas toolbar (or **Import from code...**), then the **Orbit code** tab. **Insert example** fills in a small working roadmap to learn from.
2. **Copy for AI** copies a prompt that first tells the AI to ask you clarifying questions (goal, deadline, scope, phases, dependencies, decisions, existing items), then to answer with the code. It includes the annotated format, the closed list of node and edge kinds, a worked example and the real ids of your Areas, Projects, Goals and open tasks, so the AI references them instead of inventing duplicates. Paste it into a chat and ask for the plan.
3. Paste the single code block the AI returns into the text box. A live preview lists errors by line and counts new and updated nodes, edges and bands. **Import** writes nothing until you press it.
4. **Copy as code** exports the open Roadmap in the same format, so you can send it to an AI and paste the answer back. The format is Mermaid `flowchart` plus `%%orbit` lines and starts with `%%orbit:v1`; the parser is strict about the version.

## Verify it worked

Reload the page and open the Roadmap on another device after syncing: node positions, connections, labels and decided branches are unchanged. Copy as code, then import it into a new Roadmap and compare the preview: it should report the same nodes, edges and bands.

## Troubleshooting

**Import says "Line N: ...".** The parser reports the 1-based line of the first problem. Unknown node or edge kinds are errors, not guesses: ask the AI to use only the listed kinds.

**A node I deleted is gone from the canvas.** Deleted rows wait 30 days in Trash. Restore it there.

**The canvas is cramped on a phone.** The toolbar shows icons only on narrow screens. Use the zoom buttons, Fit and the minimap to move around.

## Next

[CLI: install and sign in](cli-install.md): script Orbit from the terminal.
