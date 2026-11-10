// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState } from 'react';
import { Bookmark, Check, Pencil, Save as SaveIcon, Trash2, X } from 'lucide-react';
import { Button, Input, Popover, PopoverContent, PopoverTrigger, toast } from '@quent/components';
import type { DeepLinkTab } from '@/features/deep-link/deepLink.schema';
import { useDeepLink } from '@/features/deep-link';
import { showDeepLinkErrorToasts } from '@/features/deep-link/deepLink.toasts';
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
  editing: boolean;
  confirmingDelete: boolean;
  editName: string;
  canLoad: boolean;
  canUpdate: boolean;
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
  editing,
  confirmingDelete,
  editName,
  canLoad,
  canUpdate,
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
    <div className="flex min-w-0 items-center gap-1 rounded-sm border px-1 py-1">
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
          <button
            type="button"
            className="min-w-0 flex-1 rounded-sm px-1.5 py-1 text-left hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
            aria-label={`Load ${view.name} (${SAVED_VIEW_TAB_LABELS[view.tab]})`}
            disabled={!canLoad}
            onClick={onLoad}
          >
            <span className="block truncate text-sm">{view.name}</span>
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

export function SavedViewsMenu({ engineId, queryId, activeTab }: SavedViewsMenuProps) {
  const deepLink = useDeepLink();
  const { result, save, update, rename, remove, reset } = useSavedViews({ engineId, queryId });
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const views = useMemo(
    () => (result.ok ? [...result.value.views].sort((a, b) => b.updatedAt - a.updatedAt) : []),
    [result]
  );
  const atLimit = views.length >= MAX_SAVED_VIEWS_PER_QUERY;
  const unavailable = !result.ok && result.kind === 'unavailable';

  const onOpenChange = (next: boolean) => {
    setOpen(next);
    if (next) {
      setName(
        defaultSavedViewName(
          activeTab,
          new Date(),
          views.map(view => view.name)
        )
      );
    } else {
      setEditingId(null);
      setConfirmDeleteId(null);
    }
  };

  const saveCurrentView = () => {
    if (!result.ok || atLimit || !name.trim() || !deepLink) {
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
    toast.add({ type: 'success', title: 'View saved', description: name.trim() });
    onOpenChange(false);
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
      toast.add({ type: 'success', title: 'View updated', description: view.name });
      onOpenChange(false);
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
    <Popover open={open} onOpenChange={onOpenChange}>
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
      <PopoverContent className="w-[min(24rem,calc(100vw-2rem))] p-3">
        <div className="space-y-3">
          <header>
            <h2 className="text-sm font-medium">Saved views</h2>
            <p className="text-xs text-muted-foreground">Saved in this browser for this query.</p>
          </header>
          {result.ok ? (
            <div className="flex gap-2">
              <Input
                value={name}
                maxLength={MAX_SAVED_VIEW_NAME_LENGTH}
                aria-label="Saved view name"
                onChange={event => setName(event.target.value)}
                onFocus={event => event.currentTarget.select()}
                onKeyDown={event => event.key === 'Enter' && saveCurrentView()}
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
            </div>
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
            <div className="max-h-64 space-y-1 overflow-y-auto">
              {views.length === 0 ? (
                <p className="py-4 text-center text-xs text-muted-foreground">
                  No saved views for this query.
                </p>
              ) : (
                views.map(view => (
                  <SavedViewRow
                    key={view.id}
                    view={view}
                    editing={editingId === view.id}
                    confirmingDelete={confirmDeleteId === view.id}
                    editName={editName}
                    canLoad={deepLink?.loadSavedView !== undefined}
                    canUpdate={deepLink !== null}
                    onEditName={setEditName}
                    onLoad={() => {
                      deepLink?.loadSavedView?.(view);
                      onOpenChange(false);
                    }}
                    onUpdate={() => updateCurrentView(view)}
                    onBeginRename={() => {
                      setEditingId(view.id);
                      setEditName(view.name);
                      setConfirmDeleteId(null);
                    }}
                    onCommitRename={() => commitRename(view)}
                    onCancelRename={() => setEditingId(null)}
                    onBeginDelete={() => {
                      setConfirmDeleteId(view.id);
                      setEditingId(null);
                    }}
                    onDelete={() => deleteView(view)}
                    onCancelDelete={() => setConfirmDeleteId(null)}
                  />
                ))
              )}
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
