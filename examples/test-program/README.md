# Testing Framework Example

This is an example of how you can use the testutils provided by Zephyr to
locally test your program against specific situations and edge cases.

Before you get started below are a few guidelines:
1. Install postgresql (v14 for example).
2. Make sure postgresql is setup so that you can connect to it.
For this you need an existing user and a configured password. We
default to user: postgres and password: postgres.
3. Make sure that you only use testutils as dev dependencies, the binary
won't compile with the testutils feature turned on.
4. When crafting custom ledger transitions, make sure you're using the following
XDR library in the dev dependencies unless you want to manually perform an XDR roundtrip:

```
[dev-dependencies.stellar-xdr]
version = "=20.1.0"
git = "https://github.com/stellar/rs-stellar-xdr"
rev = "44b7e2d4cdf27a3611663e82828de56c5274cba0"
features=["next", "curr", "serde", "base64"]
```
