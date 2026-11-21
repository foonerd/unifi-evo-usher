# Security

## Reporting a flaw

A flaw that lets someone in who should be kept out, or lets someone do more than they may, is reported privately and not in a public issue: on GitHub, under this repository's **Security** tab, with **Report a vulnerability**. The report is seen by the maintainer alone until a fix is out.

What helps:

- the version, as `evo-usher-steward --version` says it;
- how the unit is run: from a package, from the image, with Compose, behind which proxy;
- what was done, what happened, and what should have happened.

It is answered on the report. A fix comes in a release, and the release says what was fixed; whoever reported it is named there if they wish.

A flaw that turns out to be the evo framework's own, and not the usher's, can be reported here all the same: it is passed on.

## Which versions

The latest release is the one that is fixed.

## What the usher holds to

The manual says who may ask what of a unit, and how that is held: [Who may ask](https://github.com/foonerd/unifi-evo-usher/wiki/Who-may-ask). A unit that is reached from beyond its own network stands behind a proxy with the site's certificate: [Behind a proxy](https://github.com/foonerd/unifi-evo-usher/wiki/Behind-a-proxy).
