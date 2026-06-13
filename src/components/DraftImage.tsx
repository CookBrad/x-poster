import { useEffect, useRef, useState } from 'react'
import type { Draft } from '../lib/db'
import {
  getDisplayableImageUrl,
  getDraftDisplayImage,
  resolveDraftImage,
} from '../lib/draftImage'

export interface DraftImageProps {
  draft: Draft
  className?: string
  onResolved?: (updated: Draft) => void
}

export function DraftImage({ draft, className, onResolved }: DraftImageProps) {
  const [imageUrl, setImageUrl] = useState<string | null>(() => getDraftDisplayImage(draft))
  const [loading, setLoading] = useState(() => !draft.image_url?.trim() && !getDraftDisplayImage(draft))
  const resolveAttemptedRef = useRef(false)

  useEffect(() => {
    resolveAttemptedRef.current = false

    const immediate = getDraftDisplayImage(draft)
    if (draft.image_url?.trim() || immediate) {
      setImageUrl(immediate)
      setLoading(false)
      if (draft.image_url?.trim()) {
        return
      }
    }

    let cancelled = false
    setLoading(true)

    void resolveDraftImage(draft)
      .then((updated) => {
        if (cancelled) return
        const url =
          getDisplayableImageUrl(updated.image_url) ?? getDraftDisplayImage(updated)
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

  const handleImageError = () => {
    if (resolveAttemptedRef.current) {
      setImageUrl(null)
      return
    }

    if (draft.image_url?.trim()) {
      const fallback = getDraftDisplayImage({ ...draft, image_url: null })
      if (fallback && fallback !== imageUrl) {
        resolveAttemptedRef.current = true
        setImageUrl(fallback)
        return
      }
    }

    resolveAttemptedRef.current = true
    setLoading(true)

    void resolveDraftImage(draft)
      .then((updated) => {
        const url =
          getDisplayableImageUrl(updated.image_url) ?? getDraftDisplayImage(updated)
        if (url) {
          setImageUrl(url)
          onResolved?.(updated)
        } else {
          setImageUrl(null)
        }
      })
      .catch(() => {
        setImageUrl(null)
      })
      .finally(() => {
        setLoading(false)
      })
  }

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
      onError={() => {
        handleImageError()
      }}
    />
  )
}