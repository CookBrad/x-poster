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
export const DRAFT_STYLE_STORAGE_KEY = 'draft_generation_style'

export const DRAFT_STYLE = {
  insight: 'insight',
  informative: 'informative',
  funny: 'funny',
  witty: 'witty',
  meme: 'meme',
} as const

export type DraftStyle = (typeof DRAFT_STYLE)[keyof typeof DRAFT_STYLE]

export const DRAFT_STYLE_OPTIONS: { value: DraftStyle; label: string }[] = [
  { value: DRAFT_STYLE.insight, label: 'Insight' },
  { value: DRAFT_STYLE.informative, label: 'Informative' },
  { value: DRAFT_STYLE.funny, label: 'Funny' },
  { value: DRAFT_STYLE.witty, label: 'Witty' },
  { value: DRAFT_STYLE.meme, label: 'Meme' },
]

export const DEFAULT_DRAFT_STYLE: DraftStyle = DRAFT_STYLE.insight

export function isDraftStyle(value: string): value is DraftStyle {
  return (Object.values(DRAFT_STYLE) as string[]).includes(value)
}

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