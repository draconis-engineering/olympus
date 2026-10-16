# APIs and Draconis Integration

**Status:** Proposed, 2026-10-16

## Integration philosophy

OAE should be useful standalone and integrate with Olympus when available. Integration should be explicit, versioned, and resilient to products being absent or offline.

## Product discovery

A shared Draconis home/registry concept has been discussed to help products discover other installed products and integration endpoints.

The discovery result should distinguish at least:

1. Product not installed.
2. Product installed but not currently running/available.
3. Product available and its integration endpoint responding.

A registry should help with discovery and configuration; it should not become a shared writable database for every product.

## API design principles

- Version public contracts explicitly.
- Keep transport and domain data models separate where useful.
- Make errors structured and actionable.
- Validate inputs at boundaries.
- Avoid hidden coupling to a particular GUI.
- Keep backward compatibility in mind and document breaking changes.
- Prefer local IPC where practical for local product-to-product communication; a localhost HTTP API may remain appropriate depending on the existing implementation and platform needs.
- Never assume that a service is safe merely because it listens on localhost.

## Suggested API areas

Exact endpoints are intentionally not specified yet; inspect the existing `oae-api` code first.

- Activity import and listing.
- Planned workout validation and management.
- Analytics job/request and result retrieval.
- Data export and deletion.
- Integration discovery/status.
- Health/diagnostics and version information.

## Ownership

- Olympus owns Olympus-specific behavior and data.
- OAE owns its analytics state and OAE-specific records.
- DTIR provides a shared contract/model; it should not imply a shared database.
- Each product exposes supported data through documented APIs or explicit file exchange.

## Import adapters

Keep FIT, GPX, Strava, manual entry, and Olympus integrations behind adapters that normalize into DTIR. Do not make the analytics engine directly dependent on a specific external provider.

## Draconis directory layout

A shared Draconis home was discussed to ease integration. Prefer a small shared registry/configuration area, with product-owned data kept separate.

Platform conventions to consider:

- Windows: `%LOCALAPPDATA%\Draconis\`
- macOS: `~/Library/Application Support/Draconis/`
- Linux: XDG directories, such as `$XDG_CONFIG_HOME/draconis/` and `$XDG_DATA_HOME/draconis/`

A single `~/.draconis/` root may be convenient during early development, but a path-resolution library should centralize this choice so product code does not hardcode paths everywhere.

## Open API decisions

- REST/HTTP, local IPC, or a combination?
- How should API compatibility be negotiated?
- How are local endpoints authenticated or protected?
- Which data is exchanged by API versus explicit file import/export?
- How does the registry represent versions and availability?
