export type FocusMode = 'timer' | 'stopwatch'
export type RunningMode = FocusMode | 'break'

export interface FocusTask {
  id: number
  title: string
  mode: FocusMode
  duration: number
  breakDuration: number
  tags: string[]
  isCompleted: boolean
  createdAt: string
  updatedAt: string
}

export interface TaskDraft {
  title: string
  mode: FocusMode
  duration: number
  breakDuration: number
  tags: string[]
}
