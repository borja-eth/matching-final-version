'use client';

import { toast } from 'sonner';
import { Toaster } from './sonner';

export const useToast = () => {
  return {
    toast: (props: {
      title?: string;
      description?: string;
      variant?: 'default' | 'destructive';
      [key: string]: any;
    }) => {
      const { title, description, variant, ...rest } = props;
      return toast(title, {
        description,
        className: variant === 'destructive' ? 'destructive' : undefined,
        ...rest,
      });
    }
  };
};

export { Toaster }; 