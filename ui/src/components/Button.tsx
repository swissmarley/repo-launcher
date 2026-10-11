import type { ButtonHTMLAttributes } from 'react'

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger'

const variants: Record<Variant, string> = {
  primary: 'bg-accent text-accent-fg hover:bg-accent-hover',
  secondary: 'bg-surface text-fg border border-border hover:bg-sunken',
  ghost: 'text-muted hover:text-fg hover:bg-sunken',
  danger: 'bg-danger text-white hover:opacity-90',
}

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant
  size?: 'md' | 'lg'
}

export function Button({ variant = 'secondary', size = 'md', className = '', ...rest }: Props) {
  const sizing = size === 'lg' ? 'min-h-11 px-5 text-base' : 'min-h-9 px-3.5 text-sm'
  return (
    <button
      type="button"
      className={`inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${sizing} ${variants[variant]} ${className}`}
      {...rest}
    />
  )
}
