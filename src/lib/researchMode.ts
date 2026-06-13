import { RESEARCH_MODE, type ResearchMode } from './constants'

export function researchModeRequiresXaiKey(mode: ResearchMode): boolean {
  return mode === RESEARCH_MODE.x || mode === RESEARCH_MODE.both
}

export function researchModeLabel(mode: ResearchMode): string {
  switch (mode) {
    case RESEARCH_MODE.rss:
      return 'RSS'
    case RESEARCH_MODE.x:
      return 'X'
    case RESEARCH_MODE.both:
      return 'RSS + X'
  }
}