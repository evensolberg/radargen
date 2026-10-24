# Security policy

## Supported versions

Security fixes go into the latest release. There are no long-term support branches: upgrade to the newest version to get a fix.

## Reporting a vulnerability

Please report a vulnerability privately, not in a public issue. On GitHub, open the repository's **Security** tab and choose **Report a vulnerability**. That opens a private advisory that only the maintainer can see.

Include what you found, the radargen version, and the steps or input that show it. You should hear back within a week. Once a fix is released, the advisory is published with credit to you, unless you would rather not be named.

## What counts

radargen reads files that may come from someone else and writes files that others open, so these are in scope:

- **Its inputs:** a CSV, a `--config` TOML file or a `--palette` CSS file that makes radargen crash, hang, use unbounded memory, or read or write a file it was not asked to.
- **Its outputs:** an HTML page, SVG or PDF that runs script, loads anything from the network, or shows content the input did not hold. The HTML page is meant to be self-contained and to escape everything it takes from the CSV.
- **Information leaking:** a row or column left out of a page by default (see `--table` in the README) that still appears in it.

Out of scope: a radar that is hard to read because of the colours a palette file chose (a palette's contrast is its author's responsibility), and problems in the browsers or PDF viewers that open radargen's output.
