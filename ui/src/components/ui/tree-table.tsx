import React, { useEffect, useLayoutEffect, useMemo, useRef, useState, useCallback } from 'react';
import * as AccordionPrimitive from '@radix-ui/react-accordion';
import { ChevronRight } from 'lucide-react';
import { cva } from 'class-variance-authority';
import { cn } from '@/lib/utils';

// Tree-table specific styling variants
const treeVariants = cva(
  'group hover:before:opacity-100 before:absolute before:left-0 before:w-full before:opacity-0 before:h-[2rem] before:-z-10'
);

const selectedTreeVariants = cva('before:opacity-100 text-accent-foreground');

export type IconComponent = React.ComponentType<{ className?: string }>;
interface TreeTableDataItem {
  id: string;
  name?: string;
  icon?: IconComponent;
  selectedIcon?: IconComponent;
  openIcon?: IconComponent;
  children?: TreeTableDataItem[];
  actions?: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
}

type TreeTableRenderItemParams = {
  item: TreeTableDataItem;
  level: number;
  isLeaf: boolean;
  isSelected: boolean;
  isOpen?: boolean;
  hasChildren: boolean;
};

const rowSurfaceClasses =
  'relative cursor-pointer transition-colors hover:bg-accent/10 data-[selected=true]:bg-accent/70';

// Tree-table specific AccordionTrigger with level-based positioning
const AccordionTrigger = React.forwardRef<
  React.ElementRef<typeof AccordionPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof AccordionPrimitive.Trigger> & {
    level?: number;
    isSelected?: boolean;
    isOpen?: boolean;
  }
>(({ className, children, level = 0, isSelected, isOpen, ...props }, ref) => {
  const chevronLeft = 10 + level * 20;

  const selectedAttr = isSelected ? 'true' : 'false';
  const chevronAttr = isOpen ? 'true' : 'false';
  const chevronTransform = isOpen ? 'translateY(-50%) rotate(90deg)' : 'translateY(-50%)';

  return (
    <AccordionPrimitive.Header className="w-full relative">
      <AccordionPrimitive.Trigger
        ref={ref}
        className={cn(
          `group flex items-center py-2 transition-all text-foreground w-full min-w-0 overflow-hidden px-0 relative ${rowSurfaceClasses}`,
          className
        )}
        data-selected={selectedAttr}
        {...props}
      >
        <div className="w-2.5 shrink-0" />
        <ChevronRight
          className="h-4 w-4 shrink-0 transition-transform duration-200 chevron-icon absolute top-1/2 text-muted-foreground"
          data-open={chevronAttr}
          style={{
            left: `${chevronLeft}px`,
            transform: chevronTransform,
          }}
        />
        <div className="ml-6 flex-1 min-w-0 overflow-hidden">{children}</div>
      </AccordionPrimitive.Trigger>
    </AccordionPrimitive.Header>
  );
});
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
  item: TreeTableDataItem;
  isOpen?: boolean;
  isSelected?: boolean;
  default?: IconComponent;
}) => {
  let Icon: IconComponent | undefined = defaultIcon;
  if (isSelected && item.selectedIcon) {
    Icon = item.selectedIcon;
  } else if (isOpen && item.openIcon) {
    Icon = item.openIcon;
  } else if (item.icon) {
    Icon = item.icon;
  }
  return Icon ? <Icon className="h-4 w-4 shrink-0 mr-2" /> : null;
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

// Tree-table specific TreeNode
const TreeNode = ({
  item,
  handleSelectChange,
  expandedItemIds,
  selectedItemId,
  defaultNodeIcon,
  defaultLeafIcon,
  renderItem,
  level = 0,
}: {
  item: TreeTableDataItem;
  handleSelectChange: (item: TreeTableDataItem | undefined) => void;
  expandedItemIds: string[];
  selectedItemId?: string;
  defaultNodeIcon?: IconComponent;
  defaultLeafIcon?: IconComponent;
  renderItem?: (params: TreeTableRenderItemParams) => React.ReactNode;
  level?: number;
}) => {
  // Expand by default if in expandedItemIds OR if item has expanded: true
  const itemExpanded = (item as { expanded?: boolean }).expanded;
  const shouldExpandByDefault = expandedItemIds.includes(item.id) || itemExpanded === true;
  const [value, setValue] = useState(shouldExpandByDefault ? [item.id] : []);
  const hasChildren = !!item.children?.length;
  const isSelected = selectedItemId === item.id;
  const isOpen = value.includes(item.id);

  return (
    <AccordionPrimitive.Root type="multiple" value={value} onValueChange={s => setValue(s)}>
      <AccordionPrimitive.Item value={item.id}>
        <AccordionTrigger
          level={level}
          isSelected={isSelected}
          isOpen={isOpen}
          className={cn(treeVariants(), isSelected && selectedTreeVariants())}
          onClick={() => {
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
              <span className="text-sm truncate text-foreground">{item.name}</span>
              <TreeActions isSelected={isSelected}>{item.actions}</TreeActions>
            </>
          )}
        </AccordionTrigger>
        <AccordionContent className="border-l border-border">
          <TreeItem
            data={item.children ? item.children : item}
            selectedItemId={selectedItemId}
            handleSelectChange={handleSelectChange}
            expandedItemIds={expandedItemIds}
            defaultLeafIcon={defaultLeafIcon}
            defaultNodeIcon={defaultNodeIcon}
            renderItem={renderItem}
            level={level + 1}
          />
        </AccordionContent>
      </AccordionPrimitive.Item>
    </AccordionPrimitive.Root>
  );
};

// Tree-table specific TreeLeaf with spacing divs
const TreeLeaf = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & {
    item: TreeTableDataItem;
    level: number;
    selectedItemId?: string;
    handleSelectChange: (item: TreeTableDataItem | undefined) => void;
    defaultLeafIcon?: IconComponent;
    renderItem?: (params: TreeTableRenderItemParams) => React.ReactNode;
  }
>(
  (
    {
      className,
      item,
      level,
      selectedItemId,
      handleSelectChange,
      defaultLeafIcon,
      renderItem,
      ...props
    },
    ref
  ) => {
    const isSelected = selectedItemId === item.id;
    const dataSelected = isSelected ? 'true' : 'false';

    return (
      <div
        ref={ref}
        className={cn(
          `flex text-left items-center py-2 before:right-1 text-foreground w-full min-w-0 px-0 relative ${rowSurfaceClasses}`,
          treeVariants(),
          className,
          isSelected && selectedTreeVariants(),
          item.disabled && 'opacity-50 cursor-not-allowed pointer-events-none'
        )}
        data-selected={dataSelected}
        onClick={() => {
          if (item.disabled) return;
          handleSelectChange(item);
          item.onClick?.();
        }}
        {...props}
      >
        {renderItem ? (
          <>
            <div className="w-2.5 shrink-0" />
            <div className="w-6 shrink-0" />
            <div className="flex-1 min-w-0 overflow-hidden">
              {renderItem({
                item,
                level,
                isLeaf: true,
                isSelected,
                hasChildren: false,
              })}
            </div>
          </>
        ) : (
          <>
            <TreeIcon item={item} isSelected={isSelected} default={defaultLeafIcon} />
            <span className="flex-grow text-sm truncate text-foreground">{item.name}</span>
            <TreeActions isSelected={isSelected && !item.disabled}>{item.actions}</TreeActions>
          </>
        )}
      </div>
    );
  }
);
TreeLeaf.displayName = 'TreeLeaf';

// Tree-table specific TreeItem that uses custom TreeNode and TreeLeaf
type TreeItemProps = {
  data: TreeTableDataItem[] | TreeTableDataItem;
  selectedItemId?: string;
  handleSelectChange: (item: TreeTableDataItem | undefined) => void;
  expandedItemIds: string[];
  defaultNodeIcon?: IconComponent;
  defaultLeafIcon?: IconComponent;
  renderItem?: (params: TreeTableRenderItemParams) => React.ReactNode;
  level?: number;
  className?: string;
};

const TreeItem = React.forwardRef<HTMLDivElement, TreeItemProps>(
  (
    {
      className,
      data,
      selectedItemId,
      handleSelectChange,
      expandedItemIds,
      defaultNodeIcon,
      defaultLeafIcon,
      renderItem,
      level,
      ...props
    },
    ref
  ) => {
    if (!(data instanceof Array)) {
      data = [data];
    }
    return (
      <div ref={ref} role="tree" className={className} {...props}>
        <ul>
          {data.map(item => (
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
                  renderItem={renderItem}
                />
              ) : (
                <TreeLeaf
                  item={item}
                  level={level ?? 0}
                  selectedItemId={selectedItemId}
                  handleSelectChange={handleSelectChange}
                  defaultLeafIcon={defaultLeafIcon}
                  renderItem={renderItem}
                />
              )}
            </li>
          ))}
        </ul>
      </div>
    );
  }
);
TreeItem.displayName = 'TreeItem';

// Tree-table specific TreeView that uses custom TreeItem
type TreeViewProps = React.HTMLAttributes<HTMLDivElement> & {
  data: TreeTableDataItem[] | TreeTableDataItem;
  initialSelectedItemId?: string;
  onSelectChange?: (item: TreeTableDataItem | undefined) => void;
  expandAll?: boolean;
  defaultNodeIcon?: IconComponent;
  defaultLeafIcon?: IconComponent;
  renderItem?: (params: TreeTableRenderItemParams) => React.ReactNode;
};

const TreeView = React.forwardRef<HTMLDivElement, TreeViewProps>(
  (
    {
      data,
      initialSelectedItemId,
      onSelectChange,
      expandAll,
      defaultLeafIcon,
      defaultNodeIcon,
      className,
      renderItem,
      ...props
    },
    ref
  ) => {
    const [selectedItemId, setSelectedItemId] = useState<string | undefined>(initialSelectedItemId);

    const handleSelectChange = useCallback(
      (item: TreeTableDataItem | undefined) => {
        setSelectedItemId(item?.id);
        if (onSelectChange) {
          onSelectChange(item);
        }
      },
      [onSelectChange]
    );

    const expandedItemIds = useMemo(() => {
      if (!initialSelectedItemId) {
        return [] as string[];
      }

      const ids: string[] = [];

      function walkTreeItems(items: TreeTableDataItem[] | TreeTableDataItem, targetId: string) {
        if (items instanceof Array) {
          for (let i = 0; i < items.length; i++) {
            ids.push(items[i]!.id);
            if (walkTreeItems(items[i]!, targetId) && !expandAll) {
              return true;
            }
            if (!expandAll) ids.pop();
          }
        } else if (!expandAll && items.id === targetId) {
          return true;
        } else if (items.children) {
          return walkTreeItems(items.children, targetId);
        }
      }

      walkTreeItems(data, initialSelectedItemId);
      return ids;
    }, [data, expandAll, initialSelectedItemId]);

    return (
      <div className={cn('overflow-hidden relative bg-transparent w-full min-w-0', className)}>
        <TreeItem
          data={data}
          ref={ref}
          selectedItemId={selectedItemId}
          handleSelectChange={handleSelectChange}
          expandedItemIds={expandedItemIds}
          defaultLeafIcon={defaultLeafIcon}
          defaultNodeIcon={defaultNodeIcon}
          renderItem={renderItem}
          level={0}
          {...props}
        />
      </div>
    );
  }
);
TreeView.displayName = 'TreeView';

export type ColumnComponent<I> = ({
  item,
  level,
  isSelected,
}: {
  item: I;
  level: number;
  isSelected: boolean;
}) => React.ReactNode;

export type Column<I> = {
  key: string;
  label: string;
  widthIndex: number;
  isFirst?: boolean;
  render: ColumnComponent<I>;
};

const useIsomorphicLayoutEffect = typeof window !== 'undefined' ? useLayoutEffect : useEffect;

const DEFAULT_COLUMN_WIDTHS = [280, 500];

export function TreeTable<I extends TreeTableDataItem>({
  data,
  columns,
  columnWidths = DEFAULT_COLUMN_WIDTHS,
  ...treeViewProps
}: {
  data: I[];
  columns: Column<I>[];
  columnWidths?: number[];
} & TreeViewProps) {
  const [treeData, setTreeData] = useState<I[]>(data);
  const [currentColumnWidths, setCurrentColumnWidths] = useState<number[]>(columnWidths);

  useEffect(() => {
    setTreeData(data);
  }, [data]);

  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(0);
  const [isLayoutReady, setIsLayoutReady] = useState(false);
  const [hasUserResized, setHasUserResized] = useState(false);

  const totalWidth = currentColumnWidths.reduce((sum, w) => sum + w, 0);

  useIsomorphicLayoutEffect(() => {
    const node = containerRef.current;
    if (!node) {
      return;
    }

    const updateWidth = () => {
      const next = node.getBoundingClientRect().width;
      setContainerWidth(next);
      if (!isLayoutReady) {
        setIsLayoutReady(true);
      }
    };

    updateWidth();

    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', updateWidth);
      return () => window.removeEventListener('resize', updateWidth);
    }

    const observer = new ResizeObserver(entries => {
      const entry = entries[0];
      if (entry) {
        setContainerWidth(entry.contentRect.width);
        if (!isLayoutReady) {
          setIsLayoutReady(true);
        }
      }
    });

    observer.observe(node);
    return () => observer.disconnect();
  }, [isLayoutReady]);

  const baseContainerWidth = containerWidth || 0;
  const effectiveWidth = useMemo(() => {
    return Math.max(totalWidth, baseContainerWidth);
  }, [baseContainerWidth, totalWidth]);

  const displayColumnWidths = useMemo(() => {
    const extraWidth = Math.max(baseContainerWidth - totalWidth, 0);
    if (extraWidth <= 0) {
      return currentColumnWidths;
    }

    const next = [...currentColumnWidths];
    const fillIndex = next.length - 1;
    if (fillIndex >= 0) {
      next[fillIndex] = next[fillIndex]! + extraWidth;
    }
    return next;
  }, [currentColumnWidths, baseContainerWidth, totalWidth]);

  const leftSpacing = 10;
  const chevronSpace = 24;
  const totalLeftSpacing = leftSpacing + chevronSpace;
  const indentPerLevel = 20;
  const finalDividerInset = 8;

  const firstColumnBodyWidth = Math.max((displayColumnWidths[0] ?? 0) - totalLeftSpacing, 0);
  const renderItemTotalWidth = Math.max(effectiveWidth - totalLeftSpacing, 0);

  const columnLayoutWidths = useMemo(
    () =>
      displayColumnWidths.map((width, index) =>
        index === 0 ? Math.max(width - totalLeftSpacing, 0) : width
      ),
    [displayColumnWidths, totalLeftSpacing]
  );

  const gridTemplateColumns = useMemo(
    () => columnLayoutWidths.map(width => `${width}px`).join(' '),
    [columnLayoutWidths]
  );

  const rowStyle = useMemo(
    () =>
      ({
        width: `${renderItemTotalWidth}px`,
        minWidth: `${renderItemTotalWidth}px`,
        gridTemplateColumns,
      }) satisfies React.CSSProperties,
    [renderItemTotalWidth, gridTemplateColumns]
  );

  const renderItem = ({ item, level, isSelected }: TreeTableRenderItemParams) => {
    const extended = item as I;
    const indentWidth = Math.max(
      Math.min(level * indentPerLevel, Math.max(firstColumnBodyWidth - 12, 0)),
      0
    );
    const firstCellPaddingLeft = indentWidth;

    return (
      <div className="grid text-sm text-foreground" style={rowStyle}>
        {columns.map(column => {
          if (column.isFirst) {
            return (
              <div
                key={column.key}
                className="flex items-center pr-1"
                style={{
                  paddingLeft: `${firstCellPaddingLeft}px`,
                  paddingRight: '6px',
                }}
              >
                <span className="flex-1 truncate text-left text-sm text-foreground">
                  {column.render({ item: extended, level, isSelected })}
                </span>
              </div>
            );
          }

          return (
            <div
              key={column.key}
              className="px-3 py-2 text-left text-xs text-muted-foreground overflow-hidden text-ellipsis whitespace-nowrap"
            >
              {column.render({ item: extended, level, isSelected })}
            </div>
          );
        })}
      </div>
    );
  };

  const handleColumnResizeStart = (index: number, e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();

    const startX = e.clientX;
    const initialWidths = hasUserResized ? currentColumnWidths : displayColumnWidths;
    const startWidths = [...initialWidths];
    const minWidth = 80;

    const onMouseMove = (moveEvent: MouseEvent) => {
      const delta = moveEvent.clientX - startX;
      const next = [...startWidths];
      const leftIndex = index;

      let newLeft = startWidths[leftIndex]! + delta;
      if (newLeft < minWidth) {
        newLeft = minWidth;
      }

      next[leftIndex] = newLeft;
      setCurrentColumnWidths(next);
      if (index !== columns.length - 1) {
        setHasUserResized(true);
      }
    };

    const onMouseUp = () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  };
  return (
    <div className="min-h-screen">
      <div
        ref={containerRef}
        className={cn(
          'bg-transparent transition-opacity duration-100 ease-out',
          isLayoutReady ? 'opacity-100' : 'opacity-0'
        )}
      >
        <div className={cn('w-full', isLayoutReady ? 'overflow-x-auto' : 'overflow-x-hidden')}>
          <div style={{ minWidth: `${effectiveWidth}px` }}>
            <div
              className="mb-1 bg-accent/40 text-xs text-muted-foreground"
              style={
                {
                  width: `${effectiveWidth}px`,
                  minWidth: `${effectiveWidth}px`,
                  position: 'relative',
                  display: 'flex',
                  alignItems: 'center',
                } as React.CSSProperties
              }
            >
              <div className="shrink-0" style={{ width: `${leftSpacing}px` }} />
              <div className="shrink-0" style={{ width: `${chevronSpace}px` }} />
              {columns.map((column, index) => {
                const columnWidth = columnLayoutWidths[column.widthIndex] ?? 0;

                return (
                  <div
                    key={column.key}
                    className="relative flex items-center px-3 py-3 text-xs font-semibold text-muted-foreground"
                    style={{ width: columnWidth }}
                  >
                    <span className="font-semibold">{column.label}</span>
                    {(index < columns.length - 1 || index === columns.length - 1) && (
                      <div
                        className="absolute top-1/2 flex h-8 w-5 -translate-y-1/2 cursor-col-resize items-center justify-center"
                        style={{
                          right: index === columns.length - 1 ? `${finalDividerInset}px` : '-2px',
                        }}
                        onMouseDown={e => handleColumnResizeStart(index, e)}
                      >
                        <div
                          className="h-4/5 w-px opacity-50 transition-opacity hover:opacity-75"
                          style={{
                            background:
                              'linear-gradient(180deg, rgba(0,0,0,0.35) 0%, rgba(0,0,0,0.25) 45%, rgba(0,0,0,0.4) 100%)',
                            boxShadow:
                              '1px 0 0 rgba(255, 255, 255, 0.04), -1px 0 0 rgba(0, 0, 0, 0.35)',
                          }}
                        />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
            <div style={{ width: `${effectiveWidth}px`, minWidth: `${effectiveWidth}px` }}>
              <TreeView data={treeData} renderItem={renderItem} {...treeViewProps} />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export { TreeView };
