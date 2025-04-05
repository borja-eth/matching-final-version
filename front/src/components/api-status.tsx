'use client';

import { useApi } from '@/lib/api-context';
import { ServerOff, ExternalLink, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useState } from 'react';

export default function ApiStatus() {
  const { isApiConnected } = useApi();
  const [isChecking, setIsChecking] = useState(false);
  
  const handleRetry = () => {
    setIsChecking(true);
    // Short timeout to show the checking state before refreshing
    setTimeout(() => {
      window.location.reload();
    }, 500);
  };
  
  if (isApiConnected) {
    return null;
  }
  
  return (
    <div className="p-4 my-4 rounded-lg bg-destructive/10 border border-destructive text-destructive">
      <div className="flex items-start gap-4">
        <ServerOff className="h-6 w-6 mt-1" />
        <div className="flex-1">
          <h3 className="text-lg font-medium">API Connection Error</h3>
          <p className="mb-2">
            Unable to connect to the API server at http://127.0.0.1:3001. The backend service may be down or unreachable.
          </p>
          <div className="bg-background/80 p-3 rounded-md border border-border mt-2 mb-3">
            <p className="font-medium mb-1">To start the API server:</p>
            <code className="block bg-secondary/30 p-2 rounded text-xs mb-2 whitespace-pre overflow-x-auto">
              # Navigate to the project root<br/>
              cd /Users/borjamartelseward/Desktop/ultimate-matching<br/><br/>
              # Run the API server<br/>
              cargo run --bin api_server
            </code>
            <p className="text-xs text-muted-foreground">The API server must be running for the exchange to function properly.</p>
          </div>
          <div className="flex gap-2 mt-4">
            <Button variant="outline" size="sm" onClick={handleRetry} disabled={isChecking}>
              {isChecking ? (
                <>
                  <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                  Checking Connection...
                </>
              ) : (
                <>
                  <RefreshCw className="h-4 w-4 mr-2" />
                  Retry Connection
                </>
              )}
            </Button>
            <Button 
              variant="outline" 
              size="sm" 
              onClick={() => window.open('https://github.com/yourusername/ultimate-matching#running-the-api-server', '_blank')}
            >
              <ExternalLink className="h-4 w-4 mr-2" />
              View Documentation
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
} 