import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  RESEARCH_MODE,
  RESEARCH_MODE_OPTIONS,
  RESEARCH_SOURCE_TYPE,
  SETTING_KEYS,
  type ResearchMode,
} from '../lib/constants'
import {
  generateDraftFromSource,
  generateDraftsFromLatestResearch,
  getAllHistoricalSources,
  getLatestResearchRun,
  resetResearchData,
  runResearch,
  type ResearchRunWithSources,
} from '../lib/db'
import {
  draftCountOptions,
  loadDraftGenerationCount,
  saveDraftGenerationCount,
} from '../lib/draftGeneration'
import { errorMessage } from '../lib/errors'
import { researchModeLabel, researchModeRequiresXaiKey } from '../lib/researchMode'
import { countUnusedResearchSources, isResearchSourceUsed } from '../lib/researchSource'
import { formatResearchSourceDate, ResearchSourceCard } from './ResearchSourceCard'
import { HistoricalSourcesList } from './HistoricalSourcesList'

type ResearchSubTab = 'current' | 'historical'

export function ResearchTab() {
  const [activeSubTab, setActiveSubTab] = useState<ResearchSubTab>('current')
  const [currentRun, setCurrentRun] = useState<ResearchRunWithSources | null>(null)
  const [hasXaiKey, setHasXaiKey] = useState(false)
  const [historicalResetKey, setHistoricalResetKey] = useState(0)
  const [showResetConfirm, setShowResetConfirm] = useState(false)
  const [draftCount, setDraftCount] = useState(loadDraftGenerationCount)
  const [researchMode, setResearchMode] = useState<ResearchMode>(RESEARCH_MODE.both)
  const [loading, setLoading] = useState(false)
  const [generating, setGenerating] = useState(false)
  const [generatingSourceId, setGeneratingSourceId] = useState<string | null>(null)
  const [pipelinePhase, setPipelinePhase] = useState<'idle' | 'research' | 'generate'>('idle')
  const [isResetting, setIsResetting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [resetSuccess, setResetSuccess] = useState<string | null>(null)
  const [generateSuccess, setGenerateSuccess] = useState<string | null>(null)

  const isPipelineBusy = pipelinePhase !== 'idle'
  const isBusy = loading || generating || isPipelineBusy || generatingSourceId !== null

  const loadLatestRun = async () => {
    try {
      const run = await getLatestResearchRun()
      setCurrentRun(run)
    } catch (loadError: unknown) {
      console.error(loadError)
    }
  }

  const refreshResearchViews = async () => {
    await loadLatestRun()
    setHistoricalResetKey((previous) => previous + 1)
  }

  const handleRunResearch = async () => {
    setLoading(true)
    setError(null)
    setGenerateSuccess(null)

    try {
      const newRun = await runResearch(researchMode)
      setCurrentRun(newRun)
      setActiveSubTab('current')
    } catch (runError: unknown) {
      console.error(runError)
      setError(
        errorMessage(runError, 'Research failed. Check your xAI API key and connection.')
      )
    } finally {
      setLoading(false)
    }
  }

  const handleDraftCountChange = (value: string) => {
    const parsed = Number.parseInt(value, 10)
    if (Number.isNaN(parsed)) {
      return
    }
    setDraftCount(saveDraftGenerationCount(parsed))
  }

  const handleGenerateFromSource = async (sourceId: string) => {
    setGeneratingSourceId(sourceId)
    setError(null)
    setGenerateSuccess(null)

    try {
      const drafts = await generateDraftFromSource(sourceId)
      setGenerateSuccess(
        `Generated ${drafts.length} post(s) from that story. Open the Posts tab to review.`
      )
      await refreshResearchViews()
    } catch (generateError: unknown) {
      console.error(generateError)
      setError(errorMessage(generateError, 'Failed to generate post from this story.'))
    } finally {
      setGeneratingSourceId(null)
    }
  }

  const handleGenerateDrafts = async () => {
    setGenerating(true)
    setError(null)
    setGenerateSuccess(null)

    try {
      const drafts = await generateDraftsFromLatestResearch(draftCount)
      setGenerateSuccess(
        `Generated ${drafts.length} draft(s) with insight-focused prompts. Open the Posts tab to review.`
      )
      await refreshResearchViews()
    } catch (generateError: unknown) {
      console.error(generateError)
      setError(errorMessage(generateError, 'Draft generation failed.'))
    } finally {
      setGenerating(false)
    }
  }

  const handleResearchAndGenerate = async () => {
    setError(null)
    setGenerateSuccess(null)
    let phase: 'research' | 'generate' = 'research'
    setPipelinePhase('research')

    try {
      const newRun = await runResearch(researchMode)
      setCurrentRun(newRun)
      setActiveSubTab('current')

      phase = 'generate'
      setPipelinePhase('generate')
      const drafts = await generateDraftsFromLatestResearch(draftCount)
      setGenerateSuccess(
        `Researched ${newRun.sources.length} source(s) and generated ${drafts.length} draft(s). Open the Posts tab to review.`
      )
      await refreshResearchViews()
    } catch (pipelineError: unknown) {
      console.error(pipelineError)
      const fallback =
        phase === 'generate'
          ? 'Draft generation failed after research completed.'
          : 'Research and generate failed. Check your xAI API key and connection.'
      setError(errorMessage(pipelineError, fallback))
    } finally {
      setPipelinePhase('idle')
    }
  }

  const performResetResearchData = async () => {
    setShowResetConfirm(false)
    setIsResetting(true)
    setError(null)
    setResetSuccess(null)

    try {
      const result = await resetResearchData()
      const remaining = await getAllHistoricalSources()
      if (remaining.length > 0) {
        throw new Error(
          `Reset reported success but ${remaining.length} historical source(s) still remain in the database.`
        )
      }

      setCurrentRun(null)
      setHistoricalResetKey((previous) => previous + 1)
      await loadLatestRun()
      setActiveSubTab('historical')
      setResetSuccess(
        `Deleted ${result.deleted_sources} source(s) and ${result.deleted_runs} research run(s).`
      )
    } catch (resetError: unknown) {
      console.error(resetError)
      setError(`Failed to reset research data: ${errorMessage(resetError)}`)
    } finally {
      setIsResetting(false)
    }
  }

  useEffect(() => {
    void loadLatestRun()

    void (async () => {
      try {
        const key = await invoke<string | null>('get_setting', {
          key: SETTING_KEYS.xaiApiKey,
        })
        setHasXaiKey(!!key)
      } catch {
        setHasXaiKey(false)
      }
    })()
  }, [])

  const rssCount = currentRun?.sources.filter(
    (source) => source.source_type === RESEARCH_SOURCE_TYPE.rss
  ).length
  const grokXCount = currentRun?.sources.filter(
    (source) => source.source_type === RESEARCH_SOURCE_TYPE.xGrok
  ).length

  const researchNeedsXaiKey = researchModeRequiresXaiKey(researchMode)
  const researchDisabled = isBusy || (researchNeedsXaiKey && !hasXaiKey)
  const generateDisabled = isBusy || !hasXaiKey || !currentRun
  const runAllDisabled = isBusy || !hasXaiKey

  const activeResearchLabel = researchModeLabel(researchMode)
  const unusedSourceCount = countUnusedResearchSources(currentRun?.sources ?? [])
  const hasUnusedSources = unusedSourceCount > 0

  return (
    <div>
      <h2 className="text-2xl font-semibold mb-2">Research — Musk Companies Only</h2>
      <p className="mb-4 text-sm opacity-70">
        Focused strictly on <strong>Elon Musk's companies</strong> (Tesla, SpaceX, xAI, Neuralink,
        Boring Company).
        <br />
        General EV news is excluded.
      </p>

      <div className="flex gap-2 mb-4">
        <button
          type="button"
          className={`btn btn-sm ${activeSubTab === 'current' ? 'btn-primary' : 'btn-outline'}`}
          onClick={() => setActiveSubTab('current')}
        >
          Current
        </button>
        <button
          type="button"
          className={`btn btn-sm ${activeSubTab === 'historical' ? 'btn-primary' : 'btn-outline'}`}
          onClick={() => setActiveSubTab('historical')}
        >
          Historical
        </button>
      </div>

      <div className="flex justify-end mb-2">
        <button
          type="button"
          className="btn btn-error btn-sm"
          onClick={() => setShowResetConfirm(true)}
          disabled={isResetting}
          title="Permanently delete all research runs and sources"
        >
          {isResetting ? 'Resetting…' : 'Reset All Research Data'}
        </button>
      </div>

      {showResetConfirm && (
        <dialog className="modal modal-open">
          <div className="modal-box">
            <h3 className="font-bold text-lg text-error">Reset all research data?</h3>
            <p className="py-4 text-sm">
              This permanently deletes every research run and all associated sources (RSS + X/Grok)
              from your local database. This cannot be undone.
            </p>
            <div className="modal-action">
              <button
                type="button"
                className="btn"
                onClick={() => setShowResetConfirm(false)}
                disabled={isResetting}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn btn-error"
                onClick={() => void performResetResearchData()}
                disabled={isResetting}
              >
                {isResetting ? (
                  <>
                    <span className="loading loading-spinner loading-xs" />
                    Deleting…
                  </>
                ) : (
                  'Yes, delete everything'
                )}
              </button>
            </div>
          </div>
          <form method="dialog" className="modal-backdrop">
            <button type="button" onClick={() => setShowResetConfirm(false)}>
              close
            </button>
          </form>
        </dialog>
      )}

      {error && <div className="alert alert-error mb-4">{error}</div>}
      {resetSuccess && <div className="alert alert-success mb-4">{resetSuccess}</div>}
      {generateSuccess && <div className="alert alert-success mb-4">{generateSuccess}</div>}

      {activeSubTab === 'current' && (
        <div>
          <div className="card bg-base-200/60 mb-4">
            <div className="card-body py-4 gap-4">
              <div className="flex flex-wrap items-end gap-4">
                <label className="form-control w-full max-w-[11rem]">
                  <div className="label py-0 pb-1">
                    <span className="label-text font-medium">Research sources</span>
                  </div>
                  <select
                    className="select select-bordered select-sm w-full"
                    value={researchMode}
                    onChange={(event) => setResearchMode(event.target.value as ResearchMode)}
                    disabled={isBusy}
                    data-testid="research-mode"
                  >
                    {RESEARCH_MODE_OPTIONS.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </label>

                <label className="form-control w-full max-w-[11rem]">
                  <div className="label py-0 pb-1">
                    <span className="label-text font-medium">Posts to generate</span>
                  </div>
                  <select
                    className="select select-bordered select-sm w-full"
                    value={draftCount}
                    onChange={(event) => handleDraftCountChange(event.target.value)}
                    disabled={isBusy}
                    data-testid="draft-generation-count"
                  >
                    {draftCountOptions().map((count) => (
                      <option key={count} value={count}>
                        {count} {count === 1 ? 'post' : 'posts'}
                      </option>
                    ))}
                  </select>
                </label>

                <div className="flex flex-wrap gap-2 ml-auto">
                  <button
                    type="button"
                    className="btn btn-sm btn-outline"
                    onClick={() => void handleRunResearch()}
                    disabled={researchDisabled}
                    title={
                      researchNeedsXaiKey && !hasXaiKey
                        ? 'xAI key required for X research'
                        : `Research ${activeResearchLabel} sources`
                    }
                    data-testid="research-button"
                  >
                    {loading ? 'Researching…' : 'Research'}
                  </button>

                  <button
                    type="button"
                    className="btn btn-sm btn-warning"
                    onClick={() => void handleGenerateDrafts()}
                    disabled={generateDisabled || !hasUnusedSources}
                    title={
                      !hasXaiKey
                        ? 'xAI key required'
                        : !currentRun
                          ? 'Run research first'
                          : !hasUnusedSources
                            ? 'All stories have already been used'
                            : `Generate up to ${draftCount} post(s) from unused stories`
                    }
                    data-testid="generate-button"
                  >
                    {generating ? 'Generating…' : 'Generate'}
                  </button>

                  <button
                    type="button"
                    className="btn btn-sm btn-primary"
                    onClick={() => void handleResearchAndGenerate()}
                    disabled={runAllDisabled}
                    title={
                      !hasXaiKey
                        ? 'xAI key required'
                        : `Research ${activeResearchLabel}, then generate up to ${draftCount} unused post(s)`
                    }
                    data-testid="run-all-button"
                  >
                    {isPipelineBusy
                      ? pipelinePhase === 'research'
                        ? 'Researching…'
                        : 'Generating…'
                      : 'Run All'}
                  </button>
                </div>
              </div>
            </div>
          </div>

          {(loading || isPipelineBusy) && (
            <div className="flex flex-col items-center justify-center py-6 mb-4 bg-base-200 rounded-box">
              <span className="loading loading-spinner loading-lg text-primary" />
              <p className="mt-3 font-medium">
                {pipelinePhase === 'generate' || generating
                  ? `Generating ${draftCount} post(s)…`
                  : `Researching ${activeResearchLabel} sources…`}
              </p>
              <p className="text-xs opacity-60 mt-1">
                {pipelinePhase === 'generate' || generating
                  ? 'Writing insight-focused posts from the latest research run'
                  : 'Collecting high-signal Musk company updates'}
              </p>
            </div>
          )}

          {!hasXaiKey && (
            <div className="alert alert-warning alert-sm mb-4">
              X research via Grok is disabled — no xAI API key is saved in Settings.
            </div>
          )}

          {currentRun ? (
            <div>
              <h3 className="text-lg font-semibold mb-1">
                Latest Research Run — {new Date(currentRun.run.run_at).toLocaleString()}
              </h3>

              <p className="text-xs mb-4 opacity-75">
                {currentRun.sources.length} total sources → {rssCount} from RSS, {grokXCount} from X
                (via Grok) • {unusedSourceCount} unused,{' '}
                {currentRun.sources.length - unusedSourceCount} used
              </p>

              {currentRun.sources.every(
                (source) => source.source_type !== RESEARCH_SOURCE_TYPE.xGrok
              ) && (
                <div className="alert alert-warning mb-4 text-sm">
                  No X posts were returned by Grok this time. Make sure your{' '}
                  <strong>xAI API key</strong> is set in Settings. Grok is currently the only source
                  for X content.
                </div>
              )}

              {currentRun.sources.some(
                (source) => source.source_type === RESEARCH_SOURCE_TYPE.xGrok
              ) && (
                <div className="alert alert-info alert-sm mb-4 text-xs">
                  X items are Grok-suggested based on its knowledge. Always verify links and quotes —
                  the model can make mistakes.
                </div>
              )}

              <div className="space-y-3">
                {currentRun.sources.map((source) => (
                  <ResearchSourceCard
                    key={source.id}
                    sourceId={source.id}
                    title={source.title}
                    content={source.content}
                    url={source.url}
                    sourceName={source.source_name}
                    sourceType={source.source_type}
                    dateLabel={formatResearchSourceDate(
                      source.published_at,
                      currentRun.run.run_at
                    )}
                    canGenerate={hasXaiKey}
                    isUsed={isResearchSourceUsed(source)}
                    generating={generatingSourceId === source.id}
                    onGeneratePost={(sourceId) => void handleGenerateFromSource(sourceId)}
                  />
                ))}
              </div>
            </div>
          ) : (
            <div className="alert alert-info">
              No research run yet. Click Research to start.
              <br />
              <span className="text-xs">
                Note: X posts come via Grok — make sure your xAI API key is configured in Settings.
              </span>
            </div>
          )}
        </div>
      )}

      {activeSubTab === 'historical' && (
        <HistoricalSourcesList
          reloadToken={historicalResetKey}
          hasXaiKey={hasXaiKey}
          generatingSourceId={generatingSourceId}
          onGenerateFromSource={(sourceId) => void handleGenerateFromSource(sourceId)}
        />
      )}
    </div>
  )
}