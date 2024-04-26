# Zephyr Client-Side Tooling

Migration still in progress!

We've decided to begin deploying to prod withouth open-sourcing the VM for everyone yet
for security reasons. Though the product is ready to attempt a BETA release on production,
so we've decided to split our monorepo (host, common, sdk, macros) into (host, macros) and (sdk, common, macros). 

> Note: most of the work right now is focusing on the host-side, as a result
> the crates in this repo are partially incomplete and still need to be polished.

# Zephyr SDK
This repo contains the reference implementation for the Zephyr rust SDK. The SDK is
a client for the host functions linked in the Zephyr VM.

### Documentation

Documentation can be found in the [Mercury docs](https://docs.mercurydata.app/).

### Examples
Examples can be found in the [zephyr-examples](https://github.com/xycloo/zephyr-examples/) repo.
