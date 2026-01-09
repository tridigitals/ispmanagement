import { writable } from 'svelte/store';
import { api } from '$lib/api/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Image } from '@tauri-apps/api/image';

function createLogoStore() {
    const { subscribe, set } = writable<string | null>(null);

    const updateWindowIcon = async (base64String: string) => {
        console.log(`[LogoStore] updateWindowIcon called. Data length: ${base64String.length}`);
        
        if (typeof window === 'undefined') return;
        
        // Skip if not running in Tauri
        // @ts-ignore
        if (!window.__TAURI_INTERNALS__) return;
        
        try {
            // Strip prefix if present (data:image/png;base64,...)
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
            // No-op, waiting for refresh call
        },
                refresh: async (token?: string) => {
                    console.log(`[LogoStore] Refreshing logo. Token provided: ${!!token}`);
                    try {
                        const logo = await api.settings.getLogo(token);
                        if (logo) {
                            console.log(`[LogoStore] Logo received from backend. Data prefix: ${logo.substring(0, 30)}...`);
                            set(logo);
                            updateWindowIcon(logo);
                        } else {
                            console.log(`[LogoStore] No logo returned from backend (null response)`);
                        }
                    } catch (err) {
                        console.error("[LogoStore] Failed to load logo:", err);
                    }
                }
        
    };
}

export const appLogo = createLogoStore();
