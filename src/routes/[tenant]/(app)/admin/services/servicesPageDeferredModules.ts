import type { getAvailableRouterNameSuggestions as getAvailableRouterNameSuggestionsValue } from '$lib/utils/packageRouterMeta';
import type {
  getPackageRouterMappingErrorFallback as getPackageRouterMappingErrorFallbackValue,
  getPackageRouterMappingReferenceError as getPackageRouterMappingReferenceErrorValue,
} from '$lib/utils/packageRouterMapping';

type AsyncModuleLoader<T> = () => Promise<T>;

export type ServicesRouterMappingHelpers = {
  getAvailableRouterNameSuggestions: typeof getAvailableRouterNameSuggestionsValue;
  getPackageRouterMappingReferenceError: typeof getPackageRouterMappingReferenceErrorValue;
  getPackageRouterMappingErrorFallback: typeof getPackageRouterMappingErrorFallbackValue;
};

function createCachedLoader<T>(loader: AsyncModuleLoader<T>): AsyncModuleLoader<T> {
  let cached: Promise<T> | null = null;

  return () => {
    if (!cached) {
      cached = loader();
    }
    return cached;
  };
}

export const loadServicesRouterMappingHelpers =
  createCachedLoader<ServicesRouterMappingHelpers>(async () => {
    const [metaModule, mappingModule] = await Promise.all([
      import('$lib/utils/packageRouterMeta'),
      import('$lib/utils/packageRouterMapping'),
    ]);

    return {
      getAvailableRouterNameSuggestions: metaModule.getAvailableRouterNameSuggestions,
      getPackageRouterMappingReferenceError: mappingModule.getPackageRouterMappingReferenceError,
      getPackageRouterMappingErrorFallback: mappingModule.getPackageRouterMappingErrorFallback,
    };
  });
