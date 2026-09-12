# Speed edit capture

The platformer render adapter serves `level-speed-control`, `level-speed-edit` and
`level-speed-edit-wrong-position`. All traverse the real main menu and use the level reset.
They start with horizontal velocity 300 px/s. The control keeps the authored Speed of 250;
the edited scenarios submit Speed 100 through `DebuggerEndpoint` during reset and wait for
its `First`-drain acknowledgement before frame 0. They never call the mutation implementation
directly. The request and acknowledgement are retained in `platformer-render/`.

Player movement decelerates by Speed / 2 per tick when no input is held. The control therefore
moves (175 + 50) / 60 = 3.75 px; Speed 100 moves (250 + 200 + 150 + 100 + 50) / 60 = 12.5 px.
Both should rest by frame 30 and stay there at frame 120. The manifests assert those predicted
positions with the level's 1 px tolerance. These predictions have not been measured in round 2.
The existing `level` remains the zero-velocity reference at x=400.

```bash
CAPTURE_RUN=entity-viewer-round2 devenv shell -- .claude/skills/capture-scenario/scripts/run.sh platformer-2d capture-audio,capture-input level level-speed-control level-speed-edit level-speed-edit-wrong-position
```

Expected exits: 0, 0, 0, 1. The negative differs from the edited manifest only in its scenario
name and frame-30 x expectation (400). Require exactly one difference: `(30, "nodes.Player2D.position")`; the comparator
reports the whole position vector when its x coordinate differs. The normal capture facts, diff files and PNGs are the movement evidence.
`speed-edit-ack.txt` must contain the accepted Float(100.0), written before READY/frame 0.
A rejected or missing acknowledgement prints `CAPTURE_ERROR`, prevents READY, and exits 1
with `ack=false complete=false`. The negative must report `ack=true complete=true` and exactly
the difference above; exit 1 alone does not establish that the position assertion ran.
