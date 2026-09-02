---
title: Blend modes
description: >-
  All 28 modes, in the order the menu shows them, grouped by family, with what
  each one does to the layers underneath.
---

A layer's blend mode decides how it combines with the composite underneath it.
The list in the Layers panel is banded by family, and the bands are worth
learning: everything in *darken* can only darken, everything in *lighten* can
only lighten, and everything in *contrast* pivots around mid-grey.

## Normal

| Mode | What it does |
|---|---|
| **Normal** | Replaces what is underneath, scaled by opacity. The default. |
| **Pass Through** | Groups only. The layers inside composite as if the group were not there. |

## Darken

Every mode here leaves white unchanged and can only make the result darker.

| Mode | What it does |
|---|---|
| **Darken** | Takes the darker of the two, channel by channel. |
| **Multiply** | Multiplies the two. The workhorse for shadows and for putting ink on paper. |
| **Color Burn** | Darkens by increasing contrast, crushing the shadows. |
| **Linear Burn** | Darkens by subtracting brightness. Flatter than Color Burn. |
| **Darker Color** | Takes the darker of the two *pixels*, judged by luminosity, rather than working channel by channel. |

## Lighten

The mirror of darken: black is unchanged and the result can only get lighter.

| Mode | What it does |
|---|---|
| **Lighten** | Takes the lighter of the two, channel by channel. |
| **Screen** | The inverse of Multiply. The workhorse for glows and highlights. |
| **Color Dodge** | Brightens by decreasing contrast, blowing out the highlights. |
| **Linear Dodge (Add)** | Brightens by adding. What light actually does. |
| **Lighter Color** | Takes the lighter of the two pixels by luminosity. |

## Contrast

These pivot around mid-grey: 50% grey leaves the backdrop unchanged, darker
values darken and lighter values lighten.

| Mode | What it does |
|---|---|
| **Overlay** | Multiply on the dark half, Screen on the light half, judged by the *backdrop*. |
| **Soft Light** | A gentler Overlay. The usual choice for dodging and burning on a grey layer. |
| **Hard Light** | Overlay judged by the *layer* instead of the backdrop. |
| **Vivid Light** | Color Burn below mid-grey, Color Dodge above. Strong. |
| **Linear Light** | Linear Burn below, Linear Dodge above. |
| **Pin Light** | Replaces values rather than blending them; useful for special effects, harsh for photographs. |
| **Hard Mix** | Pushes every channel to 0 or 255. Posterizes hard. |

## Compare

| Mode | What it does |
|---|---|
| **Difference** | The absolute difference between the two. Two identical layers give black — which is how you check whether they *are* identical. |
| **Exclusion** | A lower-contrast Difference. |
| **Subtract** | Subtracts the layer from the backdrop. |
| **Divide** | Divides the backdrop by the layer. |

## Component

These four take part of the colour from the layer and the rest from the
backdrop. They are defined on whole pixels rather than per channel, so they
have no channel-by-channel form.

| Mode | Takes from the layer | Takes from the backdrop |
|---|---|---|
| **Hue** | Hue | Saturation and luminosity |
| **Saturation** | Saturation | Hue and luminosity |
| **Color** | Hue and saturation | Luminosity |
| **Luminosity** | Luminosity | Hue and saturation |

**Color** on a layer of flat paint is how you tint a photograph without
touching its tones. **Luminosity** on a sharpened copy is how you sharpen
detail without shifting colour.

## Which ones are separable

A *separable* mode computes each of red, green and blue independently.
Twenty-two of the twenty-eight are. The six that are not — **Hue**,
**Saturation**, **Color**, **Luminosity**, **Darker Color** and **Lighter
Color** — are defined on the pixel as a whole, which is why they cannot be
applied one channel at a time.

## Opacity and fill

The blend runs first, then opacity scales the result against the backdrop. A
layer at 50% Multiply is halfway between the multiplied result and the
original — not a multiply by a half-strength layer.
