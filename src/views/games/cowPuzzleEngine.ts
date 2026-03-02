export const COW_PUZZLE_REFRESH_EVENT = 'cow-puzzle://new-game'

export type DifficultyKey = 'casual' | 'standard' | 'expert'

export interface Position {
  row: number
  col: number
}

export interface LogicRating {
  score: number
  solvedByLogic: boolean
  openingForcedPlacements: number
  forcedPlacements: number
  contradictionEliminations: number
  globalForcedPlacements: number
  propagationWaves: number
  averageRegionSpan: number
  rowDiversity: number
  colDiversity: number
}

export interface PuzzleDefinition {
  size: number
  regions: number[][]
  palette: string[]
  solutionCols: number[]
  cows: Position[]
  seed: number
  difficulty: DifficultyKey
  difficultyLabel: string
  rating: LogicRating
}

interface PuzzleTemplate {
  size: number
  regions: number[][]
  solutionCols: number[]
  rating: LogicRating
}

const BASE_PALETTE = [
  '#6F8FBF',
  '#A9D18E',
  '#8B79C9',
  '#E2BA48',
  '#F2A363',
  '#77BFDE',
  '#C76B95',
  '#8EAAC8',
  '#65C7B3',
  '#E2867B',
]

const TEMPLATE_BANK: Record<DifficultyKey, { label: string; templates: PuzzleTemplate[] }> = {
  casual: {
    label: '轻松',
    templates: [
      {
        size: 6,
        solutionCols: [2, 0, 3, 5, 1, 4],
        regions: [
          [1, 1, 0, 2, 2, 2],
          [1, 1, 2, 2, 3, 3],
          [1, 1, 2, 2, 3, 3],
          [4, 4, 5, 5, 3, 3],
          [4, 4, 5, 5, 5, 5],
          [4, 4, 4, 5, 5, 5],
        ],
        rating: {
          score: 22,
          solvedByLogic: true,
          openingForcedPlacements: 3,
          forcedPlacements: 5,
          contradictionEliminations: 1,
          globalForcedPlacements: 1,
          propagationWaves: 2,
          averageRegionSpan: 5.33,
          rowDiversity: 2.83,
          colDiversity: 3,
        },
      },
      {
        size: 6,
        solutionCols: [1, 3, 0, 4, 2, 5],
        regions: [
          [2, 0, 1, 5, 5, 5],
          [2, 2, 1, 1, 3, 5],
          [2, 2, 2, 1, 3, 5],
          [2, 4, 4, 3, 3, 5],
          [2, 4, 4, 5, 5, 5],
          [2, 2, 4, 5, 5, 5],
        ],
        rating: {
          score: 24,
          solvedByLogic: true,
          openingForcedPlacements: 2,
          forcedPlacements: 5,
          contradictionEliminations: 1,
          globalForcedPlacements: 1,
          propagationWaves: 2,
          averageRegionSpan: 5.83,
          rowDiversity: 3.17,
          colDiversity: 3.33,
        },
      },
      {
        size: 6,
        solutionCols: [0, 2, 5, 3, 1, 4],
        regions: [
          [0, 0, 1, 1, 1, 2],
          [1, 1, 1, 3, 2, 2],
          [4, 4, 3, 3, 2, 2],
          [4, 4, 3, 3, 5, 2],
          [4, 4, 3, 5, 5, 5],
          [4, 5, 5, 5, 5, 5],
        ],
        rating: {
          score: 26,
          solvedByLogic: true,
          openingForcedPlacements: 2,
          forcedPlacements: 6,
          contradictionEliminations: 2,
          globalForcedPlacements: 1,
          propagationWaves: 3,
          averageRegionSpan: 6,
          rowDiversity: 3,
          colDiversity: 3.17,
        },
      },
    ],
  },
  standard: {
    label: '标准',
    templates: [
      {
        size: 8,
        solutionCols: [0, 3, 1, 6, 2, 5, 7, 4],
        regions: [
          [0, 0, 0, 0, 1, 1, 2, 2],
          [3, 0, 0, 1, 1, 1, 2, 2],
          [3, 3, 3, 4, 1, 2, 2, 2],
          [3, 3, 3, 4, 4, 5, 2, 2],
          [3, 3, 4, 4, 4, 5, 5, 6],
          [3, 7, 7, 4, 5, 5, 5, 6],
          [7, 7, 7, 7, 5, 5, 6, 6],
          [7, 7, 7, 7, 7, 5, 6, 6],
        ],
        rating: {
          score: 35,
          solvedByLogic: true,
          openingForcedPlacements: 2,
          forcedPlacements: 6,
          contradictionEliminations: 2,
          globalForcedPlacements: 1,
          propagationWaves: 3,
          averageRegionSpan: 6.75,
          rowDiversity: 3.75,
          colDiversity: 4.12,
        },
      },
      {
        size: 8,
        solutionCols: [0, 4, 1, 3, 6, 2, 5, 7],
        regions: [
          [0, 0, 0, 0, 0, 1, 1, 1],
          [2, 0, 0, 0, 1, 1, 1, 1],
          [2, 2, 0, 4, 1, 1, 3, 3],
          [2, 2, 4, 4, 4, 3, 3, 3],
          [2, 6, 4, 4, 4, 4, 3, 5],
          [6, 6, 6, 4, 7, 5, 5, 5],
          [6, 6, 6, 6, 7, 7, 5, 5],
          [6, 6, 6, 7, 7, 7, 7, 5],
        ],
        rating: {
          score: 38,
          solvedByLogic: true,
          openingForcedPlacements: 2,
          forcedPlacements: 6,
          contradictionEliminations: 2,
          globalForcedPlacements: 2,
          propagationWaves: 3,
          averageRegionSpan: 6.63,
          rowDiversity: 4,
          colDiversity: 4.12,
        },
      },
    ],
  },
  expert: {
    label: '刁钻',
    templates: [
      {
        size: 8,
        solutionCols: [0, 3, 1, 4, 6, 2, 5, 7],
        regions: [
          [0, 0, 0, 0, 1, 1, 1, 1],
          [2, 0, 0, 1, 1, 1, 1, 1],
          [2, 2, 2, 4, 1, 1, 3, 3],
          [2, 2, 4, 4, 4, 3, 3, 3],
          [2, 4, 4, 4, 4, 4, 3, 5],
          [6, 6, 6, 4, 7, 5, 5, 5],
          [6, 6, 6, 6, 7, 7, 5, 5],
          [6, 6, 6, 7, 7, 7, 7, 5],
        ],
        rating: {
          score: 49,
          solvedByLogic: true,
          openingForcedPlacements: 1,
          forcedPlacements: 7,
          contradictionEliminations: 3,
          globalForcedPlacements: 2,
          propagationWaves: 4,
          averageRegionSpan: 6.88,
          rowDiversity: 4.12,
          colDiversity: 4.25,
        },
      },
      {
        size: 8,
        solutionCols: [0, 4, 1, 3, 7, 5, 2, 6],
        regions: [
          [0, 0, 0, 0, 0, 1, 1, 2],
          [3, 0, 0, 0, 1, 1, 1, 2],
          [3, 3, 0, 4, 1, 1, 2, 2],
          [3, 3, 3, 4, 4, 1, 2, 2],
          [3, 6, 4, 4, 4, 4, 5, 2],
          [6, 6, 6, 4, 7, 5, 5, 5],
          [6, 6, 6, 6, 7, 7, 5, 5],
          [6, 6, 6, 7, 7, 7, 7, 7],
        ],
        rating: {
          score: 53,
          solvedByLogic: true,
          openingForcedPlacements: 1,
          forcedPlacements: 7,
          contradictionEliminations: 4,
          globalForcedPlacements: 2,
          propagationWaves: 4,
          averageRegionSpan: 7,
          rowDiversity: 4.12,
          colDiversity: 4.37,
        },
      },
    ],
  },
}

export const cowPuzzleDifficultyOptions = Object.entries(TEMPLATE_BANK).map(([value, entry]) => ({
  value: value as DifficultyKey,
  label: entry.label,
}))

class SeededRandom {
  private value: number

  constructor(seed: number) {
    this.value = seed >>> 0
  }

  next() {
    this.value = (1664525 * this.value + 1013904223) >>> 0
    return this.value / 0x100000000
  }

  int(maxExclusive: number) {
    return Math.floor(this.next() * maxExclusive)
  }
}

function createRandomSeed() {
  const cryptoObj = globalThis.crypto
  if (cryptoObj?.getRandomValues) {
    const seed = new Uint32Array(1)
    cryptoObj.getRandomValues(seed)
    return seed[0]
  }
  return Math.floor(Math.random() * 0xffffffff)
}

function cloneMatrix(matrix: number[][]) {
  return matrix.map((row) => [...row])
}

function cloneRating(rating: LogicRating): LogicRating {
  return { ...rating }
}

function shuffle<T>(list: T[], rng: SeededRandom) {
  const result = [...list]
  for (let index = result.length - 1; index > 0; index--) {
    const swapIndex = rng.int(index + 1)
    ;[result[index], result[swapIndex]] = [result[swapIndex], result[index]]
  }
  return result
}

function mirrorHorizontal(template: PuzzleTemplate): PuzzleTemplate {
  const size = template.size
  return {
    ...template,
    regions: template.regions.map((row) => [...row].reverse()),
    solutionCols: template.solutionCols.map((col) => size - 1 - col),
  }
}

function mirrorVertical(template: PuzzleTemplate): PuzzleTemplate {
  return {
    ...template,
    regions: [...template.regions].reverse().map((row) => [...row]),
    solutionCols: [...template.solutionCols].reverse(),
  }
}

function transpose(template: PuzzleTemplate): PuzzleTemplate {
  const size = template.size
  const regions = Array.from({ length: size }, () => Array(size).fill(0))
  const solutionCols = Array(size).fill(0)

  for (let row = 0; row < size; row++) {
    for (let col = 0; col < size; col++) {
      regions[col][row] = template.regions[row][col]
    }
    solutionCols[template.solutionCols[row]] = row
  }

  return {
    ...template,
    regions,
    solutionCols,
  }
}

function relabelRegions(template: PuzzleTemplate, rng: SeededRandom): PuzzleTemplate {
  const mapping = shuffle(Array.from({ length: template.size }, (_, index) => index), rng)
  return {
    ...template,
    regions: template.regions.map((row) => row.map((regionId) => mapping[regionId])),
  }
}

function transformTemplate(template: PuzzleTemplate, rng: SeededRandom) {
  let current: PuzzleTemplate = {
    ...template,
    regions: cloneMatrix(template.regions),
    solutionCols: [...template.solutionCols],
    rating: cloneRating(template.rating),
  }

  if (rng.int(2) === 1) current = mirrorHorizontal(current)
  if (rng.int(2) === 1) current = mirrorVertical(current)
  if (rng.int(2) === 1) current = transpose(current)

  return relabelRegions(current, rng)
}

function buildPalette(size: number, rng: SeededRandom) {
  return shuffle(BASE_PALETTE, rng).slice(0, size)
}

export function generateCowPuzzle(difficulty: DifficultyKey): PuzzleDefinition {
  const seed = createRandomSeed()
  const rng = new SeededRandom(seed)
  const difficultyEntry = TEMPLATE_BANK[difficulty]
  const template = difficultyEntry.templates[rng.int(difficultyEntry.templates.length)]
  const transformed = transformTemplate(template, rng)

  return {
    size: transformed.size,
    regions: transformed.regions,
    palette: buildPalette(transformed.size, rng),
    solutionCols: transformed.solutionCols,
    cows: transformed.solutionCols.map((col, row) => ({ row, col })),
    seed,
    difficulty,
    difficultyLabel: difficultyEntry.label,
    rating: transformed.rating,
  }
}
