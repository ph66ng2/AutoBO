import * as React from "react";
import { cn } from "../../lib/cn";

const DialogContext = React.createContext<(open: boolean) => void>(() => {});

export function Dialog({
  open,
  onOpenChange,
  children,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: React.ReactNode;
}) {
  React.useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onOpenChange(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onOpenChange]);

  if (!open) return null;
  return (
    <DialogContext.Provider value={onOpenChange}>
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
        onClick={() => onOpenChange(false)}
      >
        {children}
      </div>
    </DialogContext.Provider>
  );
}

export function DialogContent({
  className = "",
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  const onOpenChange = React.useContext(DialogContext);
  const hasMaxW = /\bmax-w-/.test(className);
  const hasPad = /(^|\s)p-/.test(className);
  return (
    <div
      className={cn(
        "relative flex w-full max-h-[92vh] flex-col gap-4 border bg-card text-card-foreground shadow-lg sm:rounded-lg",
        hasMaxW ? "" : "max-w-lg",
        hasPad ? "" : "p-6",
        className,
      )}
      onClick={(event) => event.stopPropagation()}
      {...props}
    >
      <button
        type="button"
        className="absolute right-4 top-4 z-10 rounded-sm text-2xl leading-none text-muted-foreground hover:text-foreground"
        onClick={() => onOpenChange(false)}
        aria-label="Fechar"
      >
        ×
      </button>
      {children}
    </div>
  );
}

export function DialogHeader({
  className = "",
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex flex-col space-y-1.5 text-left", className)} {...props} />;
}

export function DialogFooter({
  className = "",
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", className)}
      {...props}
    />
  );
}

export function DialogTitle({
  className = "",
  ...props
}: React.HTMLAttributes<HTMLHeadingElement>) {
  return <h2 className={cn("text-lg font-semibold leading-none tracking-tight", className)} {...props} />;
}

export function DialogClose({
  asChild,
  children,
}: {
  asChild?: boolean;
  children: React.ReactElement;
}) {
  const onOpenChange = React.useContext(DialogContext);
  if (asChild) {
    return React.cloneElement(children, {
      onClick: (event: React.MouseEvent) => {
        const original = children.props as { onClick?: (event: React.MouseEvent) => void };
        original.onClick?.(event);
        onOpenChange(false);
      },
    });
  }
  return (
    <button type="button" onClick={() => onOpenChange(false)}>
      {children}
    </button>
  );
}
