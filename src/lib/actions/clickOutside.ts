/**
 * Svelte 5 action: Dispatch callback on click outside of node
 * Usage: use:clickOutside={{ callback: () => close() }}
 */
export interface ClickOutsideParams {
  callback: () => void;
}

export function clickOutside(node: HTMLElement, params: ClickOutsideParams) {
  let { callback } = params;

  const handleClick = (event: MouseEvent) => {
    if (node && !node.contains(event.target as Node) && !event.defaultPrevented) {
      callback();
    }
  };

  document.addEventListener('click', handleClick, true);

  return {
    update(newParams: ClickOutsideParams) {
      callback = newParams.callback;
    },
    destroy() {
      document.removeEventListener('click', handleClick, true);
    },
  };
}
