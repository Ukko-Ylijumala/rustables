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

- [ ] `SetElement`: add `key_end` (NFTA_SET_ELEM_KEY_END) for interval sets and
      `timeout`/`expiration` (NFTA_SET_ELEM_TIMEOUT) for per-element TTLs, so
      `interval,timeout` sets can be managed.
- [ ] `Batch`: chunked `sendmsg` at netlink-message boundaries within a single
      BATCH_BEGIN/END, so a transaction larger than ~256KB sends without
      EMSGSIZE while remaining atomic (currently `send()` pushes one datagram).
- [ ] Make the recv/ack path reusable (expose `recv_and_process` /
      `socket_close_wrapper`, or add a `send_large()` on `Batch`) so a chunked
      sender doesn't need crate-internal access.
