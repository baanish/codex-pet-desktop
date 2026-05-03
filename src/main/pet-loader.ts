import { existsSync, readdirSync, readFileSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { PetInfo, PetJson } from '../shared/types'

export function loadPets(): PetInfo[] {
  const petsDir = join(homedir(), '.codex/pets')
  if (!existsSync(petsDir)) return []

  const entries = readdirSync(petsDir, { withFileTypes: true })
  const pets: PetInfo[] = []

  for (const entry of entries) {
    if (!entry.isDirectory()) continue

    const petDir = join(petsDir, entry.name)
    const jsonPath = join(petDir, 'pet.json')
    const spritesheetPath = join(petDir, 'spritesheet.webp')

    if (!existsSync(jsonPath) || !existsSync(spritesheetPath)) continue

    try {
      const raw = readFileSync(jsonPath, 'utf-8')
      const json: PetJson = JSON.parse(raw)
      pets.push({
        ...json,
        directory: petDir,
        spritesheetAbsPath: spritesheetPath
      })
    } catch {
      // Skip malformed pet.json
    }
  }

  return pets
}
