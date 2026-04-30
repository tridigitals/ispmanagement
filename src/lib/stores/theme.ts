import { writable } from 'svelte/store';

type Theme = 'dark';

function applyDarkTheme() {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('theme', 'dark');
  }
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', 'dark');
  }
}

function createThemeStore() {
  const { subscribe, set } = writable<Theme>('dark');

  return {
    subscribe,
    set: (_value: Theme) => {
      applyDarkTheme();
      set('dark');
    },
    toggle: () => {
      applyDarkTheme();
      set('dark');
    },
    init: () => {
      applyDarkTheme();
      set('dark');
    },
  };
}

export const theme = createThemeStore();
