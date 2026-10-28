/**
 * Cross-platform HTTP fetcher for downloading filter lists.
 * Uses the global fetch() API (Node 18+, browsers, Deno, Bun).
 */
export async function fetchFilterList(
  url: string,
  customFetch?: typeof globalThis.fetch,
): Promise<string> {
  const fetchFn = customFetch ?? globalThis.fetch

  const response = await fetchFn(url, {
    headers: {
      'Accept': 'text/plain',
      'Accept-Encoding': 'gzip, deflate',
    },
  })

  if (!response.ok) {
    throw new Error(`Failed to fetch filter list from ${url}: ${response.status} ${response.statusText}`)
  }

  return response.text()
}
