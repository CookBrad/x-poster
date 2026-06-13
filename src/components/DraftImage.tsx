import { useEffect, useState } from 'react'
import type { Draft } from '../lib/db'
import { getDraftDisplayImage, resolveDraftImage } from '../lib/draftImage'

export interface DraftImageProps {
  draft: Draft
  className?: string
  onResolved?: (updated: Draft) => void
}

export function DraftImage({ draft, className, onResolved }: DraftImageProps) {
  const [imageUrl, setImageUrl] = useState<string | null>(() => getDraftDisplayImage(draft))
  const [loading, setLoading] = useState(() => !getDraftDisplayImage(draft))

  useEffect(() => {
    const immediate = getDraftDisplayImage(draft)
    if (immediate) {
      setImageUrl(immediate)
      setLoading(false)
      return
    }

    let cancelled = false
    setLoading(true)

    void resolveDraftImage(draft)
      .then((updated) => {
        if (cancelled) return
        const url = getDraftDisplayImage(updated)
        if (url) setImageUrl(url)
        onResolved?.(updated)
      })
      .catch(() => {
        /* preview unavailable */
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [draft.id, draft.image_url, draft.sources_json])

  if (loading) {
    return (
      <div
        className={`skeleton h-40 w-full rounded-lg mt-2 ${className ?? ''}`}
        data-testid={`draft-image-loading-${draft.id}`}
      />
    )
  }

  if (!imageUrl) {
    return null
  }

  return (
    <img
      src={imageUrl}
      alt="Draft post image"
      className={`mt-2 rounded-lg max-h-48 w-full object-cover ${className ?? ''}`}
      data-testid={`draft-image-${draft.id}`}
      onError={(e) => {
        ;(e.target as HTMLImageElement).style.display = 'none'
      }}
    />
  )
}