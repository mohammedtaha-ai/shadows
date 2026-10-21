import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { personQuery } from '@/api/person'

export function PersonTheme() {
  const person = useQuery(personQuery)
  const theme = person.data?.theme
  useEffect(() => {
    const root = document.documentElement
    const dark = root.classList.contains('dark')
    const previous = root.getAttribute('data-theme')
    return () => {
      root.classList.toggle('dark', dark)
      if (previous === null) root.removeAttribute('data-theme')
      else root.setAttribute('data-theme', previous)
    }
  }, [])
  useEffect(() => {
    if (theme === undefined) return
    const root = document.documentElement
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const apply = () => {
      const dark = theme === 'dark' || theme === 'system' && media.matches
      root.classList.toggle('dark', dark)
      root.dataset.theme = dark ? 'dark' : 'light'
    }
    apply()
    if (theme !== 'system') return
    media.addEventListener('change', apply)
    return () => media.removeEventListener('change', apply)
  }, [theme])
  return null
}
