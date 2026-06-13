export const DRAFT_STATUS = {
  pending: 'pending',
  posted: 'posted',
  skipped: 'skipped',
} as const

export type DraftStatus = (typeof DRAFT_STATUS)[keyof typeof DRAFT_STATUS]

export const SETTING_KEYS = {
  xaiApiKey: 'xai_api_key',
  grokModel: 'grok_model',
  xConsumerKey: 'x_consumer_key',
  xConsumerSecret: 'x_consumer_secret',
  xAccessToken: 'x_access_token',
  xAccessTokenSecret: 'x_access_token_secret',
} as const

export const DEFAULT_GROK_MODEL = 'grok-4.3'
export const MIN_DRAFT_GENERATION_COUNT = 1
export const DEFAULT_DRAFT_GENERATION_COUNT = 3
export const MAX_DRAFT_GENERATION_COUNT = 10
export const DRAFT_COUNT_STORAGE_KEY = 'draft_generation_count'

export function clampDraftCount(count: number): number {
  return Math.min(
    MAX_DRAFT_GENERATION_COUNT,
    Math.max(MIN_DRAFT_GENERATION_COUNT, Math.round(count))
  )
}
export const SIMULATED_POST_ID_PREFIX = 'sim_'
export const X_STATUS_URL_BASE = 'https://x.com/i/web/status/'

export const RESEARCH_SOURCE_TYPE = {
  rss: 'rss',
  xGrok: 'x_grok',
} as const

export const RESEARCH_MODE = {
  rss: 'rss',
  x: 'x',
  both: 'both',
} as const

export type ResearchMode = (typeof RESEARCH_MODE)[keyof typeof RESEARCH_MODE]

export const RESEARCH_MODE_OPTIONS: { value: ResearchMode; label: string }[] = [
  { value: RESEARCH_MODE.rss, label: 'RSS' },
  { value: RESEARCH_MODE.x, label: 'X (Grok)' },
  { value: RESEARCH_MODE.both, label: 'Both' },
]