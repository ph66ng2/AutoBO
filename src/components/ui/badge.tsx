import React from "react";
import { cn } from "../../lib/cn";

interface BadgeProps {
  variant?: "default" | "success" | "warning" | "danger" | "neutral" | "secondary";
  children: React.ReactNode;
  className?: string;
}

export function Badge({ variant = "default", children, className = "" }: BadgeProps) {
  const variants: Record<string, string> = {
    default: "border-transparent bg-primary text-primary-foreground",
    secondary: "border-transparent bg-secondary text-secondary-foreground",
    success: "border-transparent bg-emerald-100 text-emerald-800",
    warning: "border-transparent bg-amber-100 text-amber-800",
    danger: "border-transparent bg-red-100 text-red-800",
    neutral: "border-transparent bg-gray-100 text-gray-800",
  };
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
        variants[variant],
        className,
      )}
    >
      {children}
    </span>
  );
}
