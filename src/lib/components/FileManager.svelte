<script lang="ts">
    import { onMount } from "svelte";
    import { api, type FileRecord } from "$lib/api/client";
    import { toast } from "svelte-sonner";
    import { uploadStore } from "$lib/stores/upload";
    import { token } from "$lib/stores/auth";
    import Icon from "$lib/components/Icon.svelte";
    import Lightbox from "$lib/components/Lightbox.svelte";
    import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
    import { fade, fly } from "svelte/transition";
    import { flip } from "svelte/animate";

    // Props
    export let mode: "admin" | "tenant" = "admin";

    // State
    let files: FileRecord[] = [];
    let loading = true;
    let viewMode: "list" | "grid" = "grid";
    let fileInput: HTMLInputElement;
    let selectedFileIndex = -1; // -1 means lightbox closed
    
    // Modal State
    let showDeleteModal = false;
    let fileToDelete: FileRecord | null = null;
    let isDeleting = false;
    
    // Pagination & Search
    let page = 1;
    let perPage = 24;
    let total = 0;
    let totalPages = 1;
    let searchQuery = "";
    
    // Stats
    let totalSize = 0;

    async function loadFiles() {
        loading = true;
        try {
            let res;
            if (mode === "admin") {
                res = await api.storage.listFiles(page, perPage, searchQuery);
            } else {
                res = await api.storage.listFilesTenant(page, perPage, searchQuery);
            }
            
            files = res.data;
            total = res.total;
            totalPages = Math.ceil(total / perPage);
            
            // Calculate size for current page
            if(files.length > 0) totalSize = files.reduce((acc, curr) => acc + curr.size, 0);
        } catch (e: any) {
            toast.error("Failed to load files: " + e.message);
        } finally {
            loading = false;
        }
    }

    async function handleFileSelect(e: Event) {
        const target = e.target as HTMLInputElement;
        if (target.files && target.files.length > 0) {
            const file = target.files[0];
            // Delegate to global upload store
            if ($token) {
                uploadStore.upload(file, $token);
            }
            // Reset input
            target.value = '';
        }
    }

    function openLightbox(index: number) {
        selectedFileIndex = index;
    }

    function confirmDelete(file: FileRecord) {
        fileToDelete = file;
        showDeleteModal = true;
    }

    async function handleConfirmDelete() {
        if (!fileToDelete) return;
        
        isDeleting = true;
        try {
            if (mode === "admin") {
                await api.storage.deleteFile(fileToDelete.id);
            } else {
                await api.storage.deleteFileTenant(fileToDelete.id);
            }
            toast.success("File deleted successfully");
            showDeleteModal = false;
            fileToDelete = null;
            loadFiles();
        } catch (e: any) {
            toast.error(e.message);
        } finally {
            isDeleting = false;
        }
    }

    // --- Helpers ---
    function formatSize(bytes: number) {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
    }

    function getFileIcon(mimeType: string) {
        if (mimeType.startsWith("image/")) return "image";
        if (mimeType.startsWith("video/")) return "film";
        if (mimeType.startsWith("audio/")) return "music";
        if (mimeType.includes("pdf")) return "file-text";
        if (mimeType.includes("zip") || mimeType.includes("compressed")) return "archive";
        return "file";
    }

    function getIconColorClass(mimeType: string) {
        if (mimeType.startsWith("image/")) return "icon-purple";
        if (mimeType.startsWith("video/")) return "icon-pink";
        if (mimeType.startsWith("audio/")) return "icon-cyan";
        if (mimeType.includes("pdf")) return "icon-red";
        if (mimeType.includes("zip")) return "icon-orange";
        return "icon-blue";
    }

    // Debounced Search
    let searchTimer: any;
    function handleSearch(e: Event) {
        const val = (e.target as HTMLInputElement).value;
        searchQuery = val;
        clearTimeout(searchTimer);
        searchTimer = setTimeout(() => {
            page = 1;
            loadFiles();
        }, 300);
    }

    onMount(() => {
        loadFiles();
    });
</script>

<div class="page-container" in:fly={{ y: 20, duration: 300 }}>
    
    <!-- Page Header -->
    <div class="page-header">
        <div class="header-content">
            <h1>{mode === 'admin' ? 'Global Storage Manager' : 'File Manager'}</h1>
            <p class="subtitle">
                {mode === 'admin' 
                    ? 'Manage uploaded files and assets across all tenants' 
                    : 'Manage your organization\'s files and assets'}
            </p>
        </div>
        <div class="header-actions">
            {#if mode !== 'admin'}
                <input 
                    type="file" 
                    class="hidden" 
                    bind:this={fileInput} 
                    on:change={handleFileSelect} 
                />
                <button 
                    class="btn-primary" 
                    on:click={() => fileInput.click()}
                >
                    <Icon name="plus" size={18} />
                    <span>Upload</span>
                </button>
            {/if}

            <div class="stats-badge">
                <Icon name="hard-drive" size={16} />
                <span>{total} Files</span>
            </div>
            <button class="btn-refresh" on:click={loadFiles} title="Refresh">
                <Icon name="refresh-cw" size={18} class={loading ? 'spin' : ''} />
            </button>
        </div>
    </div>

    <!-- Main Content Card -->
    <div class="content-card">
        
        <!-- Toolbar -->
        <div class="toolbar">
            <div class="search-box">
                <Icon name="search" size={18} class="search-icon" />
                <input 
                    type="text" 
                    placeholder="Search files by name..."
                    bind:value={searchQuery}
                    on:input={handleSearch}
                />
            </div>
            
            <div class="view-toggles">
                <button 
                    class="toggle-btn {viewMode === 'grid' ? 'active' : ''}" 
                    on:click={() => viewMode = 'grid'}
                    title="Grid View"
                >
                    <Icon name="grid" size={18} />
                </button>
                <button 
                    class="toggle-btn {viewMode === 'list' ? 'active' : ''}" 
                    on:click={() => viewMode = 'list'}
                    title="List View"
                >
                    <Icon name="list" size={18} />
                </button>
            </div>
        </div>

        <!-- File Browser -->
        <div class="browser-area">
            {#if loading && files.length === 0}
                <div class="loading-state">
                    <div class="spinner"></div>
                    <p>Loading files...</p>
                </div>
            {:else if files.length === 0}
                <div class="empty-state">
                    <div class="empty-icon">
                        <Icon name="folder" size={48} />
                    </div>
                    <h3>No Files Found</h3>
                    <p>Try adjusting your search terms.</p>
                </div>
            {:else}
                
                {#if viewMode === 'grid'}
                    <!-- Grid View -->
                    <div class="grid-view">
                        {#each files as file, index (file.id)}
                            <div 
                                class="file-card cursor-pointer" 
                                animate:flip={{duration: 200}}
                                on:dblclick={() => openLightbox(index)}
                            >
                                <div class="file-preview {getIconColorClass(file.content_type)}">
                                    <Icon name={getFileIcon(file.content_type)} size={32} />
                                </div>
                                <div class="file-info">
                                    <div class="file-name" title={file.original_name}>{file.original_name}</div>
                                    <div class="file-meta">
                                        <span>{formatSize(file.size)}</span>
                                        <span>•</span>
                                        <span>{new Date(file.created_at).toLocaleDateString()}</span>
                                    </div>
                                </div>
                                <div class="file-actions">
                                    <button class="action-btn delete" on:click|stopPropagation={() => confirmDelete(file)} title="Delete">
                                        <Icon name="trash-2" size={14} />
                                    </button>
                                </div>
                            </div>
                        {/each}
                    </div>
                {:else}
                    <!-- List View -->
                    <div class="list-view">
                        <table class="file-table">
                            <thead>
                                <tr>
                                    <th>Name</th>
                                    <th>Size</th>
                                    <th>Type</th>
                                    <th>Uploaded</th>
                                    <th class="text-right">Action</th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each files as file, index (file.id)}
                                    <tr 
                                        animate:flip={{duration: 200}}
                                        class="cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                                        on:dblclick={() => openLightbox(index)}
                                    >
                                        <td>
                                            <div class="file-cell">
                                                <div class="list-icon {getIconColorClass(file.content_type)}">
                                                    <Icon name={getFileIcon(file.content_type)} size={16} />
                                                </div>
                                                <span class="name-text" title={file.original_name}>{file.original_name}</span>
                                            </div>
                                        </td>
                                        <td class="meta-text">{formatSize(file.size)}</td>
                                        <td class="meta-text uppercase">{file.content_type.split('/')[1] || 'FILE'}</td>
                                        <td class="meta-text">{new Date(file.created_at).toLocaleDateString()}</td>
                                        <td class="text-right">
                                            <button class="text-btn delete" on:click|stopPropagation={() => confirmDelete(file)}>
                                                Delete
                                            </button>
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>
                {/if}

            {/if}
        </div>

        <!-- Pagination Footer -->
        {#if totalPages > 1}
            <div class="pagination-footer">
                <span class="page-info">Page {page} of {totalPages}</span>
                <div class="page-controls">
                    <button disabled={page === 1} on:click={() => {page--; loadFiles()}}>Previous</button>
                    <button disabled={page === totalPages} on:click={() => {page++; loadFiles()}}>Next</button>
                </div>
            </div>
        {/if}

    </div>
    
    <!-- Lightbox Overlay -->
    {#if selectedFileIndex > -1}
        <Lightbox 
            bind:index={selectedFileIndex} 
            {files} 
            on:close={() => selectedFileIndex = -1} 
        />
    {/if}

    <ConfirmDialog
        bind:show={showDeleteModal}
        title="Delete File"
        message={`Are you sure you want to permanently delete "${fileToDelete?.original_name}"?`}
        confirmText="Delete"
        type="danger"
        loading={isDeleting}
        on:confirm={handleConfirmDelete}
    />
</div>

<style>
    /* Layout Structure */
    .page-container {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
        max-width: 1400px;
        margin: 0 auto;
        width: 100%;
        height: 100%; /* Important for component reuse */
        padding: 1rem 2rem 2rem 2rem; /* Added space around the content */
    }

    /* Header */
    .page-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
        flex-wrap: wrap;
        gap: 1rem;
    }

    .page-header h1 {
        font-size: 1.75rem;
        font-weight: 700;
        color: var(--text-primary);
        margin: 0 0 0.25rem 0;
    }

    .subtitle {
        color: var(--text-secondary);
        font-size: 0.95rem;
        margin: 0;
    }

    .header-actions {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .stats-badge {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.4rem 0.8rem;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--text-secondary);
    }

    .btn-refresh {
        width: 36px;
        height: 36px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 8px;
        border: 1px solid var(--border-color);
        background: var(--bg-surface);
        color: var(--text-secondary);
        cursor: pointer;
        transition: all 0.2s;
    }

    .btn-refresh:hover {
        border-color: var(--color-primary);
        color: var(--color-primary);
    }

    .btn-primary {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: var(--color-primary);
        color: white;
        border: none;
        border-radius: 8px;
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        transition: filter 0.2s;
        height: 36px;
    }

    .btn-primary:hover {
        filter: brightness(1.1);
    }

    .btn-primary:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    .hidden {
        display: none;
    }

    /* Content Card */
    .content-card {
        background: var(--bg-surface);
        border-radius: var(--radius-lg);
        border: 1px solid var(--border-color);
        box-shadow: var(--shadow-sm);
        display: flex;
        flex-direction: column;
        min-height: 500px; /* Substantial height */
        flex: 1; /* Expand to fill space */
    }

    /* Toolbar */
    .toolbar {
        padding: 1rem 1.5rem;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 1rem;
    }

    .search-box {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        background: var(--bg-app);
        padding: 0.5rem 1rem;
        border-radius: 10px;
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
        transition: all 0.2s;
        flex: 1;
        max-width: 450px;
    }

    .search-box:focus-within {
        border-color: var(--color-primary);
        background: var(--bg-surface);
        box-shadow: 0 0 0 3px var(--color-primary-subtle);
        color: var(--color-primary);
    }

    .search-box input {
        background: transparent;
        border: none;
        color: var(--text-primary);
        outline: none;
        width: 100%;
        font-size: 0.9rem;
    }

    .search-icon {
        flex-shrink: 0;
        opacity: 0.7;
    }

    .view-toggles {
        display: flex;
        background: var(--bg-app);
        padding: 3px;
        border-radius: 8px;
        border: 1px solid var(--border-color);
    }

    .toggle-btn {
        padding: 0.4rem;
        border-radius: 6px;
        border: none;
        background: transparent;
        color: var(--text-secondary);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
    }

    .toggle-btn:hover {
        color: var(--text-primary);
    }

    .toggle-btn.active {
        background: var(--bg-surface);
        color: var(--color-primary);
        box-shadow: 0 1px 3px rgba(0,0,0,0.1);
    }

    /* Browser Area */
    .browser-area {
        flex: 1;
        padding: 1.5rem;
        background: var(--bg-hover); /* Slight contrast for content area */
        overflow-y: auto; /* Allow scrolling internally */
    }

    /* Grid View */
    .grid-view {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
        gap: 1rem;
    }

    .file-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1rem;
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
        transition: all 0.2s;
        position: relative;
    }

    .file-card:hover {
        transform: translateY(-2px);
        box-shadow: var(--shadow-md);
        border-color: var(--color-primary);
    }

    .file-preview {
        width: 64px;
        height: 64px;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-bottom: 1rem;
    }

    .file-info {
        width: 100%;
        margin-bottom: 0.5rem;
    }

    .file-name {
        font-weight: 500;
        font-size: 0.9rem;
        color: var(--text-primary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin-bottom: 0.2rem;
    }

    .file-meta {
        font-size: 0.75rem;
        color: var(--text-secondary);
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.4rem;
    }

    .file-actions {
        position: absolute;
        top: 0.5rem;
        right: 0.5rem;
        opacity: 0;
        transition: opacity 0.2s;
    }

    .file-card:hover .file-actions {
        opacity: 1;
    }

    .action-btn {
        width: 28px;
        height: 28px;
        border-radius: 6px;
        border: none;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        background: var(--bg-app);
        color: var(--text-secondary);
    }

    .action-btn.delete:hover {
        background: #fee2e2;
        color: #ef4444;
    }

    /* List View */
    .file-table {
        width: 100%;
        border-collapse: separate;
        border-spacing: 0;
        background: var(--bg-surface);
        border-radius: 8px;
        border: 1px solid var(--border-color);
        overflow: hidden;
    }

    .file-table th {
        text-align: left;
        padding: 0.8rem 1rem;
        font-size: 0.8rem;
        font-weight: 600;
        color: var(--text-secondary);
        background: var(--bg-app);
        border-bottom: 1px solid var(--border-color);
    }

    .file-table td {
        padding: 0.8rem 1rem;
        border-bottom: 1px solid var(--border-subtle);
        vertical-align: middle;
    }

    .file-table tr:last-child td {
        border-bottom: none;
    }

    .file-cell {
        display: flex;
        align-items: center;
        gap: 0.8rem;
    }

    .list-icon {
        width: 32px;
        height: 32px;
        border-radius: 6px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .name-text {
        font-weight: 500;
        font-size: 0.9rem;
        color: var(--text-primary);
        max-width: 300px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .meta-text {
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .uppercase { text-transform: uppercase; }
    .text-right { text-align: right; }

    .text-btn {
        background: none;
        border: none;
        font-size: 0.8rem;
        font-weight: 500;
        cursor: pointer;
    }

    .text-btn.delete {
        color: var(--color-danger);
        opacity: 0.7;
    }
    .text-btn.delete:hover {
        opacity: 1;
        text-decoration: underline;
    }

    /* Icon Colors */
    .icon-purple { background: rgba(168, 85, 247, 0.1); color: #a855f7; }
    .icon-pink { background: rgba(236, 72, 153, 0.1); color: #ec4899; }
    .icon-cyan { background: rgba(6, 182, 212, 0.1); color: #06b6d4; }
    .icon-red { background: rgba(239, 68, 68, 0.1); color: #ef4444; }
    .icon-orange { background: rgba(249, 115, 22, 0.1); color: #f97316; }
    .icon-blue { background: rgba(59, 130, 246, 0.1); color: #3b82f6; }

    /* States */
    .loading-state, .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 4rem 1rem;
        color: var(--text-secondary);
    }

    .empty-icon {
        margin-bottom: 1rem;
        opacity: 0.5;
    }

    .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        margin-bottom: 1rem;
        animation: spin 1s linear infinite;
    }

    .spin { animation: spin 1s linear infinite; }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }

    /* Pagination */
    .pagination-footer {
        padding: 1rem 1.5rem;
        border-top: 1px solid var(--border-color);
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: var(--bg-surface);
    }

    .page-info {
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .page-controls {
        display: flex;
        gap: 0.5rem;
    }

    .page-controls button {
        padding: 0.3rem 0.8rem;
        border: 1px solid var(--border-color);
        background: var(--bg-app);
        border-radius: 6px;
        font-size: 0.85rem;
        color: var(--text-primary);
        cursor: pointer;
        transition: all 0.2s;
    }

    .page-controls button:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .page-controls button:not(:disabled):hover {
        border-color: var(--color-primary);
        color: var(--color-primary);
    }
</style>
