# Fork changes

This is a fork of [rustables](https://gitlab.com/rustwall/rustables) 0.8.7,
licensed **GPL-3.0-or-later**, same as upstream. Per GPL-3.0 §5(a), modified
files carry a dated change notice.

It exists because upstream 0.8.7 cannot manage large, interval- and
timeout-based nftables sets over netlink: its `SetElement` carries only a key
(no range or per-element TTL), and `Batch::send()` cannot push a transaction
larger than one datagram. The changes below address those limitations.

Baseline: upstream v0.8.7 (tag `v0.8.7`, commit 9670e1c).

## Planned / applied changes

- [x] `SetElement`: add `key_end` (NFTA_SET_ELEM_KEY_END), `flags`
      (NFTA_SET_ELEM_FLAGS), and `timeout`/`expiration` (NFTA_SET_ELEM_TIMEOUT)
      so `interval,timeout` sets can be managed. `SetBuilder::add_interval()`
      emits the rbtree **half-open two-element** form (start boundary + end
      boundary flagged NFT_SET_ELEM_INTERVAL_END at `broadcast+1`); `add_with_timeout()`
      adds a timed host element. Verified against a 6.x kernel with ranges + TTLs.
      NB: `KEY_END` is concat/pipapo-only — rbtree interval sets reject it
      (EINVAL), hence the two-element representation.
- [x] `Batch::send()`: large atomic transactions (hundreds of thousands of set
      elements) failed with EMSGSIZE because a netfilter batch must reach the
      kernel as one datagram and `netlink_sendmsg()` rejects datagrams larger
      than `sk_sndbuf - 32`. Raise the send buffer to fit the batch via
      `SndBufForce` (bypasses net.core.wmem_max; needs CAP_NET_ADMIN, which
      ruleset mutation already requires). A ~8MB / 400k-element single-transaction
      flush+refill now commits atomically. Verified against a 6.x kernel.
- [x] ~~Expose the recv/ack path for an external chunked sender~~ — not needed:
      the fix lives inside `Batch::send()`, which already has crate-internal recv
      access. No public API change required.
- [x] `Hook`: add a `dev` field (NFTA_HOOK_DEV) plus a `Hook::netdev_ingress(dev,
      priority)` constructor, so netdev-family chains can bind to a device.
      `HookClass` only covers the NF_INET_* hooks, so the NF_NETDEV_INGRESS
      hooknum (0) is set directly; `priority` is the signed NFTA_HOOK_PRIORITY
      bit-pattern (e.g. -500). Verified: full netdev/ingress table + chain + sets
      + `ip saddr @set drop` rules install atomically on a 6.x kernel.
