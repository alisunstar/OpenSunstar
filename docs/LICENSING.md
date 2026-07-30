# OpenSunstar licensing and product boundary

## Public client

The source code in this repository is the OpenSunstar public client and is licensed under the [Apache License 2.0](../LICENSE), unless a file or bundled third-party component states otherwise. This includes the desktop application, the `os` CLI, local configuration and workflow features, and client-side integrations that communicate with optional services.

Contributions intentionally submitted to this repository are licensed under Apache-2.0 in accordance with section 5 of the license, unless an explicit written agreement says otherwise.

## Private commercial control plane

The hosted OpenSunstar commercial control plane is separate proprietary software. Account registration and authentication, subscriptions and billing, multi-tenant team services, and cloud operations are maintained in a separate private repository. They are not included in this repository and are not granted under this repository's Apache-2.0 license.

Public client code that displays or calls control-plane capabilities remains part of this Apache-2.0 repository. Access to a hosted service may additionally be governed by its own terms of service and subscription terms.

## License transition

OpenSunstar releases through v1.1.9 were published under the MIT License. Those releases remain available under the license that accompanied them. Beginning with v1.2.0, the public client is published under Apache-2.0. Changing the license for new releases does not revoke rights already granted for earlier releases.

## Third-party software and trademarks

Bundled dependencies, skills, design-system references, and other third-party materials retain their own licenses and notices. Apache-2.0 does not grant trademark rights beyond the limited uses described in section 6 of the license.
