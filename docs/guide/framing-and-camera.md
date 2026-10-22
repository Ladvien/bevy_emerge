# Framing and camera

## When the map is framed

The Map frames the whole map once, when it opens. After that the camera is yours: the framing never overwrites a move you have made.

If the map cannot be framed, the editor says so in a problem, rather than retrying every frame.

## Moving the camera

- **W**, **A**, **S**, **D** pan along the screen axes.
- **Q** and **E** turn the view.
- The **scroll wheel** zooms. One wheel detent changes the viewport height by a fixed ratio, so a zoomed-in view stops at a floor rather than springing back.

These keys are the same on every tab of the Map door. The complete list is in [Keys](keys.md).

## Framing from an agent

The MCP tool `emerge_frame_view` frames the Map camera on a box. Its `target` is one of:

- `"map"`: the whole map.
- `"selection"`: the locked placement. It is refused when nothing is locked.
- `{"placements": [ids]}`: the union of the named placements.
- `{"rect": [x0, z0, x1, z1]}`: a rectangle on the floor.

A person has no key to frame the selection or a set of placements. Framing a placement is something the agent does with this tool.

See [Agents and the MCP server](agents-and-mcp.md).

## Snips and the camera

A snip (`Cmd+\`) records the Map camera's position and direction at the moment the person takes it, when there is a Map camera, so the picture can be read against the view it came from. A snip moves nothing: the camera stays where it was.
