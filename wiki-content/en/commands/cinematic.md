## Description

`/cinematic` plays a cinematic: a scripted camera shot, described in a pack, that sweeps over the surroundings like in a film. The base pack contains a demonstration one.

## Syntax

`/cinematic <id>`, where `id` is the cinematic's full identifier (namespace included).

## Examples

- `/cinematic strata:showcase`: the camera rises and circles around where you are.

## Behaviour

Handled directly by your screen, in every game mode. The shot starts from **your position**: the same cinematic works anywhere. While it plays, you can't move, the interface gives way to black bars and any subtitles, then the view returns to your eyes. Escape (or B on a gamepad) skips the cinematic if it allows it. An unknown identifier is reported.

## Tips

Stand somewhere open before playing a shot: the camera doesn't steer around obstacles.

## Discovery in game

`/help cinematic` recalls the syntax, and Tab completes the identifiers of every cinematic available in your packs.
