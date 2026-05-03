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
  /** 无可滚动距离时不视为「到底」，否则播放滚动会在首帧把练习误判为结束（WKWebView 常见）。 */
  if (max <= 0) {
    return { nextScrollTop: 0, reachedBottom: false }
  }
  const next = Math.min(max, params.scrollTop + params.deltaY)
  const eps = params.epsilonPx ?? DEFAULT_EPS_PX
  const reachedBottom = next >= max - eps
  return { nextScrollTop: next, reachedBottom }
}
