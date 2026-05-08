type SearchViewParams = {
  query: string;
  loading: boolean;
  hasSearched: boolean;
  resultCount: number;
};

type SearchViewState =
  | { kind: 'idle'; message: string }
  | { kind: 'loading'; message: string }
  | { kind: 'empty'; message: string }
  | { kind: 'results'; message: string };

export function getCustomerSearchViewState(params: SearchViewParams): SearchViewState {
  const query = params.query.trim();

  if (query.length < 2 || !params.hasSearched) {
    return {
      kind: 'idle',
      message: 'Search with at least 2 characters to load existing customers.',
    };
  }

  if (params.loading) {
    return {
      kind: 'loading',
      message: 'Searching customers...',
    };
  }

  if (params.resultCount <= 0) {
    return {
      kind: 'empty',
      message: `No customers found for "${query}".`,
    };
  }

  return {
    kind: 'results',
    message: '',
  };
}

