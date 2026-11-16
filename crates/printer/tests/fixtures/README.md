# Recorded jobs

The files here are print jobs as an independent program sends them. The
tests in `../raster.rs` hold this crate's encoder to them byte for byte.

## Where they come from

They were recorded once, outside this repository, with the `brother_ql`
crate, version 3.0.6 (MIT, <https://github.com/mkienitz/brother_ql>),
whose authors have confirmed it on the QL-800, QL-810W and QL-820NWB.
That crate is not a dependency of this one, at build time or at run time.

Each file is the output of `PrintJob::compile()` for one label, built
with `PrintJobBuilder::new(media)`, `.add_label(picture)` and
`.cut_behavior(CutBehavior::CutEach)`, everything else left at that
crate's defaults: one copy, 300 dots to the inch, no compression,
priority to print quality.

The picture is black on white, as wide as the media prints and as long
as given below. The dot `x` across and `y` along is black when

    (x + 2 * y) % 7 == 0 || (x < 24 && y < 12)
        || x + 1 == across || y + 1 == along

which is the page `pattern` in `../support/mod.rs` draws.

## The files

| File | Media of that crate | Media here | Dots |
| --- | --- | --- | --- |
| `ql800-62-150.bin` | `C62` | `62` | 696 x 150 |
| `ql800-62-red-150.bin` | `C62R` | `62-red` | 696 x 150 |
| `ql800-12-150.bin` | `C12` | `12` | 106 x 150 |
| `ql800-62x29.bin` | `D62x29` | `62x29` | 696 x 271 |
| `ql800-23x23.bin` | `D23x23` | `23x23` | 236 x 202 |

## What they are evidence of

That for the QL-800 series this crate sends what a program known to print
on those printers sends. They are not evidence for any other model: the
other models are sent the commands their own references list, which
differ, and are held to those references instead.
