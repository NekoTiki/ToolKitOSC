import ipaddr, { type IPv6 } from 'ipaddr.js'

export function getStableIp(ip: string): string {
  if (ip === 'unknown') return 'unknown'

  const addr = ipaddr.parse(ip)

  if (addr.kind() === 'ipv6') {
    return (addr as IPv6).parts
      .slice(0, 4)
      .map((part) => part.toString(16))
      .join(':')
  } else return addr.toString()
}
