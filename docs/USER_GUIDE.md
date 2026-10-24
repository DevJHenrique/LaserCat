# LaserCat user guide

See the [README](../README.md) for an overview and [FILE_FORMAT.md](FILE_FORMAT.md) for project compatibility. UI labels below use the English interface. Choose **Language → Português (Brasil)** in the top bar to switch; the choice is saved on this computer.

## First job

Ports are discovered on startup and refreshed every three seconds while disconnected. Windows discovery also checks the serial-port registry. You can enter `COM3` manually and select a baud rate (115200 by default). Connecting does not require bed dimensions, but movement/jobs remain blocked until the bed is configured. Disconnect LaserGRBL before using the same port.

1. Configure the actual usable width/height, axis orientation, and homing capability. These settings do not automatically rewrite firmware parameters.
2. Import SVG, PNG, or JPEG, or create rectangles, circles, and text. Drag artwork and use Ctrl-click for multiple selection.
3. Configure line engraving, filled engraving, raster engraving, or cutting operations. Each operation has power, speed, passes, and priority; an object can engrave before cutting.
4. Generate toolpaths. Correct out-of-bounds artwork or raster margins. Inspect scale, orientation, and overlaps.
5. In **Device**, select **GRBL simulator** to try the flow without hardware. For a real controller, choose the correct COM port with simulation disabled.
6. In user-origin mode, connection defines the current position as G54 X0/Y0, without movement. If you reposition the job origin, use **Set bed zero here**. In absolute-table mode, home first and then reference the usable lower-left corner.
7. Run **Frame without laser** and verify physical positioning before starting an engraving/cutting job.

## Editing on the bed

Drag blue corners to resize. Uniform scaling follows the object's proportion lock; Shift temporarily toggles it. Drag the circular handle to rotate; Shift snaps to 15-degree steps. Each resize/rotation gesture can be undone once. Locked objects cannot be transformed.

The move, scale, and rotate icons open a floating editor with position, dimensions, percentage scale, angle, and mirroring. Collapsing the **Device**, **Objects**, or **Engraving / cutting** sidebar sections frees vertical space for the others.

Preparing and previewing are separate modes. Preview keeps the same object sidebar in read-only mode, with G-code/time panels on the right. Changing artwork or operation settings requires regenerating the plan.

## Coordinates and simple machines

The editor uses millimeters, X right and Y up. SVG/images are normalized from their top-left input origin. Objects rotate around their center; X/Y describe their rectangle before rotation. Axis inversion maps the bed to G54 coordinates without changing the electrical direction of the motors. Jog controls move the controller's physical axes.

Without sensors, the operator establishes the usable work origin. Software bounds checking cannot discover the machine's physical limits. Absolute mode does not assume homing position equals the usable lower-left corner.

**Return to origin** returns to G54 X0/Y0 with the laser off at 1000 mm/min. It needs an idle, referenced controller and configured bed. It does not send `$H` or redefine zero. Cancellation invalidates reference. Reconnecting in user-origin mode defines a new zero at the current position.

**Homing (sensors)** requires profile support and `$22=1` reported by the controller. The application reports `$22` without enabling it automatically.

## Operations and images

- Power is a percentage of `$30`; execution requires `$31=0` and `$32=1`. Settings are read from the controller rather than rewritten automatically.
- Speed is in mm/min. Default values are editing parameters, not validated material recipes.
- Dynamic `M4` is the default; constant `M3` is an explicit per-operation choice.
- Raster operations support brightness, contrast, inversion, ordered dithering, physical resolution, and scanline spacing. Fully transparent pixels never receive emission.
- Optional laser-off overscan defaults to 0 mm and is included in bounds validation. Acceleration compensation and tonal quality are not guaranteed for every speed/margin combination.
- Engraving precedes cutting. Lower operation priority runs first within each category. Detectable nested closed cuts run inside contours before outer contours.
- Overlapping objects remain independent. Bounding-box intersections are highlighted; paths are not automatically deduplicated or boolean-clipped.

## Calibration

Open calibration from machine configuration or **Device**. Choose engraving/cutting, power and speed ranges, grid size, sample dimensions, and passes. Save a `.lcc` project, then inspect and run it through the normal workflow.

Columns vary power; rows from bottom to top vary speed. Labels are engraved using the lowest power and highest speed in the selected ranges. The generator checks required bed area. Existing unsaved work follows the normal save/discard prompt before the new project opens.

Templates in `calibration/` have no assumed machine dimensions. Configure the actual device before planning them.

## SVG and physical scale

SVG physical units are preserved; unitless pixels use 96 DPI. PNG pHYs and JPEG JFIF density are used when valid, otherwise 96 DPI is assumed. Imported text is embedded as outlines; review font-substitution warnings. Text created in the app uses Arial.

Curves are approximated with an initial 0.05 mm tolerance at final physical scale. Cutting and line engraving follow path centerlines without kerf compensation. Filled engraving respects fill rules and holes. Unsupported SVG effects block vector operations; those assets may be raster-engraved. External resources are rejected.

Automatic vectorization, node editing, nesting, camera support, rotary axes, and other protocols are not implemented.

## Execution and stopping

Streaming uses character counting to respect the serial buffer. Progress is **acknowledged commands**, not exact physical completion. Completion requires all acknowledgements and a subsequent `Idle` status. Estimates do not simulate all controller acceleration behavior.

The project is frozen while running/paused. Pause/resume use GRBL realtime commands. Cancellation resets the controller, clears the queue, and invalidates reference. Disconnections, faults, and resets never automatically resume a job. **Stop and edit** waits for cancellation/reidentification before returning to editing; a communication failure is shown explicitly.

Software buttons depend on controller communication and do not replace the physical stop mechanism.

## Projects and recovery

`.lcc` is a ZIP archive with a versioned JSON manifest and embedded assets. Saving uses a temporary file in the same directory and atomic replacement. Edited projects are eligible for local recovery every 30 seconds at `%LOCALAPPDATA%\LaserCat\recovery.lcc`. Startup offers recovery when available, and a clean save removes the recovery copy. Closing unsaved work offers save, discard, or cancel.

Undo/redo retains up to 30 recent states while sharing immutable assets. Local device settings stay separate from object history. Old `.laser` projects and previous local settings remain supported.

Current limits: 64 MiB per asset, 512 MiB per project, 40 million processed pixels, and one million path segments. Increase scan spacing or simplify artwork if a job exceeds processing limits.
