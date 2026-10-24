# Partman style and design guide

For partman and partman-like, and associated applications.

## General

All controls should have hover texts to describe what they do, and
include any keyboard shortcut if available.

## Workflow

### Workflow Steps

In a workflow there is a number of steps.  Starting a workflow brings
up all steps in the workflow in a top bar, with the first one active.
Steps are normally disabled at start, having prerequisites created in
prior steps.  For example, having a pininfo file enables the
Peripherals step.

### Workflow Step Bar

The workflow steps (previously referred to as "crumbs") at the top can
be clicked to directly move to any enabled step.  Visually the
Workflow Step Bar has the names of the steps in a directional set of
buttons.  Imagine "| Plan >" as a button.  They convery direction and
progress.  Somewhere in each step is a button to advance to the next
step, called the Workflow Advance Button, which is visually a button
with the name of the next step and a short arrow.  It is based
somewhere towards the lower right, visually the "last" control in the
step.  This control can be active even if the Step Bar step button is
disabled; in this case the workflow advance button performs validation
on existing data, or just basic checks if needed, and unlocks the next
step, then advances to it.

### Automatic State Save

No manual state saving is used, instead the state of an incomplete
workflow should be automatically saved with the application state, or
in a project, or document, or what the workflow applies to.

### Recomputation and Invalidation

Going back to a previous step and making changes can cause disabling
of other steps.  When this happens settings made in those now disabled
should attempt to be retained, unless the settings are dependent on
data that has now changed, in which case they may be recalculated to
new defaults or reset.  The exact behavior is decided on a
case-by-case basis, and this is a good decision to ask about.

### Cancel

In some apps a workflow can be canceled, in others it can't because
the workflow embodies the entire app.

### The Work Area

The Step Bar should always indicate which step is currently the active
one and gray out those disabled ones.  A Workflow Step should take
over the app portion used by Workflows (the Workflow work area), but
can limit itself to a "card" view if there are only a few controls or
inputs; in this view the step work area is a smaller rectangle in the
larger area.

### Work Area Scroll Bars

If tables grow or text is added, the card can and should grow
vertically to contain its contents to the extent possible within the
larger allotted app Workflow Area, without necessitating scroll bars.
If it no longer fits, vertical scroll bars are added to scrollable
elements within it, such as tables, unless those scrollable elements
become too small, in which case the step view resorts to vertically
scrolling the entire work area.  For horizontal scrolling only the
entire work area can become scrollable, and only if it can't fit, for
example if a user shrinks the app window; but in general the
horizontal width of the active work area should be static unless it
absolutely can't reasonably fit its contents, or the user resizes it
to a new static size (assuming this functionality is available).

### Modal Workflows

A step can be a dialog (modal window) as well, in which case the Step
Bar is in the dialog. An example of this could be to obtain price
data, which goes through a part search and then review and update.
For such a workflow the last step has a Step Advance control that
identifies the action of the entire workflow, for example Update-> or
Import->, which performs the action and closes the workflow. Such a
dialog can be cancelled at any time, discarding the workflow state.
The app can't be exited while it's active.  (Exception: when built
with an option to permit this, for test purposes, if needed.)

In other apps, the workflow may not be closable if it embodies the
entire app or a project. In the latter case closing the project simply
closes the project, but retains its state.

## Dragging

### General

When dragging visual items like cards around it's important that the
card is visually moved around.  Dragging can be canceled with the ESC
key, facilitating a return to the pre-drag state.

### Dragging to Reorder

If things are moved around and their relative ordering (one or two
dimensions) is changed, the whole visual item dragged should move and
its new order highlighted with a bar between the items when it will be
placed if dropped.  Reordering drags can be canceled mid-drag with the
ESC key, facilitating a return to the pre-drag state (order) similar
to any other drag operation.

When a card or other selectable item is clicked to drag it becomes the
selected item and stays the selected card even if the drag is
cancelled.

## Cards

### General

Think of the app work area as a canvas with cards.

There is a number of items, for example different categories of
information, each of which is a card: pricing, history,
specifications, etc.

A card has a tab, for example "KiCad", "Stock", or "Pricing" at the
top.

The info correlating to the tab then fills the rest of the card.

### Card stacks

Cards are stackable, this is called a card stack.

When two cards are stacked the top shows both tabs, which can be moved
to reorder, similar to browser tabs.

Selecting a tab determines which card is visible.  We'll call this the
top card.

Changing the top card by clicking on a tab does not reorder the tabs.

Moving a card to a stack resizes it to match the stack view.  If moved
again it will retain this size.  Thus all cards in a stack have the
same size, which becomes the stack view if they're separated again.

### Moving and free placement

Card stacks are movable and resizable. They snap to other stack edges
both when moving and resizing.

They also snap to window edges (or against the sidebar).

The default layout is an overview card at the top with the picture and
the overview information, some controls, etc.  Immediately below it,
all the other cards in a single stack, snapped to the overview card.

Until the user moves or resizes a stack, the default layout follows
the window: the stacks are one under another across the whole work
area, each as tall as its top card's contents, and the last one takes
the rest of the height.

To drag a card stack grab it at the top bar area (stack bar) outside
the tabs.

### Overlapping card stacks

Stacks can be placed on top of one another (Z axis), then raised by
clicking on it.

### Rearranging

If a tab is grabbed and dragged sideways, rearrange the tabs.  As
movement starts, enter tab rearrangement mode until released.

When a card is clicked to drag it becomes selected and stays the
selected card even if the drag is cancelled.

If a tab is grabbed and moved up or down, unstack the card that tab
belongs to and move the card, this is card rearrangement mode.  The
card keeps its stack's size.  Dropping it on another stack's bar adds
it to that stack, between the tabs where it's dropped, as its top
card; the bar is highlighted while the card is over it.  Dropped
anywhere else it stays a stack of its own.

Tab and card rearrangement modes are mutually exclusive and both can't
be active, after one is activated prevent the other.

### Resizing a stack

To resize a card stack, grab it in the bottom right corner and drag
the corner around.

### Purpose

The purpose here is for a user to be able to effective use the page as
a board to place the part information as they like.

### Spinner

If information in a card is loading, put a spinner in the stack view
if it's the top (visible) card. Information loading should be done by
background threads, never the foreground UI thread.

### Visual separation/appearance

We want stacks to be visually distinguishable, for example with a
frame outline.  This can simply be a strip of page background color.
The stack view should have a visual bar.  The stacks are then shaded
to stand out from the background, and the stack bar a little more so.

Give each card a little bit of margin, maybe equivalent to two
characters, to help keep them visually distinct.

### State save

The user layout should be persisted and applied when viewing any part
in the catalog.  Each user signed in has their own.  A layout can
also be exported to a file and imported, to take it to another
computer.

A saved layout survives cards coming and going: cards no longer there
leave their stacks, and new ones join the last stack.

### Work area scroll bars

If the app window is sized or resized so not all stack views fit it
should become scrollable in the directions that are overflowing.

A card whose contents don't fit its stack scrolls vertically inside
the card.

## Agent Sidebar

An agent sidebar should have a control on the edge that opens it.
Like a small triangle halfway vertically.  If closed, this slides the
agent sidebar open to cover half the work area by default.  The user
can drag it to adjust their preferred width, which is remembered.
When open the triangle switches to the inside of the agent sidebar and
becomes a closing control.  When the closing control is pressed the
agent sidebar slides shut.  When opened again it returns to the user's
preferred width.  The preferred width is saved with the other agent
settings.  When slide open the width is capped to the entire work area
width.
