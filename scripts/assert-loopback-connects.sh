#!/usr/bin/env bash
set -euo pipefail

trace_file="${1:-}"
if [[ -z "$trace_file" || ! -f "$trace_file" ]]; then
  echo "Usage: $0 /path/to/strace-connect.log" >&2
  exit 2
fi

network_connects=$(grep -E 'connect\(.*sa_family=AF_INET(6)?' "$trace_file" || true)
if [[ -z "$network_connects" ]]; then
  echo "No IPv4 or IPv6 connection attempts were observed."
  exit 0
fi

unexpected=$(printf '%s\n' "$network_connects" | grep -Ev '127\.|"::1"|"::ffff:127\.' || true)
if [[ -n "$unexpected" ]]; then
  echo "Unexpected non-loopback connection attempts:" >&2
  printf '%s\n' "$unexpected" >&2
  exit 1
fi

echo "All observed network connection attempts stayed on loopback."
