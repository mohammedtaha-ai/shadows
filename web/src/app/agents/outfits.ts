import manifest from '@/assets/outfits/manifest.json'

const images = import.meta.glob<string>('../../assets/outfits/*.png', {
  eager: true, query: '?url', import: 'default',
})
export const OUTFITS = manifest.map(outfit => ({
  ...outfit, image: images[`../../assets/outfits/${outfit.id}.png`],
}))
export function outfitOf(id: string | null | undefined) {
  return OUTFITS.find(outfit => outfit.id === id) ?? OUTFITS.find(outfit => outfit.id === 'default')!
}
