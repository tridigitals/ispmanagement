import { writable } from 'svelte/store';

type Theme = 'dark' | 'light';

function createThemeStore() {
    // Get initial theme from localStorage or default to dark
    const initialTheme: Theme = (typeof localStorage !== 'undefined' && localStorage.getItem('theme') as Theme) || 'dark';
    
    const { subscribe, set } = writable<Theme>(initialTheme);

    return {
        subscribe,
        set: (value: Theme) => {
            if (typeof localStorage !== 'undefined') {
                localStorage.setItem('theme', value);
                document.documentElement.setAttribute('data-theme', value);
            }
            set(value);
        },
        toggle: () => {
            let current: Theme = 'dark';
            subscribe(v => current = v)();
            const next = current === 'dark' ? 'light' : 'dark';
            
            if (typeof localStorage !== 'undefined') {
                localStorage.setItem('theme', next);
                document.documentElement.setAttribute('data-theme', next);
            }
            set(next);
        },
        init: () => {
            if (typeof localStorage !== 'undefined') {
                const savedTheme = localStorage.getItem('theme') as Theme || 'dark';
                document.documentElement.setAttribute('data-theme', savedTheme);
                set(savedTheme);
            }
        }
    };
}

export const theme = createThemeStore();
