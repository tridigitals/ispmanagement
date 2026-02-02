<script lang="ts">
    import { onMount } from "svelte";
    import { api, type FileRecord } from "$lib/api/client";
    import { toast } from "svelte-sonner";
    import { uploadStore } from "$lib/stores/upload";
    import { token, can } from "$lib/stores/auth";
    import Icon from "$lib/components/ui/Icon.svelte";
    import Lightbox from "$lib/components/ui/Lightbox.svelte";
    import ConfirmDialog from "$lib/components/ui/ConfirmDialog.svelte";
    import { fade, fly } from "svelte/transition";
    import { flip } from "svelte/animate";
    import { t } from "svelte-i18n";
    import { get } from "svelte/store";

    // Props (Svelte 5)
    let { mode = "admin", showHeader = true } = $props();

    // State
    let files = $state<FileRecord[]>([]);
    let loading = $state(true);
    let viewMode = $state<"list" | "grid">("grid");
    let fileInput = $state<HTMLInputElement>();
    let selectedFileIndex = $state(-1); // -1 means lightbox closed

    // API URL
    const API_BASE =
        import.meta.env.VITE_API_URL || "http://localhost:3000/api";

    // Modal State
    let showDeleteModal = $state(false);
    let fileToDelete = $state<FileRecord | null>(null);
    let isDeleting = $state(false);

    // Selection State
    let selectedFileIds = $state<string[]>([]);
    let isBatchDeleting = $state(false);

    // Pagination & Search
    let page = $state(1);
    let perPage = $state(24);
    let total = $state(0);
    let totalPages = $state(1);
    let searchQuery = $state("");
    let activeFilter = $state<"all" | "image" | "video" | "audio" | "document">(
        "all",
    );

    // Stats
    let totalSize = $state(0);

    function matchesFilter(f: FileRecord, filter: typeof activeFilter) {
        if (filter === "all") return true;
        if (filter === "image") return f.content_type.startsWith("image/");
        if (filter === "video") return f.content_type.startsWith("video/");
        if (filter === "audio") return f.content_type.startsWith("audio/");
        if (filter === "document")
            return (
                !f.content_type.startsWith("image/") &&
                !f.content_type.startsWith("video/") &&
                !f.content_type.startsWith("audio/")
            );
        return true;
    }

    const filterCards = $derived.by(
        () =>
            [
                {
                    id: "all",
                    label:
                        $t("components.file_manager.filters.all") || "All Files",
                    icon: "hard-drive",
                },
                {
                    id: "image",
                    label:
                        $t("components.file_manager.filters.images") || "Images",
                    icon: "image",
                },
                {
                    id: "video",
                    label:
                        $t("components.file_manager.filters.videos") || "Videos",
                    icon: "film",
                },
                {
                    id: "audio",
                    label:
                        $t("components.file_manager.filters.audio") || "Audio",
                    icon: "music",
                },
                {
                    id: "document",
                    label:
                        $t("components.file_manager.filters.documents") ||
                        "Documents",
                    icon: "file-text",
                },
            ] as const,
    );

    let filterCounts = $derived.by(() => {
        const counts: Record<string, number> = {
            all: files.length,
            image: 0,
            video: 0,
            audio: 0,
            document: 0,
        };

        for (const f of files) {
            if (matchesFilter(f, "image")) counts.image++;
            if (matchesFilter(f, "video")) counts.video++;
            if (matchesFilter(f, "audio")) counts.audio++;
            if (matchesFilter(f, "document")) counts.document++;
        }
        return counts;
    });

    function setActiveFilter(next: typeof activeFilter) {
        activeFilter = next;
        deselectAll();
    }

    // Derived filtered files
    let filteredFiles = $derived(
        files.filter((f) => matchesFilter(f, activeFilter)),
    );

    // Selection Helpers
    function toggleSelection(id: string) {
        if (selectedFileIds.includes(id)) {
            selectedFileIds = selectedFileIds.filter((fid) => fid !== id);
        } else {
            selectedFileIds = [...selectedFileIds, id];
        }
    }

    function selectAll() {
        if (selectedFileIds.length === filteredFiles.length) {
            selectedFileIds = [];
        } else {
            selectedFileIds = filteredFiles.map((f) => f.id);
        }
    }

    function deselectAll() {
        selectedFileIds = [];
    }

    let selectedTotalSize = $derived.by(() => {
        if (selectedFileIds.length === 0) return 0;
        const map = new Map(files.map((f) => [f.id, f.size]));
        return selectedFileIds.reduce((acc, id) => acc + (map.get(id) || 0), 0);
    });

    async function handleBatchDelete() {
        if (selectedFileIds.length === 0) return;

        // Show confirmation using existing modal logic mechanism or new one?
        // Let's repurpose the modal: "Delete 5 files?"
        // Ideally we need a separate confirm flow or reuse the existing dialog with tweaked text.
        // For simplicity, let's reuse fileToDelete=null as a signal for batch?
        // Or cleaner: add isBatch flag to the dialog context.

        fileToDelete = null; // null means batch mode
        showDeleteModal = true;
    }

    async function executeBatchDelete() {
        isDeleting = true;
        let successCount = 0;
        let errors = 0;

        // Execute sequentially or parallel? Parallel limit is safer but Promise.all is okay for <50 files.
        const promises = selectedFileIds.map(async (id) => {
            try {
                if (mode === "admin") {
                    await api.storage.deleteFile(id);
                } else {
                    await api.storage.deleteFileTenant(id);
                }
                successCount++;
            } catch (e) {
                errors++;
            }
        });

        await Promise.all(promises);

        isDeleting = false;
        showDeleteModal = false;
        selectedFileIds = [];

        if (errors > 0) {
            toast.warning(
                `Deleted ${successCount} files. Failed to delete ${errors} files.`,
            );
        } else {
            toast.success(`Deleted ${successCount} files successfully.`);
        }
        loadFiles();
    }

    // Auto-reload on upload success (Svelte 5 Effect)
    $effect(() => {
        const finished = $uploadStore.filter((u) => u.status === "success");
        if (finished.length > 0) {
            loadFiles();
            uploadStore.clearFinished();
        }
    });

    async function loadFiles() {
        loading = true;
        try {
            selectedFileIds = [];
            fileToDelete = null;
            showDeleteModal = false;

            let res;
            if (mode === "admin") {
                res = await api.storage.listFiles(page, perPage, searchQuery);
            } else {
                res = await api.storage.listFilesTenant(
                    page,
                    perPage,
                    searchQuery,
                );
            }

            files = res.data;
            total = res.total;

            totalPages = Math.ceil(total / perPage);

            // Calculate size for current page
            if (files.length > 0)
                totalSize = files.reduce((acc, curr) => acc + curr.size, 0);
        } catch (e: any) {
            toast.error(
                get(t)("components.file_manager.toasts.load_failed", {
                    values: { message: e.message },
                }) || "Failed to load files: " + e.message,
            );
        } finally {
            loading = false;
        }
    }

    async function handleFileSelect(e: Event) {
        const target = e.target as HTMLInputElement;
        if (target.files && target.files.length > 0) {
            // Delegate to global upload store
            if ($token) {
                for (const file of Array.from(target.files)) {
                    uploadStore.upload(file, $token);
                }
            }
            // Reset input
            target.value = "";
        }
    }

    function openLightbox(index: number) {
        selectedFileIndex = index;
    }

    function handleItemClick(index: number) {
        // On mobile, single tap opens lightbox
        if (window.innerWidth < 768) {
            openLightbox(index);
        }
    }

    function confirmDelete(file: FileRecord) {
        fileToDelete = file;
        showDeleteModal = true;
    }

    async function handleConfirmDelete() {
        if (!fileToDelete && selectedFileIds.length > 0) {
            await executeBatchDelete();
            return;
        }

        if (!fileToDelete) return;

        isDeleting = true;
        try {
            if (mode === "admin") {
                await api.storage.deleteFile(fileToDelete.id);
            } else {
                await api.storage.deleteFileTenant(fileToDelete.id);
            }
            toast.success(
                get(t)("components.file_manager.toasts.deleted") ||
                    "File deleted successfully",
            );
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
        if (mimeType.includes("zip") || mimeType.includes("compressed"))
            return "archive";
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
    {#if showHeader}
        <!-- Page Header -->
        <div class="page-header">
            <div class="header-content">
                <h1>
                    {mode === "admin"
                        ? $t("components.file_manager.title_admin") ||
                          "Global Storage Manager"
                        : $t("components.file_manager.title") || "File Manager"}
                </h1>
                <p class="subtitle">
                    {mode === "admin"
                        ? $t("components.file_manager.subtitle_admin") ||
                          "Manage uploaded files and assets across all tenants"
                        : $t("components.file_manager.subtitle") ||
                          "Manage your organization's files and assets"}
                </p>
            </div>
            <div class="header-actions">
                {#if mode !== "admin" && $can("upload", "storage")}
                    <input
                        type="file"
                        class="hidden"
                        multiple
                        bind:this={fileInput}
                        onchange={handleFileSelect}
                    />
                    <button
                        class="btn btn-primary"
                        onclick={() => fileInput?.click()}
                    >
                        <Icon name="plus" size={18} />
                        <span>
                            {$t("components.file_manager.upload") || "Upload"}
                        </span>
                    </button>
                {/if}

                <div class="stats-badge">
                    <Icon name="hard-drive" size={16} />
                    <span>
                        {total} {$t("components.file_manager.files") || "Files"}
                    </span>
                </div>
                <button
                    class="btn-refresh"
                    onclick={loadFiles}
                    title={$t("common.refresh") || "Refresh"}
                    aria-label={$t("common.refresh") || "Refresh"}
                >
                    <Icon
                        name="refresh-cw"
                        size={18}
                        class={loading ? "spin" : ""}
                    />
                </button>
            </div>
        </div>
    {/if}

    <div
        class="filter-cards"
        aria-label={$t("components.file_manager.filters.aria_group") ||
            "File type filters"}
    >
        {#each filterCards as c}
            <button
                type="button"
                class="filter-card"
                class:active={activeFilter === c.id}
                onclick={() => setActiveFilter(c.id)}
                aria-label={$t("components.file_manager.filters.aria_filter", {
                    values: { label: c.label },
                }) || `Filter: ${c.label}`}
                title={$t("components.file_manager.filters.aria_filter", {
                    values: { label: c.label },
                }) || `Filter: ${c.label}`}
            >
                <div class="fc-icon">
                    <Icon name={c.icon} size={18} />
                </div>
                <div class="fc-text">
                    <div class="fc-title">{c.label}</div>
                    <div class="fc-value">{filterCounts[c.id]}</div>
                </div>
            </button>
        {/each}
    </div>

    <!-- Main Content Card -->
    <div class="glass-card">
        <div class="fm-main">
            <!-- Toolbar / Action Bar -->
            <div class="toolbar">
                {#if selectedFileIds.length > 0}
                    <div class="action-bar">
                        <div class="selection-count">
                            <span class="count-pill"
                                >{selectedFileIds.length}</span
                            >
                            <span>
                                {$t("components.file_manager.selected") ||
                                    "Selected"}
                            </span>
                            <span class="sep">•</span>
                            <span class="meta"
                                >{formatSize(selectedTotalSize)}</span
                            >
                        </div>
                        <div class="action-buttons">
                            <button class="btn-ghost" onclick={deselectAll}
                                >{$t("common.cancel") || "Cancel"}</button
                            >
                            <button
                                class="btn-danger-sm"
                                onclick={handleBatchDelete}
                            >
                                <Icon name="trash-2" size={16} />
                                {$t("components.file_manager.delete_selected") ||
                                    "Delete Selected"}
                            </button>
                        </div>
                    </div>
                {:else}
                    <div class="search-box">
                        <Icon name="search" size={18} class="search-icon" />
                        <input
                            type="text"
                            placeholder={$t(
                                "components.file_manager.search_placeholder",
                            ) || "Search files by name..."}
                            bind:value={searchQuery}
                            oninput={handleSearch}
                        />
                    </div>

                    <div class="view-toggles">
                        <button
                            class="toggle-btn {viewMode === 'grid'
                                ? 'active'
                                : ''}"
                            onclick={() => (viewMode = "grid")}
                            title={$t("components.file_manager.view.grid") || "Grid View"}
                            aria-label={$t("components.file_manager.view.grid") || "Grid View"}
                        >
                            <Icon name="grid" size={18} />
                        </button>
                        <button
                            class="toggle-btn {viewMode === 'list'
                                ? 'active'
                                : ''}"
                            onclick={() => (viewMode = "list")}
                            title={$t("components.file_manager.view.list") || "List View"}
                            aria-label={$t("components.file_manager.view.list") || "List View"}
                        >
                            <Icon name="list" size={18} />
                        </button>
                    </div>

                    <div class="toolbar-actions">
                        {#if mode !== "admin" && $can("upload", "storage")}
                            <input
                                type="file"
                                class="hidden"
                                multiple
                                bind:this={fileInput}
                                onchange={handleFileSelect}
                            />
                            <button
                                class="btn btn-primary"
                                onclick={() => fileInput?.click()}
                            >
                                <Icon name="plus" size={18} />
                                <span>
                                    {$t("components.file_manager.upload") ||
                                        "Upload"}
                                </span>
                            </button>
                        {/if}

                        <div class="stats-badge">
                            <Icon name="hard-drive" size={16} />
                            <span>
                                {total} {$t("components.file_manager.files") ||
                                    "Files"}
                            </span>
                        </div>
                        <button
                            class="btn-refresh"
                            onclick={loadFiles}
                            title={$t("common.refresh") || "Refresh"}
                            aria-label={$t("common.refresh") || "Refresh"}
                        >
                            <Icon
                                name="refresh-cw"
                                size={18}
                                class={loading ? "spin" : ""}
                            />
                        </button>
                    </div>
                {/if}
            </div>

            <!-- File Browser -->
            <div class="browser-area">
                {#if loading && files.length === 0}
                    <div class="loading-state">
                        <div class="spinner"></div>
                        <p>
                            {$t("components.file_manager.loading") ||
                                "Loading files..."}
                        </p>
                    </div>
                {:else if files.length === 0}
                    <div class="empty-state">
                        <div class="empty-icon">
                            <Icon name="folder" size={48} />
                        </div>
                        <h3>
                            {$t("components.file_manager.empty.title") ||
                                "No Files Found"}
                        </h3>
                        <p>
                            {$t("components.file_manager.empty.subtitle") ||
                                "Try adjusting your search terms."}
                        </p>
                    </div>
                {:else if viewMode === "grid"}
                    <!-- Grid View -->
                    <div class="grid-view">
                        {#each filteredFiles as file, index (file.id)}
                            <div
                                class="file-card cursor-pointer {selectedFileIds.includes(
                                    file.id,
                                )
                                    ? 'selected'
                                    : ''}"
                                animate:flip={{ duration: 200 }}
                                ondblclick={() => openLightbox(index)}
                                onclick={() => handleItemClick(index)}
                                role="button"
                                tabindex="0"
                                onkeydown={(e) =>
                                    (e.key === "Enter" || e.key === " ") &&
                                    handleItemClick(index)}
                            >
                                {#if $can("delete", "storage")}
                                    <div class="selection-checkbox">
                                        <input
                                            type="checkbox"
                                            checked={selectedFileIds.includes(
                                                file.id,
                                            )}
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                toggleSelection(file.id);
                                            }}
                                        />
                                    </div>
                                {/if}

                                {#if file.content_type.startsWith("image/")}
                                    <div class="file-preview-image">
                                        <img
                                            src={`${API_BASE}/storage/files/${file.id}/content`}
                                            alt={file.original_name}
                                            loading="lazy"
                                            onerror={(e) => {
                                                const target =
                                                    e.currentTarget as HTMLImageElement;
                                                target.style.display = "none";
                                                const fallback =
                                                    target.nextElementSibling as HTMLElement;
                                                if (fallback)
                                                    fallback.style.display =
                                                        "flex";
                                            }}
                                        />
                                        <!-- Fallback Icon (hidden by default) -->
                                        <div
                                            class="file-preview {getIconColorClass(
                                                file.content_type,
                                            )} fallback"
                                            style="display: none;"
                                        >
                                            <Icon
                                                name={getFileIcon(
                                                    file.content_type,
                                                )}
                                                size={32}
                                            />
                                        </div>
                                    </div>
                                {:else}
                                    <div
                                        class="file-preview {getIconColorClass(
                                            file.content_type,
                                        )}"
                                    >
                                        <Icon
                                            name={getFileIcon(
                                                file.content_type,
                                            )}
                                            size={32}
                                        />
                                    </div>
                                {/if}

                                <div class="file-info">
                                    <div
                                        class="file-name"
                                        title={file.original_name}
                                    >
                                        {file.original_name}
                                    </div>
                                    <div class="file-meta">
                                        <span>{formatSize(file.size)}</span>
                                        <span>•</span>
                                        <span
                                            >{new Date(
                                                file.created_at,
                                            ).toLocaleDateString()}</span
                                        >
                                    </div>
                                </div>

                                {#if $can("delete", "storage")}
                                    <div class="file-actions">
                                        <button
                                            class="action-btn delete"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                confirmDelete(file);
                                            }}
                                            title={$t("common.delete") || "Delete"}
                                        >
                                            <Icon name="trash-2" size={14} />
                                        </button>
                                    </div>
                                {/if}
                            </div>
                        {/each}
                    </div>
                {:else}
                    <!-- List View -->
                    <div class="list-view">
                        <table class="file-table">
                            <thead>
                                <tr>
                                    <th class="w-10 text-center">
                                        {#if $can("delete", "storage")}
                                            <input
                                                type="checkbox"
                                                checked={selectedFileIds.length >
                                                    0 &&
                                                    selectedFileIds.length ===
                                                        filteredFiles.length}
                                                indeterminate={selectedFileIds.length >
                                                    0 &&
                                                    selectedFileIds.length <
                                                        filteredFiles.length}
                                                onclick={selectAll}
                                            />
                                        {/if}
                                    </th>
                                    <th>
                                        {$t("components.file_manager.columns.name") ||
                                            "Name"}
                                    </th>
                                    <th>
                                        {$t("components.file_manager.columns.size") ||
                                            "Size"}
                                    </th>
                                    <th>
                                        {$t("components.file_manager.columns.type") ||
                                            "Type"}
                                    </th>
                                    <th>
                                        {$t("components.file_manager.columns.uploaded") ||
                                            "Uploaded"}
                                    </th>
                                    <th class="text-right">
                                        {$t("components.file_manager.columns.action") ||
                                            "Action"}
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each filteredFiles as file, index (file.id)}
                                    <tr
                                        animate:flip={{ duration: 200 }}
                                        class="cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                                        ondblclick={() => openLightbox(index)}
                                        onclick={() => handleItemClick(index)}
                                    >
                                        <td
                                            class="text-center"
                                            onclick={(e) => e.stopPropagation()}
                                        >
                                            {#if $can("delete", "storage")}
                                                <input
                                                    type="checkbox"
                                                    checked={selectedFileIds.includes(
                                                        file.id,
                                                    )}
                                                    onclick={() =>
                                                        toggleSelection(
                                                            file.id,
                                                        )}
                                                />
                                            {/if}
                                        </td>
                                        <td>
                                            <div class="file-cell">
                                                {#if file.content_type.startsWith("image/")}
                                                    <img
                                                        src={`${API_BASE}/storage/files/${file.id}/content`}
                                                        alt={file.original_name}
                                                        class="list-thumbnail"
                                                        loading="lazy"
                                                        onerror={(e) => {
                                                            const target =
                                                                e.currentTarget as HTMLImageElement;
                                                            target.style.display =
                                                                "none";
                                                            const fallback =
                                                                target.nextElementSibling as HTMLElement;
                                                            if (fallback)
                                                                fallback.style.display =
                                                                    "flex";
                                                        }}
                                                    />
                                                    <!-- Fallback -->
                                                    <div
                                                        class="list-icon {getIconColorClass(
                                                            file.content_type,
                                                        )} fallback"
                                                        style="display: none;"
                                                    >
                                                        <Icon
                                                            name={getFileIcon(
                                                                file.content_type,
                                                            )}
                                                            size={16}
                                                        />
                                                    </div>
                                                {:else}
                                                    <div
                                                        class="list-icon {getIconColorClass(
                                                            file.content_type,
                                                        )}"
                                                    >
                                                        <Icon
                                                            name={getFileIcon(
                                                                file.content_type,
                                                            )}
                                                            size={16}
                                                        />
                                                    </div>
                                                {/if}
                                                <span
                                                    class="name-text"
                                                    title={file.original_name}
                                                    >{file.original_name}</span
                                                >
                                            </div>
                                        </td>
                                        <td class="meta-text"
                                            >{formatSize(file.size)}</td
                                        >
                                        <td class="meta-text uppercase"
                                            >{file.content_type.split("/")[1] ||
                                                "FILE"}</td
                                        >
                                        <td class="meta-text"
                                            >{new Date(
                                                file.created_at,
                                            ).toLocaleDateString()}</td
                                        >
                                        <td class="text-right">
                                            {#if $can("delete", "storage")}
                                                <button
                                                    class="text-btn delete"
                                                    onclick={(e) => {
                                                        e.stopPropagation();
                                                        confirmDelete(file);
                                                    }}
                                                >
                                                    {$t("common.delete") ||
                                                        "Delete"}
                                                </button>
                                            {/if}
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>
                {/if}
            </div>

            <!-- Pagination Footer -->
            {#if totalPages > 1}
                <div class="pagination-footer">
                    <span class="page-info">
                        {$t("components.file_manager.pagination.page_of", {
                            values: { page, total: totalPages },
                        }) || `Page ${page} of ${totalPages}`}
                    </span>
                    <div class="page-controls">
                        <button
                            type="button"
                            disabled={page === 1}
                            onclick={() => {
                                page--;
                                loadFiles();
                            }}
                        >
                            {$t("common.previous") || "Previous"}
                        </button>
                        <button
                            type="button"
                            disabled={page === totalPages}
                            onclick={() => {
                                page++;
                                loadFiles();
                            }}
                        >
                            {$t("common.next") || "Next"}
                        </button>
                    </div>
                </div>
            {/if}
        </div>
    </div>

    <!-- Lightbox Overlay -->
    {#if selectedFileIndex > -1}
        <Lightbox
            bind:index={selectedFileIndex}
            {files}
            onclose={() => (selectedFileIndex = -1)}
        />
    {/if}

    <ConfirmDialog
        bind:show={showDeleteModal}
        title={fileToDelete
            ? $t("components.file_manager.confirm.delete_one_title") ||
              "Delete File"
            : $t("components.file_manager.confirm.delete_many_title", {
                  values: { count: selectedFileIds.length },
              }) || `Delete ${selectedFileIds.length} files`}
        message={fileToDelete
            ? $t("components.file_manager.confirm.delete_one_message") ||
              "Are you sure you want to delete this file? This action cannot be undone."
            : $t("components.file_manager.confirm.delete_many_message", {
                  values: {
                      count: selectedFileIds.length,
                      size: formatSize(selectedTotalSize),
                  },
              }) ||
              `Are you sure you want to delete ${selectedFileIds.length} files (${formatSize(selectedTotalSize)})? This action cannot be undone.`}
        confirmText={fileToDelete
            ? $t("common.delete") || "Delete"
            : $t("components.file_manager.confirm.delete_many_confirm", {
                  values: { count: selectedFileIds.length },
              }) || `Delete ${selectedFileIds.length}`}
        type="danger"
        onconfirm={handleConfirmDelete}
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
        height: 100%;
        padding: 1rem 2rem 2rem 2rem;
        box-sizing: border-box; /* Ensure padding doesn't affect width calculation */
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

    .hidden {
        display: none;
    }

    /* Filter cards */
    .filter-cards {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
        gap: 1rem;
    }

    .filter-card {
        background: linear-gradient(145deg, rgba(255, 255, 255, 0.06), rgba(255, 255, 255, 0.02));
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 18px;
        padding: 1rem 1.1rem;
        display: flex;
        align-items: center;
        gap: 0.85rem;
        cursor: pointer;
        text-align: left;
        transition: transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
    }

    .filter-card:hover {
        transform: translateY(-1px);
        border-color: rgba(99, 102, 241, 0.28);
        box-shadow: 0 14px 36px rgba(0, 0, 0, 0.25);
    }

    .filter-card.active {
        border-color: rgba(99, 102, 241, 0.45);
        box-shadow:
            0 14px 36px rgba(0, 0, 0, 0.28),
            0 0 0 1px rgba(99, 102, 241, 0.2) inset;
    }

    .fc-icon {
        width: 42px;
        height: 42px;
        border-radius: 14px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: 1px solid rgba(255, 255, 255, 0.1);
        background: rgba(255, 255, 255, 0.06);
        color: var(--text-primary);
        flex-shrink: 0;
    }

    .fc-text {
        flex: 1;
        min-width: 0;
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: 0.75rem;
    }

    .fc-title {
        font-weight: 750;
        color: var(--text-primary);
        font-size: 0.95rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .fc-value {
        font-weight: 800;
        color: var(--text-primary);
        font-size: 1.1rem;
    }

    :global([data-theme="light"]) .filter-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 10px 28px rgba(0, 0, 0, 0.06),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }

    :global([data-theme="light"]) .filter-card:hover {
        box-shadow:
            0 14px 36px rgba(0, 0, 0, 0.08),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }

    :global([data-theme="light"]) .filter-card.active {
        border-color: rgba(99, 102, 241, 0.3);
        box-shadow:
            0 14px 36px rgba(0, 0, 0, 0.08),
            0 0 0 1px rgba(99, 102, 241, 0.16) inset;
    }

    /* Main glass container */
    .glass-card {
        background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
        border-radius: 16px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
        overflow: hidden;
        min-height: 520px;
    }

    :global([data-theme="light"]) .glass-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 12px 32px rgba(0, 0, 0, 0.08),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }

    .fm-main {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-width: 0; /* Prevent flex overflow */
    }

    .action-bar {
        display: flex;
        justify-content: space-between;
        align-items: center;
        width: 100%;
        animation: fade-in 0.2s;
    }

    .selection-count {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .sep {
        opacity: 0.6;
    }

    .meta {
        color: var(--text-secondary);
        font-weight: 650;
        font-size: 0.9rem;
    }

    .count-pill {
        background: var(--color-primary);
        color: white;
        padding: 0.1rem 0.5rem;
        border-radius: 12px;
        font-size: 0.8rem;
    }

    .action-buttons {
        display: flex;
        gap: 0.5rem;
    }

    .btn-ghost {
        background: transparent;
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
        padding: 0.4rem 0.8rem;
        border-radius: 6px;
        font-size: 0.85rem;
        cursor: pointer;
        transition: all 0.2s;
    }

    .btn-ghost:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .btn-danger-sm {
        background: #fee2e2;
        color: #ef4444;
        border: 1px solid #fecaca;
        padding: 0.4rem 0.8rem;
        border-radius: 6px;
        font-size: 0.85rem;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 0.4rem;
        transition: all 0.2s;
        font-weight: 600;
    }

    .btn-danger-sm:hover {
        background: #ef4444;
        color: white;
        border-color: #ef4444;
    }

    @keyframes fade-in {
        from {
            opacity: 0;
            transform: translateY(-5px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }

    /* Toolbar */
    .toolbar {
        padding: 1rem 1.5rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.08);
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 1rem;
    }

    :global([data-theme="light"]) .toolbar {
        border-bottom-color: rgba(0, 0, 0, 0.06);
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

    .toolbar-actions {
        display: inline-flex;
        align-items: center;
        gap: 0.75rem;
        flex-shrink: 0;
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
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
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

    .file-card.selected {
        border-color: var(--color-primary);
        background-color: var(--bg-active);
        box-shadow: 0 0 0 2px var(--color-primary-subtle);
    }

    .selection-checkbox {
        position: absolute;
        top: 10px;
        left: 10px;
        z-index: 10;
    }

    .file-card:hover {
        transform: translateY(-2px);
        box-shadow: var(--shadow-md);
        border-color: var(--color-primary);
    }

    .file-preview {
        width: 100%;
        height: 120px; /* Fixed height for consistency */
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-bottom: 1rem;
        background-color: var(--bg-app); /* Fallback bg */
    }

    .file-preview-image {
        width: 100%;
        height: 120px;
        margin-bottom: 1rem;
        border-radius: 12px;
        overflow: hidden;
        background-color: var(--bg-app);
    }

    .file-preview-image img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
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
        flex-shrink: 0;
    }

    .list-thumbnail {
        width: 32px;
        height: 32px;
        border-radius: 6px;
        object-fit: cover;
        flex-shrink: 0;
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

    .uppercase {
        text-transform: uppercase;
    }
    .text-right {
        text-align: right;
    }

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
    .icon-purple {
        background: rgba(168, 85, 247, 0.1);
        color: #a855f7;
    }
    .icon-pink {
        background: rgba(236, 72, 153, 0.1);
        color: #ec4899;
    }
    .icon-cyan {
        background: rgba(6, 182, 212, 0.1);
        color: #06b6d4;
    }
    .icon-red {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
    }
    .icon-orange {
        background: rgba(249, 115, 22, 0.1);
        color: #f97316;
    }
    .icon-blue {
        background: rgba(59, 130, 246, 0.1);
        color: #3b82f6;
    }

    /* States */
    .loading-state,
    .empty-state {
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

    .spin {
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    /* Pagination */
    .pagination-footer {
        padding: 1rem 1.5rem;
        border-top: 1px solid rgba(255, 255, 255, 0.08);
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: rgba(255, 255, 255, 0.015);
    }

    :global([data-theme="light"]) .pagination-footer {
        border-top-color: rgba(0, 0, 0, 0.06);
        background: rgba(0, 0, 0, 0.015);
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

    @media (max-width: 768px) {
        .page-container {
            padding: 1rem 1.25rem;
            gap: 1rem;
            width: 100%;
            overflow-x: hidden;
        }

        .glass-card {
            max-width: 100%;
        }

        .page-header {
            flex-direction: column;
            align-items: flex-start;
            gap: 1rem;
        }

        .header-actions {
            width: 100%;
            justify-content: space-between;
        }

        .filter-cards {
            grid-template-columns: repeat(2, minmax(0, 1fr));
            gap: 0.75rem;
        }

        .filter-card {
            padding: 0.85rem 0.95rem;
            border-radius: 16px;
        }

        .fc-icon {
            width: 40px;
            height: 40px;
            border-radius: 12px;
        }

        .fc-title {
            font-size: 0.9rem;
        }

        .fc-value {
            font-size: 1rem;
        }

        .toolbar {
            flex-direction: column;
            align-items: stretch;
            padding: 1rem;
        }

        .action-bar {
            flex-direction: column;
            align-items: stretch;
            gap: 0.75rem;
        }

        .selection-count {
            flex-wrap: wrap;
        }

        .action-buttons {
            justify-content: flex-end;
            flex-wrap: wrap;
        }

        .toolbar-actions {
            width: 100%;
            justify-content: space-between;
            flex-wrap: wrap;
            gap: 0.5rem;
        }

        .toolbar-actions :global(.btn) {
            flex: 1 1 auto;
        }

        .stats-badge {
            flex: 1 1 auto;
            justify-content: center;
        }

        .search-box {
            max-width: none;
        }

        .view-toggles {
            display: none; /* Hide toggles on mobile */
        }

        .list-view {
            overflow-x: auto;
            display: block;
            width: 100%;
        }

        .file-table {
            min-width: 600px; /* Force scroll */
        }

        /* Ensure browser area expands but contains overflow */
        .browser-area {
            padding: 0.75rem;
        }

        .grid-view {
            grid-template-columns: repeat(2, minmax(0, 1fr));
            gap: 0.5rem;
        }

        .file-card {
            padding: 0.5rem;
        }

        .file-preview {
            height: 100px;
            margin-bottom: 0.5rem;
        }

        .file-preview-image {
            height: 100px;
            margin-bottom: 0.5rem;
        }

        .file-name {
            font-size: 0.8rem;
        }

        .file-meta {
            font-size: 0.7rem;
        }

        .pagination-footer {
            flex-direction: column;
            gap: 1rem;
            text-align: center;
            padding: 1rem;
        }
    }
</style>

