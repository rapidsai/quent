// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useMemo, useRef, useState, type Ref } from 'react';
import { Bookmark, Check, Pencil, Save as SaveIcon, Trash2, X } from 'lucide-react';
import {
  Button,
  Input,
  Popover,
  PopoverContent,
  PopoverTrigger,
  ThinScroll,
  toast,
} from '@quent/components';
import { cn } from '@quent/utils';
import type { DeepLinkTab } from '@/features/deep-link/deepLink.schema';
import { useDeepLink } from '@/features/deep-link';
import { showDeepLinkErrorToasts } from '@/features/deep-link/deepLink.toasts';
import { isSavedViewModified } from './savedViews.modified';
import { defaultSavedViewName, SAVED_VIEW_TAB_LABELS } from './savedViews.names';
import {
  MAX_SAVED_VIEW_NAME_LENGTH,
  MAX_SAVED_VIEWS_PER_QUERY,
  type SavedViewV1,
} from './savedViews.schema';
import type { SavedViewsWriteResult } from './savedViews.storage';
import { useSavedViews } from './useSavedViews';

interface SavedViewsMenuProps {
  engineId: string;
  queryId: string;
  activeTab: DeepLinkTab;
  selectedViewId?: string;
  onSelectView: (viewId: string) => void;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function showWriteError(result: Exclude<SavedViewsWriteResult, { ok: true }>) {
  toast.add({
    type: 'error',
    title: 'Could not update saved views',
    description: result.message,
  });
}

interface SavedViewRowProps {
  view: SavedViewV1;
  selected: boolean;
  modified: boolean;
  editing: boolean;
  confirmingDelete: boolean;
  editName: string;
  canLoad: boolean;
  canUpdate: boolean;
  loadButtonRef?: Ref<HTMLButtonElement>;
  onEditName: (name: string) => void;
  onLoad: () => void;
  onUpdate: () => void;
  onBeginRename: () => void;
  onCommitRename: () => void;
  onCancelRename: () => void;
  onBeginDelete: () => void;
  onDelete: () => void;
  onCancelDelete: () => void;
}

function SavedViewRow({
  view,
  selected,
  modified,
  editing,
  confirmingDelete,
  editName,
  canLoad,
  canUpdate,
  loadButtonRef,
  onEditName,
  onLoad,
  onUpdate,
  onBeginRename,
  onCommitRename,
  onCancelRename,
  onBeginDelete,
  onDelete,
  onCancelDelete,
}: SavedViewRowProps) {
  return (
    <div
      role="listitem"
      aria-current={selected ? 'true' : undefined}
      className={cn(
        'flex min-w-0 items-center gap-1 rounded-sm border px-1 py-1 hover:bg-accent',
        selected && 'bg-accent'
      )}
    >
      {editing ? (
        <>
          <Input
            autoFocus
            value={editName}
            maxLength={MAX_SAVED_VIEW_NAME_LENGTH}
            aria-label={`Rename ${view.name}`}
            className="h-7 min-w-0"
            onChange={event => onEditName(event.target.value)}
            onKeyDown={event => {
              if (event.key === 'Enter') {
                onCommitRename();
              } else if (event.key === 'Escape') {
                onCancelRename();
              }
            }}
          />
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            aria-label="Save rename"
            disabled={!editName.trim()}
            onClick={onCommitRename}
          >
            <Check className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            aria-label="Cancel rename"
            onClick={onCancelRename}
          >
            <X className="size-3.5" />
          </Button>
        </>
      ) : confirmingDelete ? (
        <div role="alert" className="flex min-w-0 flex-1 items-center gap-1">
          <span className="min-w-0 flex-1 truncate px-1 text-xs">Delete “{view.name}”?</span>
          <Button autoFocus type="button" variant="destructive" size="xs" onClick={onDelete}>
            Delete
          </Button>
          <Button type="button" variant="ghost" size="xs" onClick={onCancelDelete}>
            Cancel
          </Button>
        </div>
      ) : (
        <>
          <span className="flex size-4 shrink-0 items-center justify-center" aria-hidden="true">
            {selected && <Check className="size-3.5" />}
          </span>
          {selected && (
            <span id={`saved-view-${view.id}-selection`} className="sr-only">
              {modified ? 'Currently selected, modified since loaded' : 'Currently selected'}
            </span>
          )}
          <button
            ref={loadButtonRef}
            type="button"
            className="min-w-0 flex-1 cursor-pointer rounded-sm px-1.5 py-1 text-left disabled:cursor-not-allowed disabled:opacity-50"
            aria-label={`Load ${view.name} (${SAVED_VIEW_TAB_LABELS[view.tab]})`}
            aria-describedby={selected ? `saved-view-${view.id}-selection` : undefined}
            disabled={!canLoad}
            onClick={onLoad}
          >
            <span className="block truncate text-sm">
              {view.name}
              {modified && (
                <span aria-hidden="true" title="Modified since loaded">
                  *
                </span>
              )}
            </span>
            <span className="block text-xs text-muted-foreground">
              {SAVED_VIEW_TAB_LABELS[view.tab]}
            </span>
          </button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            aria-label={`Rename ${view.name}`}
            onClick={onBeginRename}
          >
            <Pencil className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            aria-label={`Update ${view.name}`}
            title="Update with current view"
            disabled={!canUpdate}
            onClick={onUpdate}
          >
            <SaveIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            aria-label={`Delete ${view.name}`}
            onClick={onBeginDelete}
          >
            <Trash2 className="size-3.5" />
          </Button>
        </>
      )}
    </div>
  );
}

export function SavedViewsMenu({
  engineId,
  queryId,
  activeTab,
  selectedViewId,
  onSelectView,
  open,
  onOpenChange,
}: SavedViewsMenuProps) {
  const deepLink = useDeepLink();
  const { result, save, update, rename, remove, reset } = useSavedViews({ engineId, queryId });
  // Loading a view remounts this menu (the route re-keys its store provider). If we
  // mount already open, skip the open animation so the menu doesn't flash.
  const [mountedOpen, setMountedOpen] = useState(open);
  const [savingNewView, setSavingNewView] = useState(false);
  const [name, setName] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const saveActionRef = useRef<HTMLButtonElement>(null);
  const selectedViewButtonRef = useRef<HTMLButtonElement>(null);
  const restoreSaveActionFocusRef = useRef(false);
  const views = useMemo(
    () => (result.ok ? [...result.value.views].sort((a, b) => b.updatedAt - a.updatedAt) : []),
    [result]
  );
  const selectedView = views.find(view => view.id === selectedViewId);
  // Only compared while the menu is open; the capture is skipped otherwise.
  const selectedViewModified = useMemo(
    () =>
      open && selectedView && deepLink
        ? isSavedViewModified(selectedView, deepLink.captureSavedView())
        : false,
    [open, selectedView, deepLink]
  );
  const atLimit = views.length >= MAX_SAVED_VIEWS_PER_QUERY;
  const unavailable = !result.ok && result.kind === 'unavailable';

  useEffect(() => {
    if (!savingNewView && restoreSaveActionFocusRef.current) {
      restoreSaveActionFocusRef.current = false;
      saveActionRef.current?.focus();
    }
  }, [savingNewView]);

  const handleOpenChange = (next: boolean) => {
    onOpenChange(next);
    if (!next) {
      setMountedOpen(false);
      setSavingNewView(false);
      setName('');
      setEditingId(null);
      setConfirmDeleteId(null);
    }
  };

  const beginSavingNewView = () => {
    setName(
      defaultSavedViewName(
        activeTab,
        new Date(),
        views.map(view => view.name)
      )
    );
    setSavingNewView(true);
    setEditingId(null);
    setConfirmDeleteId(null);
  };

  const cancelSavingNewView = (restoreFocus = false) => {
    restoreSaveActionFocusRef.current = restoreFocus;
    setSavingNewView(false);
    setName('');
  };

  const saveCurrentView = () => {
    if (!savingNewView || !result.ok || atLimit || !name.trim() || !deepLink) {
      return;
    }
    const captured = deepLink.captureSavedView();
    if (!captured.ok) {
      showDeepLinkErrorToasts({
        id: 'saved-view-capture-error',
        title: 'Could not save view',
        messages: captured.messages,
      });
      return;
    }
    const write = save({
      name,
      tab: captured.tab,
      payload: captured.payload,
      extras: captured.extras,
    });
    if (!write.ok) {
      showWriteError(write);
      return;
    }
    onSelectView(write.id);
    toast.add({ type: 'success', title: 'View saved', description: name.trim() });
    cancelSavingNewView(true);
  };

  const commitRename = (view: SavedViewV1) => {
    if (!editName.trim()) {
      return;
    }
    const write = rename(view.id, editName);
    if (!write.ok) {
      showWriteError(write);
    } else {
      setEditingId(null);
    }
  };

  const updateCurrentView = (view: SavedViewV1) => {
    if (!deepLink) {
      return;
    }
    const captured = deepLink.captureSavedView();
    if (!captured.ok) {
      showDeepLinkErrorToasts({
        id: 'saved-view-update-error',
        title: 'Could not update view',
        messages: captured.messages,
      });
      return;
    }
    const write = update(view.id, {
      tab: captured.tab,
      payload: captured.payload,
      extras: captured.extras,
    });
    if (!write.ok) {
      showWriteError(write);
    } else {
      onSelectView(view.id);
      toast.add({ type: 'success', title: 'View updated', description: view.name });
    }
  };

  const deleteView = (view: SavedViewV1) => {
    const write = remove(view.id);
    if (!write.ok) {
      showWriteError(write);
    } else {
      setConfirmDeleteId(null);
    }
  };

  return (
    <Popover open={open} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8 gap-1.5 px-2"
          aria-label="Saved views"
          title={unavailable ? result.message : 'Saved views'}
          disabled={unavailable}
        >
          <Bookmark className="size-4" />
          <span className="hidden sm:inline">Views</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent
        className={cn(
          'w-[min(24rem,calc(100vw-2rem))] p-3',
          mountedOpen && 'data-[state=open]:animate-none!'
        )}
        onOpenAutoFocus={event => {
          if (selectedViewButtonRef.current) {
            event.preventDefault();
            selectedViewButtonRef.current.focus();
          }
        }}
        onEscapeKeyDown={event => {
          if (savingNewView) {
            event.preventDefault();
            cancelSavingNewView(true);
          }
        }}
      >
        <div className="space-y-3">
          <header>
            <h2 className="text-sm font-medium">Saved views</h2>
            <p className="text-xs text-muted-foreground">Saved in this browser for this query.</p>
          </header>
          {result.ok ? (
            savingNewView ? (
              <div className="flex gap-2">
                <Input
                  autoFocus
                  value={name}
                  maxLength={MAX_SAVED_VIEW_NAME_LENGTH}
                  aria-label="Saved view name"
                  onChange={event => setName(event.target.value)}
                  onFocus={event => event.currentTarget.select()}
                  onKeyDown={event => {
                    if (event.key === 'Enter') {
                      // Focus moves to "Save current view" after saving; without this the
                      // same Enter press would click it and reopen the form.
                      event.preventDefault();
                      saveCurrentView();
                    }
                  }}
                />
                <Button
                  type="button"
                  size="sm"
                  disabled={atLimit || !name.trim()}
                  aria-describedby={atLimit ? 'saved-views-limit-message' : undefined}
                  onClick={saveCurrentView}
                >
                  Save
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-9 shrink-0"
                  aria-label="Cancel saving view"
                  onClick={() => cancelSavingNewView(true)}
                >
                  <X className="size-4" />
                </Button>
              </div>
            ) : (
              <Button
                ref={saveActionRef}
                type="button"
                variant="ghost"
                size="sm"
                className="w-full"
                disabled={atLimit}
                aria-describedby={atLimit ? 'saved-views-limit-message' : undefined}
                onClick={beginSavingNewView}
              >
                <SaveIcon className="size-4" />
                Save current view
              </Button>
            )
          ) : (
            <div role="alert" className="rounded-sm border p-2 text-xs text-muted-foreground">
              <p>{result.message}</p>
              {result.kind === 'corrupt' && (
                <Button
                  type="button"
                  variant="ghost"
                  size="xs"
                  className="mt-1"
                  onClick={() => {
                    const write = reset();
                    if (!write.ok) {
                      showWriteError(write);
                    }
                  }}
                >
                  Reset saved views
                </Button>
              )}
            </div>
          )}
          {atLimit && (
            <p id="saved-views-limit-message" className="text-xs text-muted-foreground">
              Delete a view to save another.
            </p>
          )}
          {result.ok && (
            <ThinScroll role={views.length > 0 ? 'list' : undefined} className="max-h-64 space-y-1">
              {views.length === 0 ? (
                <p className="py-4 text-center text-xs text-muted-foreground">
                  No saved views for this query.
                </p>
              ) : (
                views.map(view => (
                  <SavedViewRow
                    key={view.id}
                    view={view}
                    selected={selectedViewId === view.id}
                    modified={selectedViewId === view.id && selectedViewModified}
                    editing={editingId === view.id}
                    confirmingDelete={confirmDeleteId === view.id}
                    editName={editName}
                    canLoad={deepLink?.loadSavedView !== undefined}
                    canUpdate={deepLink !== null}
                    loadButtonRef={selectedViewId === view.id ? selectedViewButtonRef : undefined}
                    onEditName={setEditName}
                    onLoad={() => {
                      deepLink?.loadSavedView?.(view);
                    }}
                    onUpdate={() => updateCurrentView(view)}
                    onBeginRename={() => {
                      cancelSavingNewView();
                      setEditingId(view.id);
                      setEditName(view.name);
                      setConfirmDeleteId(null);
                    }}
                    onCommitRename={() => commitRename(view)}
                    onCancelRename={() => setEditingId(null)}
                    onBeginDelete={() => {
                      cancelSavingNewView();
                      setConfirmDeleteId(view.id);
                      setEditingId(null);
                    }}
                    onDelete={() => deleteView(view)}
                    onCancelDelete={() => setConfirmDeleteId(null)}
                  />
                ))
              )}
            </ThinScroll>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
