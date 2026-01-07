import { writable } from 'svelte/store';
import { api } from '$lib/api/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Image } from '@tauri-apps/api/image';

function createLogoStore() {
    const { subscribe, set } = writable<string | null>(null);

    const updateWindowIcon = async (base64String: string) => {
        if (typeof window === 'undefined') return;
        
        try {
            // Strip prefix if present
            const base64 = base64String.split(',')[1] || base64String;
            
            // Decode base64 to bytes
            const binaryString = atob(base64);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }

            // Create Tauri Image and set icon
            const image = await Image.fromBytes(bytes);
            await getCurrentWindow().setIcon(image);
        } catch (err) {
            console.warn("Failed to set window icon:", err);
        }
    };

    return {
        subscribe,
        set: (value: string | null) => {
            set(value);
            if (value) updateWindowIcon(value);
        },
        init: async () => {
            try {
                const logo = await api.settings.getLogo();
                if (logo) {
                    set(logo);
                    updateWindowIcon(logo);
                }
            } catch (err) {
                console.error("Failed to load logo:", err);
            }
        }
    };
}

export const appLogo = createLogoStore();
