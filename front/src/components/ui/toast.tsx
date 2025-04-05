'use client';

// Since we're using sonner for our toasts, we don't need the Radix UI implementation
// Just re-export the toast function from sonner

import { toast } from 'sonner';

export { toast };

// This file is kept for compatibility with any code that might import from it,
// but we're actually using sonner directly as our toast library instead of Radix UI 