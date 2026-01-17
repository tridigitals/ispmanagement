import { writable, derived, get } from "svelte/store";
import { toast } from "svelte-sonner";
import { appSettings } from "$lib/stores/settings";

export interface UploadItem {
    id: string;
    file: File;
    progress: number; // 0 - 100
    status: "pending" | "uploading" | "success" | "error";
    error?: string;
    xhr?: XMLHttpRequest;
}

function createUploadStore() {
    const { subscribe, update, set } = writable<UploadItem[]>([]);

    return {
        subscribe,
        
        // Start a new upload
        upload: (file: File, token: string) => {
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
            const allowedExts = allowedExtsStr.split(',').map(s => s.trim().toLowerCase());
            const fileExt = file.name.split('.').pop()?.toLowerCase() || "";

            if (!allowedExts.includes(fileExt) && !allowedExts.includes("*")) {
                toast.error(`File type .${fileExt} is not allowed.`);
                return;
            }

            const id = Math.random().toString(36).substring(7);
            const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';
            const UPLOAD_URL = `${API_BASE}/storage/upload`;

            console.log(`[Upload] Starting upload: ${file.name} (${file.size} bytes)`);
            console.log(`[Upload] Target URL: ${UPLOAD_URL}`);

            const item: UploadItem = {
                id,
                file,
                progress: 0,
                status: "uploading",
            };

            // Add to store
            update(items => [item, ...items]);

            // Create XHR
            const xhr = new XMLHttpRequest();
            item.xhr = xhr;

            // Setup Data
            const formData = new FormData();
            formData.append("file", file);

            // Progress Handler
            xhr.upload.onprogress = (e) => {
                if (e.lengthComputable) {
                    const percent = Math.round((e.loaded / e.total) * 100);
                    // console.log(`[Upload] Progress: ${percent}%`); // Uncomment if needed, spammy
                    update(items => 
                        items.map(i => i.id === id ? { ...i, progress: percent } : i)
                    );
                }
            };

            xhr.upload.onloadstart = () => {
                console.log("[Upload] Network: Upload started");
            };

            // Completion Handler
            xhr.onload = () => {
                console.log(`[Upload] Network: Finished with status ${xhr.status}`);
                if (xhr.status >= 200 && xhr.status < 300) {
                    update(items => 
                        items.map(i => i.id === id ? { ...i, status: "success", progress: 100 } : i)
                    );
                    // toast.success(`Uploaded: ${file.name}`);
                    
                    // Auto remove success after 5 seconds
                    setTimeout(() => {
                        update(items => items.filter(i => i.id !== id));
                    }, 5000);

                } else {
                    let errorMessage = "Upload failed";
                    try {
                        const res = JSON.parse(xhr.responseText);
                        errorMessage = res.error || res.message || errorMessage;
                    } catch (e) {
                        errorMessage = xhr.responseText || errorMessage;
                    }
                    console.error("[Upload] Server Error:", errorMessage);
                    update(items => 
                        items.map(i => i.id === id ? { ...i, status: "error", error: errorMessage } : i)
                    );
                    toast.error(`Failed: ${file.name} - ${errorMessage}`);
                }
            };

            // Error Handler
            xhr.onerror = () => {
                console.error("[Upload] Network Error (XHR onerror)");
                update(items => 
                    items.map(i => i.id === id ? { ...i, status: "error", error: "Network Error" } : i)
                );
                toast.error(`Network Error: ${file.name}`);
            };
            
            xhr.onabort = () => {
                console.warn("[Upload] Aborted by user");
            };

            // Execute
            console.log("[Upload] Sending request...");
            xhr.open("POST", UPLOAD_URL);
            xhr.setRequestHeader("Authorization", `Bearer ${token}`);
            xhr.send(formData);
        },

        // Cancel an upload
        cancel: (id: string) => {
            update(items => {
                const item = items.find(i => i.id === id);
                if (item && item.xhr && item.status === 'uploading') {
                    item.xhr.abort();
                }
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
