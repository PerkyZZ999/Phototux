# Professional Raster Graphics Editor

## Information Architecture (Vendor Neutral)

---

# Application

```

Application

│

├── Welcome Screen

│   ├── New File

│   ├── Open File

│   ├── Recent Files

│   ├── Templates

│   └── Preferences

│

├── Workspace

│   ├── Menu Bar

│   ├── Toolbar

│   ├── Tool Options Bar

│   ├── Document Tabs

│   ├── Canvas

│   ├── Side Panels

│   ├── Status Bar

│   └── Workspace Manager

│

├── Documents

│   ├── New

│   ├── Open

│   ├── Save

│   ├── Save As

│   ├── Export

│   ├── Print

│   └── Close

│

├── Workspaces

│   ├── Essentials

│   ├── Photography

│   ├── Design

│   ├── Painting

│   ├── Minimal

│   ├── Custom

│   └── Reset Workspace

│

├── Menu System

│   ├── File

│   ├── Edit

│   ├── Image

│   ├── Layer

│   ├── Select

│   ├── Filter

│   ├── View

│   ├── Window

│   └── Help

│

├── Editing System

│   ├── Layers

│   ├── Masks

│   ├── Adjustment Layers

│   ├── Filters

│   ├── Layer Styles

│   ├── Selections

│   ├── Paths

│   ├── Shapes

│   └── History

│

├── Export

│   ├── PNG

│   ├── JPEG

│   ├── TIFF

│   ├── BMP

│   ├── WebP

│   ├── SVG (Vector Layers)

│   └── PDF

│

├── Automation

│   ├── Actions

│   ├── Batch Processing

│   ├── Scripts

│   └── Plugin System

│

└── Preferences

    ├── General

    ├── Interface

    ├── Workspace

    ├── Tools

    ├── History

    ├── Performance

    ├── Scratch Disk

    ├── Units

    ├── Guides

    ├── Grid

    ├── Shortcuts

    └── Themes

```

---



# Main Workspace

```

+--------------------------------------------------------------+

| Menu Bar                                                     |

+--------------------------------------------------------------+

| Tool Options Bar                                             |

+--------------------------------------------------------------+

| Toolbar |           Document Canvas         | Panels         |

|         |                                   | Layers         |

|         |                                   | History        |

|         |                                   | Properties     |

|         |                                   | Color          |

|         |                                   | Brushes        |

|         |                                   | Navigator      |

+--------------------------------------------------------------+

| Status Bar                                                   |

+--------------------------------------------------------------+

```

---



# Menu Architecture



## File

- New
- Open
- Open Recent
- Save
- Save As
- Export
- Print
- Document Properties
- Exit

---



## Edit

- Undo
- Redo
- Cut
- Copy
- Paste
- Fill
- Stroke
- Free Transform
- Preferences

---



## Image

- Image Size
- Canvas Size
- Rotate
- Flip
- Crop
- Trim
- Duplicate
- Color Mode
- Bit Depth
- Adjustments

---



## Layer

- New Layer
- Duplicate
- Delete
- Group
- Merge
- Flatten
- Layer Mask
- Clipping Mask
- Arrange
- Smart Layer (Optional)
- Layer Styles

---



## Select

- Select All
- Deselect
- Reselect
- Inverse
- Modify
- Feather
- Expand
- Contract
- Color Range

---



## Filter

- Blur
- Sharpen
- Noise
- Distort
- Stylize
- Pixelate
- Render
- Other

---



## View

- Zoom
- Guides
- Grid
- Rulers
- Snap
- Screen Mode
- Fullscreen

---



## Window

- Panels
- Workspaces
- Reset Workspace

---



## Help

- Documentation
- Shortcuts
- About

---



# Toolbar



## Selection

- Move
- Marquee
- Lasso
- Polygon Lasso
- Magnetic Lasso
- Magic Wand
- Quick Selection

---



## Crop

- Crop
- Perspective Crop
- Slice

---



## Retouch

- Spot Healing
- Healing Brush
- Clone Stamp
- Patch
- Blur
- Sharpen
- Smudge
- Dodge
- Burn
- Sponge

---



## Paint

- Brush
- Pencil
- Color Replacement
- Mixer Brush
- Paint Bucket
- Gradient

---



## Drawing

- Pen
- Freeform Pen
- Path Selection
- Direct Selection

---



## Text

- Horizontal Type
- Vertical Type
- Text Mask

---



## Shapes

- Rectangle
- Rounded Rectangle
- Ellipse
- Polygon
- Line
- Custom Shape

---



## Navigation

- Hand
- Rotate View
- Zoom

---



## Colors

- Foreground
- Background
- Swap
- Default Colors

---



# Panels



## Document

- Layers
- Channels
- Paths
- History

---



## Properties

- Properties
- Layer Styles
- Adjustments

---



## Painting

- Brushes
- Brush Settings
- Patterns
- Gradients

---



## Color

- Color Picker
- Swatches

---



## Typography

- Character
- Paragraph
- Glyphs

---



## Information

- Navigator
- Histogram
- Info

---



## Automation

- Actions
- Tool Presets

---



# Document Architecture

```

Document

│

├── Canvas

│

├── Artboards (Optional)

│

├── Layer Groups

│   ├── Pixel Layers

│   ├── Text Layers

│   ├── Shape Layers

│   ├── Fill Layers

│   └── Adjustment Layers

│

├── Channels

│

├── Paths

│

└── History

```

---



# Layer Structure

```

Layer

│

├── Name

├── Visible

├── Locked

├── Opacity

├── Blend Mode

├── Mask

├── Clipping Mask

├── Effects

│   ├── Shadow

│   ├── Glow

│   ├── Stroke

│   ├── Overlay

│   └── Bevel

└── Metadata

```

---



# Editing Workflow

```

New/Open

      │

      ▼

Import Images

      │

      ▼

Organize Layers

      │

      ▼

Selections

      │

      ▼

Masks

      │

      ▼

Painting

      │

      ▼

Retouch

      │

      ▼

Color Adjustments

      │

      ▼

Effects

      │

      ▼

Export

```

---



# Functional Modules

| Module | Purpose |

|----------|----------|

| Document Manager | Open/save/export |

| Workspace Manager | UI layout |

| Tool System | Editing tools |

| Layer System | Non-destructive editing |

| History Engine | Undo/Redo |

| Brush Engine | Painting |

| Selection Engine | Pixel selection |

| Path Engine | Bézier paths |

| Text Engine | Typography |

| Filter Engine | Image processing |

| Color Engine | Color management |

| Plugin Manager | Extensions |

| Automation | Actions & Scripts |

| Preferences | User configuration |

---



# Context Menu Architecture

Context menus are dynamic and depend on the object currently under the cursor.

```
Context Menu
│
├── Canvas
├── Layer
├── Layer Group
├── Multiple Layers
├── Selection
├── Guide
├── Ruler
├── Path
├── Shape
├── Text Layer
├── Brush
├── Gradient
├── Swatch
├── History State
├── Panel
├── Tab
├── Document
└── Empty Workspace
```

---



# Canvas Context Menu

Appears when right-clicking the canvas.

- Undo
- Redo
- Step Backward
- Step Forward
- Paste
- Paste in Place
- Free Transform
- Deselect
- Select All
- Invert Selection
- Fill
- Stroke
- Clear
- Rotate View
- Flip View Horizontally
- Zoom In
- Zoom Out
- Fit on Screen
- Actual Pixels
- New Guide
- Canvas Properties

---



# Layer Context Menu

Appears when right-clicking a layer.

- Rename
- Duplicate Layer
- Delete Layer
- Convert to Smart Layer *(optional)*
- Rasterize
- Merge Down
- Merge Visible
- Group Layers
- Ungroup
- Create Clipping Mask
- Release Clipping Mask
- Add Layer Mask
- Delete Layer Mask
- Apply Layer Mask
- Enable Layer Mask
- Select Layer Contents
- Copy Layer Style
- Paste Layer Style
- Clear Layer Style
- Blending Options
- Export Layer
- Layer Properties

---



# Layer Group Context Menu

- Rename Group
- Duplicate Group
- Delete Group
- Expand All
- Collapse All
- Group Selected Layers
- Ungroup
- Merge Group
- Export Group

---



# Multiple Layer Context Menu

- Group
- Merge Layers
- Align
- Distribute
- Link Layers
- Unlink Layers
- Duplicate
- Delete
- Convert to Smart Layer *(optional)*
- Export Selected Layers

---



# Selection Context Menu

Appears when a pixel selection exists.

- Transform Selection
- Feather
- Expand
- Contract
- Border
- Inverse
- Save Selection
- Load Selection
- Fill
- Stroke
- Copy
- Cut
- Paste Into
- Layer via Copy
- Layer via Cut

---



# Text Layer Context Menu

- Edit Text
- Convert to Shape
- Rasterize Text
- Duplicate
- Warp Text
- Character Settings
- Paragraph Settings
- Anti-alias Mode
- Export

---



# Shape Layer Context Menu

- Edit Shape
- Convert to Path
- Rasterize Shape
- Duplicate
- Merge Shape
- Fill Options
- Stroke Options

---



# Path Context Menu

- Make Selection
- Fill Path
- Stroke Path
- Duplicate Path
- Delete Path
- Export Path

---



# Guide Context Menu

- Delete Guide
- Lock Guides
- Unlock Guides
- Hide Guides
- New Guide
- Guide Properties

---



# Brush Context Menu

- Brush Size
- Hardness
- Opacity
- Flow
- Spacing
- Angle
- Roundness
- Rename Preset
- Duplicate Preset
- Delete Preset
- Export Preset

---



# Gradient Context Menu

- Edit Gradient
- Duplicate
- Rename
- Delete
- Import
- Export

---



# Swatch Context Menu

- Rename
- Delete
- Duplicate
- Set Foreground
- Set Background
- Export Palette

---



# History Context Menu

- Delete State
- Create Snapshot
- Clear History
- History Options

---



# Panel Context Menu

Available by right-clicking a panel tab.

- Close Panel
- Close Other Panels
- Undock
- Dock Left
- Dock Right
- Float Panel
- Collapse Panel
- Reset Panel
- Panel Options

---



# Document Tab Context Menu

- Save
- Save As
- Duplicate
- Close
- Close Others
- Reveal in File Manager
- Document Properties

---



# Empty Workspace Context Menu

- New Document
- Open Document
- Paste
- Reset Workspace
- Workspace Manager
- Preferences

---



# Context Menu Guidelines



## Object-Oriented

Every object owns its own menu.

Examples:

Canvas
→ Editing actions

Layer
→ Layer actions

Text
→ Typography actions

Brush
→ Brush settings

Panel
→ UI actions

---



## Progressive Disclosure

Show only actions relevant to the selected object.

Examples:

Text Layer

✓ Edit Text

✓ Warp

✓ Rasterize

✗ Merge Group

✗ Make Selection

---



## Destructive Actions

Always grouped at the bottom.

Example

---

Delete Layer

Clear History

Flatten Image

Rasterize

---



## Frequently Used Actions

Appear first.

Typical order

1. Edit
2. Duplicate
3. Rename
4. Properties

---

Advanced

---

Delete

---



## Keyboard Shortcuts

Display shortcuts when available.

Example

Undo               Ctrl+Z

Duplicate          Ctrl+J

Free Transform     Ctrl+T

Delete             Del

---



# Internal Architecture

```
Right Click
      │
      ▼
Hit Testing
      │
      ▼
Identify Object
      │
      ▼
Menu Factory
      │
      ▼
Populate Commands
      │
      ▼
Enable / Disable Items
      │
      ▼
Display Context Menu
```

The **Menu Factory** should dynamically build each context menu from the currently selected object(s), making it easy to extend the application with new tools, panels, layer types, and plugins without hardcoding menu definitions.

# Design Principles

- Document-centric editing
- Layer-based composition
- Dockable panels
- Context-sensitive tool options
- Fully customizable workspace
- Non-destructive editing
- Multi-document interface
- Keyboard-first workflow
- Hardware-accelerated rendering
- Extensible plugin architecture

