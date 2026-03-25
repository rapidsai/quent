// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import * as AccordionPrimitive from '@radix-ui/react-accordion';
import { ChevronRight } from 'lucide-react';
import { cva } from 'class-variance-authority';
import { cn } from '@/lib/utils';

const treeVariants = cva('group px-2 rounded-sm transition-all duration-150 hover:bg-secondary/70');

const treeNodeVariants = cva('group px-2 rounded-sm transition-all duration-150');

const selectedTreeVariants = cva('bg-secondary/70 text-foreground font-medium');

const dragOverVariants = cva('before:opacity-100 before:bg-primary/20 text-primary-foreground');

interface TreeDataItem {
  id: string;
  name: string;
  icon?: React.ComponentType<{ className?: string }>;
  selectedIcon?: React.ComponentType<{ className?: string }>;
  openIcon?: React.ComponentType<{ className?: string }>;
  children?: TreeDataItem[];
  actions?: React.ReactNode;
  onClick?: () => void;
  draggable?: boolean;
  droppable?: boolean;
  disabled?: boolean;
  className?: string;
}

type TreeRenderItemParams<T extends TreeDataItem = TreeDataItem> = {
  item: T;
  level: number;
  isLeaf: boolean;
  isSelected: boolean;
  isOpen?: boolean;
  hasChildren: boolean;
};

type TreeProps<T extends TreeDataItem = TreeDataItem> = React.HTMLAttributes<HTMLDivElement> & {
  data: T[] | T;
  initialSelectedItemId?: string;
  onSelectChange?: (item: T | undefined) => void;
  expandAll?: boolean;
  defaultNodeIcon?: React.ComponentType<{ className?: string }>;
  defaultLeafIcon?: React.ComponentType<{ className?: string }>;
  onDocumentDrag?: (sourceItem: T, targetItem: T) => void;
  renderItem?: (params: TreeRenderItemParams<T>) => React.ReactNode;
};

function TreeView<T extends TreeDataItem = TreeDataItem>({
  data,
  initialSelectedItemId,
  onSelectChange,
  expandAll,
  defaultLeafIcon,
  defaultNodeIcon,
  className,
  onDocumentDrag,
  renderItem,
  ...props
}: TreeProps<T>) {
  const [selectedItemId, setSelectedItemId] = React.useState<string | undefined>(
    initialSelectedItemId
  );

  const [draggedItem, setDraggedItem] = React.useState<T | null>(null);

  const handleSelectChange = React.useCallback(
    (item: T | undefined) => {
      setSelectedItemId(item?.id);
      if (onSelectChange) {
        onSelectChange(item);
      }
    },
    [onSelectChange]
  );

  const handleDragStart = React.useCallback((item: T) => {
    setDraggedItem(item);
  }, []);

  const handleDrop = React.useCallback(
    (targetItem: T) => {
      if (draggedItem && onDocumentDrag && draggedItem.id !== targetItem.id) {
        onDocumentDrag(draggedItem, targetItem);
      }
      setDraggedItem(null);
    },
    [draggedItem, onDocumentDrag]
  );

  const expandedItemIds = React.useMemo(() => {
    if (!initialSelectedItemId) {
      return [] as string[];
    }

    const ids: string[] = [];

    function walkTreeItems(items: T[] | T, targetId: string): boolean | undefined {
      if (Array.isArray(items)) {
        for (let i = 0; i < items.length; i++) {
          ids.push(items[i].id);
          if (walkTreeItems(items[i], targetId) && !expandAll) {
            return true;
          }
          if (!expandAll) ids.pop();
        }
      } else if (!expandAll && items.id === targetId) {
        return true;
      } else if (items.children) {
        return walkTreeItems(items.children as T[], targetId);
      }
    }

    walkTreeItems(data, initialSelectedItemId);
    return ids;
  }, [data, expandAll, initialSelectedItemId]);

  return (
    <div className={cn('overflow-hidden relative', className)}>
      <TreeItem
        data={data}
        selectedItemId={selectedItemId}
        handleSelectChange={handleSelectChange}
        expandedItemIds={expandedItemIds}
        defaultLeafIcon={defaultLeafIcon}
        defaultNodeIcon={defaultNodeIcon}
        handleDragStart={handleDragStart}
        handleDrop={handleDrop}
        draggedItem={draggedItem}
        renderItem={renderItem}
        level={0}
        {...props}
      />
    </div>
  );
}

TreeView.displayName = 'TreeView';

type TreeItemProps<T extends TreeDataItem = TreeDataItem> = TreeProps<T> & {
  selectedItemId?: string;
  handleSelectChange: (item: T | undefined) => void;
  expandedItemIds: string[];
  defaultNodeIcon?: React.ComponentType<{ className?: string }>;
  defaultLeafIcon?: React.ComponentType<{ className?: string }>;
  handleDragStart?: (item: T) => void;
  handleDrop?: (item: T) => void;
  draggedItem: T | null;
  level?: number;
};

function TreeItem<T extends TreeDataItem = TreeDataItem>({
  className,
  data,
  selectedItemId,
  handleSelectChange,
  expandedItemIds,
  defaultNodeIcon,
  defaultLeafIcon,
  handleDragStart,
  handleDrop,
  draggedItem,
  renderItem,
  level,
  ...props
}: TreeItemProps<T>) {
  let items: T[];
  if (!Array.isArray(data)) {
    items = [data];
  } else {
    items = data;
  }
  return (
    <div role="tree" className={className} {...props}>
      <ul>
        {items.map((item: T) => (
          <li key={item.id}>
            {item.children ? (
              <TreeNode
                item={item}
                level={level ?? 0}
                selectedItemId={selectedItemId}
                expandedItemIds={expandedItemIds}
                handleSelectChange={handleSelectChange}
                defaultNodeIcon={defaultNodeIcon}
                defaultLeafIcon={defaultLeafIcon}
                handleDragStart={handleDragStart}
                handleDrop={handleDrop}
                draggedItem={draggedItem}
                renderItem={renderItem}
              />
            ) : (
              <TreeLeaf
                item={item}
                level={level ?? 0}
                selectedItemId={selectedItemId}
                handleSelectChange={handleSelectChange}
                defaultLeafIcon={defaultLeafIcon}
                handleDragStart={handleDragStart}
                handleDrop={handleDrop}
                draggedItem={draggedItem}
                renderItem={renderItem}
              />
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

TreeItem.displayName = 'TreeItem';

function TreeNode<T extends TreeDataItem = TreeDataItem>({
  item,
  handleSelectChange,
  expandedItemIds,
  selectedItemId,
  defaultNodeIcon,
  defaultLeafIcon,
  handleDragStart,
  handleDrop,
  draggedItem,
  renderItem,
  level = 0,
}: {
  item: T;
  handleSelectChange: (item: T | undefined) => void;
  expandedItemIds: string[];
  selectedItemId?: string;
  defaultNodeIcon?: React.ComponentType<{ className?: string }>;
  defaultLeafIcon?: React.ComponentType<{ className?: string }>;
  handleDragStart?: (item: T) => void;
  handleDrop?: (item: T) => void;
  draggedItem: T | null;
  renderItem?: (params: TreeRenderItemParams<T>) => React.ReactNode;
  level?: number;
}) {
  const [value, setValue] = React.useState(expandedItemIds.includes(item.id) ? [item.id] : []);
  const [isDragOver, setIsDragOver] = React.useState(false);
  const hasChildren = !!item.children?.length;
  const isSelected = selectedItemId === item.id;
  const isOpen = value.includes(item.id);

  const onDragStart = (e: React.DragEvent) => {
    if (!item.draggable) {
      e.preventDefault();
      return;
    }
    e.dataTransfer.setData('text/plain', item.id);
    handleDragStart?.(item);
  };

  const onDragOver = (e: React.DragEvent) => {
    if (item.droppable !== false && draggedItem && draggedItem.id !== item.id) {
      e.preventDefault();
      setIsDragOver(true);
    }
  };

  const onDragLeave = () => {
    setIsDragOver(false);
  };

  const onDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    handleDrop?.(item);
  };

  return (
    <AccordionPrimitive.Root type="multiple" value={value} onValueChange={s => setValue(s)}>
      <AccordionPrimitive.Item value={item.id}>
        <AccordionTrigger
          className={cn(
            treeNodeVariants(),
            isSelected && selectedTreeVariants(),
            isDragOver && dragOverVariants(),
            item.className
          )}
          draggable={!!item.draggable}
          onDragStart={onDragStart}
          onDragOver={onDragOver}
          onDragLeave={onDragLeave}
          onDrop={onDrop}
        >
          <div
            className="flex items-center flex-1"
            onClick={e => {
              e.stopPropagation();
              handleSelectChange(item);
              item.onClick?.();
            }}
          >
            {renderItem ? (
              renderItem({
                item,
                level,
                isLeaf: false,
                isSelected,
                isOpen,
                hasChildren,
              })
            ) : (
              <>
                <TreeIcon
                  item={item}
                  isSelected={isSelected}
                  isOpen={isOpen}
                  default={defaultNodeIcon}
                />
                <span className="text-sm truncate">{item.name}</span>
                <TreeActions isSelected={isSelected}>{item.actions}</TreeActions>
              </>
            )}
          </div>
        </AccordionTrigger>
        <AccordionContent className="ml-4 pl-1 border-l border-border/70">
          <TreeItem
            data={item.children ? (item.children as T[]) : item}
            selectedItemId={selectedItemId}
            handleSelectChange={handleSelectChange}
            expandedItemIds={expandedItemIds}
            defaultLeafIcon={defaultLeafIcon}
            defaultNodeIcon={defaultNodeIcon}
            handleDragStart={handleDragStart}
            handleDrop={handleDrop}
            draggedItem={draggedItem}
            renderItem={renderItem}
            level={level + 1}
          />
        </AccordionContent>
      </AccordionPrimitive.Item>
    </AccordionPrimitive.Root>
  );
}

function TreeLeaf<T extends TreeDataItem = TreeDataItem>({
  className,
  item,
  level,
  selectedItemId,
  handleSelectChange,
  defaultLeafIcon,
  handleDragStart,
  handleDrop,
  draggedItem,
  renderItem,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & {
  item: T;
  level: number;
  selectedItemId?: string;
  handleSelectChange: (item: T | undefined) => void;
  defaultLeafIcon?: React.ComponentType<{ className?: string }>;
  handleDragStart?: (item: T) => void;
  handleDrop?: (item: T) => void;
  draggedItem: T | null;
  renderItem?: (params: TreeRenderItemParams<T>) => React.ReactNode;
}) {
  const [isDragOver, setIsDragOver] = React.useState(false);
  const isSelected = selectedItemId === item.id;

  const onDragStart = (e: React.DragEvent) => {
    if (!item.draggable || item.disabled) {
      e.preventDefault();
      return;
    }
    e.dataTransfer.setData('text/plain', item.id);
    handleDragStart?.(item);
  };

  const onDragOver = (e: React.DragEvent) => {
    if (item.droppable !== false && !item.disabled && draggedItem && draggedItem.id !== item.id) {
      e.preventDefault();
      setIsDragOver(true);
    }
  };

  const onDragLeave = () => {
    setIsDragOver(false);
  };

  const onDrop = (e: React.DragEvent) => {
    if (item.disabled) return;
    e.preventDefault();
    setIsDragOver(false);
    handleDrop?.(item);
  };

  return (
    <div
      className={cn(
        'ml-5 flex text-left items-center py-2 cursor-pointer before:right-1',
        treeVariants(),
        className,
        isSelected && selectedTreeVariants(),
        isDragOver && dragOverVariants(),
        item.disabled && 'opacity-50 cursor-not-allowed pointer-events-none',
        item.className
      )}
      onClick={() => {
        if (item.disabled) return;
        handleSelectChange(item);
        item.onClick?.();
      }}
      draggable={!!item.draggable && !item.disabled}
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
      {...props}
    >
      {renderItem ? (
        <>
          {item.children?.length && <div className="h-4 w-4 shrink-0 mr-1" />}
          {renderItem({
            item,
            level,
            isLeaf: true,
            isSelected,
            hasChildren: false,
          })}
        </>
      ) : (
        <>
          <TreeIcon item={item} isSelected={isSelected} default={defaultLeafIcon} />
          <span className="flex-grow text-sm truncate">{item.name}</span>
          <TreeActions isSelected={isSelected && !item.disabled}>{item.actions}</TreeActions>
        </>
      )}
    </div>
  );
}

TreeLeaf.displayName = 'TreeLeaf';

const AccordionTrigger = React.forwardRef<
  React.ElementRef<typeof AccordionPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Trigger>
>(({ className, children, ...props }, ref) => (
  <AccordionPrimitive.Header>
    <AccordionPrimitive.Trigger
      ref={ref}
      className={cn(
        'flex flex-1 w-full items-center py-2 transition-all first:[&[data-state=open]>svg]:first-of-type:rotate-90',
        className
      )}
      {...props}
    >
      <ChevronRight className="h-4 w-4 shrink-0 transition-transform duration-200 text-accent-foreground/50 mr-1" />
      {children}
    </AccordionPrimitive.Trigger>
  </AccordionPrimitive.Header>
));
AccordionTrigger.displayName = AccordionPrimitive.Trigger.displayName;

const AccordionContent = React.forwardRef<
  React.ElementRef<typeof AccordionPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Content>
>(({ className, children, ...props }, ref) => (
  <AccordionPrimitive.Content
    ref={ref}
    className={cn(
      'overflow-hidden text-sm transition-all data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down',
      className
    )}
    {...props}
  >
    <div className="pb-1 pt-0">{children}</div>
  </AccordionPrimitive.Content>
));
AccordionContent.displayName = AccordionPrimitive.Content.displayName;

const TreeIcon = ({
  item,
  isOpen,
  isSelected,
  default: defaultIcon,
}: {
  item: TreeDataItem;
  isOpen?: boolean;
  isSelected?: boolean;
  default?: React.ComponentType<{ className?: string }>;
}) => {
  let Icon: React.ComponentType<{ className?: string }> | undefined = defaultIcon;
  if (isSelected && item.selectedIcon) {
    Icon = item.selectedIcon;
  } else if (isOpen && item.openIcon) {
    Icon = item.openIcon;
  } else if (item.icon) {
    Icon = item.icon;
  }
  return Icon ? <Icon className="h-4 w-4 shrink-0 mr-2" /> : <></>;
};

const TreeActions = ({
  children,
  isSelected,
}: {
  children: React.ReactNode;
  isSelected: boolean;
}) => {
  return (
    <div className={cn(isSelected ? 'block' : 'hidden', 'absolute right-3 group-hover:block')}>
      {children}
    </div>
  );
};

export {
  TreeView,
  type TreeDataItem,
  type TreeRenderItemParams,
  AccordionTrigger,
  AccordionContent,
  TreeLeaf,
  TreeNode,
  TreeItem,
};
