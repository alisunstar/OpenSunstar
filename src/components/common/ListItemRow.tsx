import React from "react";

interface ListItemRowProps {
  isLast?: boolean;
  children: React.ReactNode;
  id?: string;
  highlighted?: boolean;
}

export const ListItemRow: React.FC<ListItemRowProps> = ({
  isLast,
  children,
  id,
  highlighted,
}) => {
  return (
    <div
      id={id}
      className={`group flex items-center gap-3 px-4 py-2.5 hover:bg-muted/50 transition-colors ${
        !isLast ? "border-b border-border-default" : ""
      } ${highlighted ? "ring-2 ring-inset ring-primary/60 bg-primary/5" : ""}`}
    >
      {children}
    </div>
  );
};
