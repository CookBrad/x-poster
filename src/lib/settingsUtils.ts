export function maskSecret(secret: string, visiblePrefix = 7): string {
  if (!secret) {
    return ''
  }
  if (secret.length <= visiblePrefix) {
    return '••••••••'
  }
  return `${secret.slice(0, visiblePrefix)}••••••••`
}

export function countFilledFields(values: Record<string, string>, keys: readonly string[]): number {
  return keys.filter((key) => (values[key] ?? '').trim().length > 0).length
}