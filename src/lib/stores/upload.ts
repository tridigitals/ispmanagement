import { writable, derived, get } from "svelte/store";
import { toast } from "svelte-sonner";
import { appSettings } from "$lib/stores/settings";

export interface UploadItem {
    id: string;
    file: File;
    progress: number; // 0 - 100
    status: "pending" | "uploading" | "success" | "error";
    error?: string;
}

function createUploadStore() {
    const { subscribe, update, set } = writable<UploadItem[]>([]);

    return {
        subscribe,

        // Start a new upload (Chunked)
        upload: async (file: File, token: string) => {
            const settings = get(appSettings);

            // 1. Client-side Validation: Size
            const maxMb = parseInt(settings.storage_max_file_size_mb || "500");
            const maxBytes = maxMb * 1024 * 1024;

            if (file.size > maxBytes) {
                toast.error(`File is too large! Maximum allowed is ${maxMb} MB.`);
                return;
            }

            // 2. Client-side Validation: Extension
            const allowedExtsStr = settings.storage_allowed_extensions || "jpg,jpeg,png,gif,pdf,doc,docx,xls,xlsx,zip,mp4,mov";
            const allowedExts = allowedExtsStr.split(',').map((s: string) => s.trim().toLowerCase());
            const fileExt = file.name.split('.').pop()?.toLowerCase() || "";

            if (!allowedExts.includes(fileExt) && !allowedExts.includes("*")) {
                toast.error(`File type .${fileExt} is not allowed.`);
                return;
            }

            const id = Math.random().toString(36).substring(7);
            const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';
            const CHUNK_SIZE = 5 * 1024 * 1024; // 5MB Chunks (Safe for CF/Nginx)

            const item: UploadItem = {
                id,
                file,
                progress: 0,
                status: "uploading",
            };

            // Add to store
            update(items => [item, ...items]);

            try {
                // Step 1: Init
                const initRes = await fetch(`${API_BASE}/storage/upload/init`, {
                    method: 'POST',
                    headers: { 'Authorization': `Bearer ${token}` }
                });

                if (!initRes.ok) throw new Error("Failed to initialize upload");
                const { upload_id } = await initRes.json();

                // Step 2: Upload Chunks
                let offset = 0;
                const total = file.size;

                while (offset < total) {
                    // Check if cancelled
                    const currentItem = get(uploadStore).find(i => i.id === id);
                    if (!currentItem || currentItem.status !== 'uploading') break;

                    const chunk = file.slice(offset, offset + CHUNK_SIZE);
                    const formData = new FormData();
                    formData.append("upload_id", upload_id);
                    formData.append("chunk", chunk);

                    const chunkRes = await fetch(`${API_BASE}/storage/upload/chunk`, {
                        method: 'POST',
                        headers: { 'Authorization': `Bearer ${token}` },
                        body: formData
                    });

                    if (!chunkRes.ok) throw new Error(`Chunk upload failed at offset ${offset}`);

                    offset += chunk.size;
                    const percent = Math.round((offset / total) * 100);

                    update(items =>
                        items.map(i => i.id === id ? { ...i, progress: percent } : i)
                    );
                }

                // Step 3: Complete
                const completeRes = await fetch(`${API_BASE}/storage/upload/complete`, {
                    method: 'POST',
                    headers: {
                        'Authorization': `Bearer ${token}`,
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({
                        upload_id,
                        file_name: file.name,
                        content_type: file.type || 'application/octet-stream'
                    })
                });

                if (!completeRes.ok) throw new Error("Failed to finalize upload");

                // Success
                update(items =>
                    items.map(i => i.id === id ? { ...i, status: "success", progress: 100 } : i)
                );

                // Cleanup
                setTimeout(() => {
                    update(items => items.filter(i => i.id !== id));
                }, 5000);

            } catch (e: any) {
                update(items =>
                    items.map(i => i.id === id ? { ...i, status: "error", error: e.message } : i)
                );
                toast.error(`Failed: ${file.name}`);
            }
        },

        // Cancel an upload (Logical cancel only)
        cancel: (id: string) => {
            update(items => {
                // Removing it will break the loop in 'upload' function
                return items.filter(i => i.id !== id);
            });
        },

        // Clear finished uploads
        clearFinished: () => {
            update(items => items.filter(i => i.status === 'uploading' || i.status === 'pending'));
        }
    };
}

export const uploadStore = createUploadStore();

// Derived store for global loading state if needed
export const isUploading = derived(uploadStore, $items => $items.some(i => i.status === 'uploading'));
