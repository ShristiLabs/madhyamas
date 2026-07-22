import { Component, type ErrorInfo, type ReactNode } from "react"
import { AlertCircle, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/button"

interface ErrorBoundaryProps {
  children: ReactNode
  /** Label shown in the error message, e.g. "gRPC Panel" */
  label?: string
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
}

/**
 * Catches render errors in child components and shows a friendly fallback
 * instead of a blank screen. The user can retry to re-mount the subtree.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`[ErrorBoundary${this.props.label ? `: ${this.props.label}` : ""}]`, error, info)
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null })
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
          <AlertCircle className="h-10 w-10 text-destructive" />
          <div>
            <p className="text-lg font-semibold">
              {this.props.label ?? "This panel"} encountered an error
            </p>
            <p className="mt-1 max-w-md text-sm text-muted-foreground">
              {this.state.error?.message ?? "An unexpected error occurred while rendering."}
            </p>
          </div>
          <Button variant="outline" size="sm" onClick={this.handleRetry}>
            <RefreshCw className="mr-2 h-4 w-4" /> Retry
          </Button>
        </div>
      )
    }
    return this.props.children
  }
}
