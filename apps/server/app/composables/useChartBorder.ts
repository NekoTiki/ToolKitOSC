// Every bar segment and arc slice across the dashboard charts gets the same flat, outlined look -
// a white border separating it from whatever's next to it or behind it. Chart.js's own ArcElement
// default already draws this for doughnuts (elements.arc.borderColor is '#fff' + borderWidth 2);
// this makes that explicit and applies the same treatment to Bar datasets too (BarElement's default
// borderWidth is 0), so bars and doughnuts read as the same visual family instead of one looking
// outlined and the other flat.
export function useChartBorder(): { borderColor: string; borderWidth: number } {
  return { borderColor: tailwindColor('white'), borderWidth: 2 }
}
