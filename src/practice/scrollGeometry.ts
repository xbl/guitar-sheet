const DEFAULT_EPS_PX = 2

export function maxScrollTop(scrollHeight: number, clientHeight: number): number {
  return Math.max(0, scrollHeight - clientHeight)
}

export function isAtScrollBottom(
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
  epsilonPx: number = DEFAULT_EPS_PX,
): boolean {
  const max = maxScrollTop(scrollHeight, clientHeight)
  return scrollTop >= max - epsilonPx
}

export function applyScrollDelta(params: {
  scrollTop: number
  scrollHeight: number
  clientHeight: number
  deltaY: number
  epsilonPx?: number
}): { nextScrollTop: number; reachedBottom: boolean } {
  const max = maxScrollTop(params.scrollHeight, params.clientHeight)
  const next = Math.min(max, params.scrollTop + params.deltaY)
  const eps = params.epsilonPx ?? DEFAULT_EPS_PX
  const reachedBottom = next >= max - eps
  return { nextScrollTop: next, reachedBottom }
}
