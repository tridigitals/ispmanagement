<script lang="ts">
    import { createEventDispatcher, onMount } from "svelte";
    import { fade, scale } from "svelte/transition";
    import Icon from "$lib/components/Icon.svelte";
    import type { FileRecord } from "$lib/api/client";

    export let files: FileRecord[] = [];
    export let index: number = 0;

    const dispatch = createEventDispatcher();
    const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';

    $: currentFile = files[index];
    $: isImage = currentFile?.content_type.startsWith("image/");
    $: isVideo = currentFile?.content_type.startsWith("video/");
    $: isAudio = currentFile?.content_type.startsWith("audio/");
    $: isPdf = currentFile?.content_type === "application/pdf";
    $: isText = currentFile?.content_type.startsWith("text/") || 
                currentFile?.content_type === "application/json" ||
                currentFile?.content_type === "application/xml" ||
                currentFile?.original_name.endsWith(".md") ||
                currentFile?.original_name.endsWith(".csv");
    
    // Use HTTP API endpoint for serving files
    $: fileSrc = currentFile ? `${API_BASE}/storage/files/${currentFile.id}/content` : "";

    let textContent = "";
    let loadingText = false;

    $: if (currentFile && isText) {
        loadTextContent();
    }

    async function loadTextContent() {
        loadingText = true;
        textContent = "";
        try {
            const res = await fetch(fileSrc);
            if (res.ok) {
                textContent = await res.text();
            } else {
                textContent = "Failed to load content.";
            }
        } catch (e) {
            textContent = "Error loading content.";
        } finally {
            loadingText = false;
        }
    }

    function close() {
        dispatch("close");
    }

    function next(e?: Event) {
        if(e) e.stopPropagation();
        if (index < files.length - 1) {
            index++;
        } else {
            index = 0; // Loop back to start
        }
    }

    function prev(e?: Event) {
        if(e) e.stopPropagation();
        if (index > 0) {
            index--;
        } else {
            index = files.length - 1; // Loop to end
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") close();
        if (e.key === "ArrowRight") next();
        if (e.key === "ArrowLeft") prev();
    }

    function formatSize(bytes: number) {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
    }
</script>

<svelte:window on:keydown={handleKeydown} />

<div 
    class="lightbox-overlay" 
    transition:fade={{ duration: 200 }}
    on:click={close}
>
    <!-- Top Bar -->
    <div class="top-bar" on:click|stopPropagation>
        <div class="file-counter">
            {index + 1} / {files.length}
        </div>
        <div class="actions">
            <!-- <button class="action-btn" title="Download">
                <Icon name="download" size={20} />
            </button> -->
            <button class="action-btn close" on:click={close}>
                <Icon name="x" size={24} />
            </button>
        </div>
    </div>

    <!-- Navigation Buttons -->
    <button class="nav-btn prev" on:click={prev} title="Previous">
        <Icon name="chevron-left" size={32} />
    </button>

    <button class="nav-btn next" on:click={next} title="Next">
        <Icon name="chevron-right" size={32} />
    </button>

    <!-- Main Content -->
    <div class="content-wrapper" on:click|stopPropagation>
        {#key currentFile.id}
            <div class="media-container" in:scale={{ start: 0.95, duration: 200 }}>
                {#if isImage}
                    <img src={fileSrc} alt={currentFile.original_name} class="media-content" />
                {:else if isVideo}
                    <!-- svelte-ignore a11y-media-has-caption -->
                    <video src={fileSrc} controls class="media-content" autoplay></video>
                {:else if isAudio}
                    <div class="audio-player">
                        <div class="audio-visual">
                            <Icon name="music" size={80} />
                        </div>
                        <audio src={fileSrc} controls autoplay class="w-full mt-6"></audio>
                    </div>
                {:else if isPdf}
                    <object data={fileSrc} type="application/pdf" class="pdf-viewer">
                        <div class="generic-file">
                            <p>Browser does not support PDF preview.</p>
                            <a href={fileSrc} download={currentFile.original_name} class="btn-download">Download PDF</a>
                        </div>
                    </object>
                {:else if isText}
                    <div class="text-viewer">
                        {#if loadingText}
                            <div class="spinner"></div>
                        {:else}
                            <pre>{textContent}</pre>
                        {/if}
                    </div>
                {:else}
                    <div class="generic-file">
                        <Icon name="file-text" size={64} class="mb-4 text-gray-400" />
                        <h3>Preview not available</h3>
                        <p>{currentFile.original_name}</p>
                        <p class="text-sm text-gray-500 mb-6">{formatSize(currentFile.size)}</p>
                        <a href={fileSrc} download={currentFile.original_name} class="btn-download">
                            <Icon name="download" size={18} />
                            Download File
                        </a>
                    </div>
                {/if}
            </div>
        {/key}
        
        <div class="caption">
            <h3>{currentFile.original_name}</h3>
            <p>{formatSize(currentFile.size)} • {new Date(currentFile.created_at).toLocaleString()}</p>
        </div>
    </div>
</div>

<style>
    .lightbox-overlay {
        position: fixed;
        inset: 0;
        z-index: 9999;
        background-color: rgba(0, 0, 0, 0.9);
        backdrop-filter: blur(5px);
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
    }

    .top-bar {
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        padding: 1.5rem 2rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        z-index: 50;
    }

    .file-counter {
        font-family: monospace;
        background: rgba(255, 255, 255, 0.1);
        padding: 4px 12px;
        border-radius: 20px;
        font-size: 0.9rem;
    }

    .action-btn {
        background: transparent;
        border: none;
        color: rgba(255, 255, 255, 0.7);
        cursor: pointer;
        padding: 8px;
        border-radius: 50%;
        transition: all 0.2s;
        display: flex;
    }

    .action-btn:hover {
        background: rgba(255, 255, 255, 0.1);
        color: white;
    }

    .action-btn.close:hover {
        background: rgba(239, 68, 68, 0.2);
        color: #ef4444;
    }

    .nav-btn {
        position: absolute;
        top: 50%;
        transform: translateY(-50%);
        background: rgba(0, 0, 0, 0.3);
        border: none;
        color: white;
        cursor: pointer;
        padding: 1rem;
        z-index: 40;
        transition: all 0.2s;
        border-radius: 8px;
    }

    .nav-btn:hover {
        background: rgba(255, 255, 255, 0.1);
    }

    .nav-btn.prev { left: 1rem; }
    .nav-btn.next { right: 1rem; }

    .content-wrapper {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;
        max-width: 90vw;
        max-height: 90vh;
    }

    .media-container {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
        overflow: hidden;
    }

    .media-content {
        max-width: 100%;
        max-height: 80vh;
        object-fit: contain;
        border-radius: 4px;
        box-shadow: 0 20px 50px rgba(0,0,0,0.5);
    }

    .audio-player {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        background: rgba(30, 41, 59, 0.8);
        padding: 3rem;
        border-radius: 16px;
        min-width: 300px;
        width: 100%;
        max-width: 500px;
    }

    .pdf-viewer {
        width: 80vw;
        height: 80vh;
        border-radius: 8px;
        background: white;
    }

    .text-viewer {
        background: white;
        color: #1e293b;
        padding: 2rem;
        border-radius: 8px;
        width: 80vw;
        height: 80vh;
        overflow: auto;
        font-family: monospace;
        white-space: pre-wrap;
    }

    .btn-download {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.5rem;
        background: var(--color-primary, #6366f1);
        color: white;
        text-decoration: none;
        border-radius: 8px;
        font-weight: 600;
        transition: filter 0.2s;
    }

    .btn-download:hover {
        filter: brightness(1.1);
    }

    .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid rgba(0,0,0,0.1);
        border-top-color: var(--color-primary, #6366f1);
        border-radius: 50%;
        animation: spin 1s linear infinite;
        margin: auto;
    }

    @keyframes spin { to { transform: rotate(360deg); } }

    .audio-visual {
        width: 120px;
        height: 120px;
        background: linear-gradient(135deg, #06b6d4, #3b82f6);
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        box-shadow: 0 10px 30px rgba(6, 182, 212, 0.3);
        margin-bottom: 1.5rem;
    }

    .generic-file {
        background: #1e293b;
        padding: 4rem;
        border-radius: 1rem;
        text-align: center;
        min-width: 300px;
    }

    .caption {
        text-align: center;
    }

    .caption h3 {
        margin: 0;
        font-weight: 500;
        font-size: 1.1rem;
    }

    .caption p {
        margin: 0.25rem 0 0 0;
        font-size: 0.85rem;
        color: rgba(255, 255, 255, 0.6);
    }
</style>
