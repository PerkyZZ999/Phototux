# Handbook parity depth candidates (2026-07-17)

## GPU sharpen pack

`LayerPackPlan.sharpen` + effect shader mode 7 (Laplacian), aligned with `cpu_sharpen_rgba`.

## Shape booleans

`shape.boolean` (union/intersect/difference/exclusion): coverage bake of two shape layers → new raster layer. Vector-preserving booleans still deferred (DR-028).

## OS clipboard

`arboard` host bridge: copy publishes image to system clipboard; paste falls back to OS image when in-app buffer empty. 64 MiB bound retained.
