export function PagingFeedback({ loading, error, onRetry }: { loading?: boolean; error?: string | null; onRetry?: () => void }) {
  return error ? <div role="alert">Couldn’t load music. {error} <button type="button" onClick={onRetry}>Retry loading</button></div> : loading ? <p role="status">Loading music…</p> : null;
}
