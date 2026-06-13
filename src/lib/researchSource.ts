import type { ResearchSource } from './db'

export function isResearchSourceUsed(source: Pick<ResearchSource, 'used_at'>): boolean {
  return Boolean(source.used_at)
}

export function countUnusedResearchSources(sources: ResearchSource[]): number {
  return sources.filter((source) => !isResearchSourceUsed(source)).length
}