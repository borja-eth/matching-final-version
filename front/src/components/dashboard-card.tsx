"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { LucideIcon } from "lucide-react"

interface DashboardCardProps {
  title: string
  value: string | number
  icon: LucideIcon
  description?: string
  trend?: "up" | "down" | "neutral"
  trendValue?: string
}

export default function DashboardCard({
  title,
  value,
  icon: Icon,
  description,
  trend,
  trendValue,
}: DashboardCardProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        <Icon className="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value}</div>
        {description && (
          <p className="text-xs text-muted-foreground">{description}</p>
        )}
        {trend && trendValue && (
          <div className="flex items-center space-x-1 pt-1">
            <span
              className={
                trend === "up"
                  ? "text-emerald-500"
                  : trend === "down"
                  ? "text-rose-500"
                  : "text-muted-foreground"
              }
            >
              {trend === "up" ? "↑" : trend === "down" ? "↓" : "→"}
            </span>
            <span className="text-xs text-muted-foreground">{trendValue}</span>
          </div>
        )}
      </CardContent>
    </Card>
  )
} 