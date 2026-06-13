import { useState } from 'react'

interface SecretInputProps {
  value: string
  onChange: (value: string) => void
  placeholder?: string
  inputTestId?: string
  toggleTestId?: string
  inputClassName?: string
}

export function SecretInput({
  value,
  onChange,
  placeholder,
  inputTestId,
  toggleTestId,
  inputClassName = 'input input-bordered input-sm font-mono',
}: SecretInputProps) {
  const [visible, setVisible] = useState(false)

  return (
    <div className="join w-full">
      <input
        type={visible ? 'text' : 'password'}
        className={`${inputClassName} join-item flex-1`}
        placeholder={placeholder}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        data-testid={inputTestId}
        autoComplete="off"
      />
      <button
        type="button"
        className="btn btn-ghost join-item border border-l-0 btn-sm min-w-[4.5rem]"
        onClick={() => setVisible((current) => !current)}
        aria-label={visible ? 'Hide value' : 'Show value'}
        data-testid={toggleTestId}
      >
        {visible ? 'Hide' : 'Show'}
      </button>
    </div>
  )
}