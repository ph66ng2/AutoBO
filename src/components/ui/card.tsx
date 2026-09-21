import React from "react";
import { cn } from "../../lib/cn";

interface CardProps {
  className?: string;
  children: React.ReactNode;
  onClick?: () => void;
}

export function Card({ className = "", children, onClick }: CardProps) {
  return (
    <div
      className={cn("rounded-xl border bg-card text-card-foreground shadow", className)}
      onClick={onClick}
    >
      {children}
    </div>
  );
}

export function CardHeader({ className = "", children }: CardProps) {
  return <div className={cn("flex flex-col space-y-1.5 p-6", className)}>{children}</div>;
}

export function CardTitle({ className = "", children }: CardProps) {
  return (
    <h3 className={cn("font-semibold leading-none tracking-tight", className)}>{children}</h3>
  );
}

export function CardContent({ className = "", children }: CardProps) {
  return <div className={cn("p-6 pt-0", className)}>{children}</div>;
}
