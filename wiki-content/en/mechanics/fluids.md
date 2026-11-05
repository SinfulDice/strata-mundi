## Principle

Water and lava **flow**. Each liquid has **sources** (the water of seas and lakes, the lava lakes of the depths) and thinner and thinner **flows**: downwards first, then sideways, heading for the nearest slope. Water spreads up to 7 blocks on flat ground, lava only 3, and much more slowly. Removing the source makes the flow go away. Water and lava touching each other give **stone** or **obsidian**.

## What you see

The surface of a liquid **slopes** with the flow: full near the source, almost flat at the end. Two cells of the same liquid are never split by a wall, and a liquid with more of itself above fills its whole block (waterfalls). Water is translucent, lava opaque; their surface **ripples** gently, which the graphics option "Liquid animation" stops.

## Swimming

- You **sink slowly** when you touch nothing.
- **Hold jump**: you rise, until you float at the surface.
- You move **half as fast** in water, much slower still in lava.
- Against a bank, a **little kick** helps you climb out of the water.
- With your head in a liquid, the view fades into a blue (water) or orange (lava) **fog**.
- Flying in creative, liquids don't get in the way.

## The bucket (creative)

In the **Tools** tab of the creative inventory:
- the **water bucket** and the **lava bucket** pour a source with a **right click**, against the targeted block;
- the empty **bucket**, pointed at a source, **takes it back**.

In creative, the bucket stays as it is after use. The server checks every use (reach, targeted block), just like placing a block.

## Settings

The world option `fluids` sets the flow (`flow: false` stops it) and how many liquid cells are updated each tick (`updates_per_tick`).
