'use client';

import { useApi } from '@/lib/api-context';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Button } from "./ui/button";
import { PlusCircle, RefreshCw } from "lucide-react";
import { toast } from "sonner";

export default function InstrumentSelector() {
  const { instruments, selectedInstrument, selectInstrument, refreshInstruments, createDefaultInstrument, isApiConnected } = useApi();
  
  if (!isApiConnected) {
    return (
      <Button variant="outline" disabled>
        <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
        Connecting...
      </Button>
    );
  }

  // Find the name of the selected instrument
  const selectedInstrumentName = selectedInstrument 
    ? instruments.find(i => i.id === selectedInstrument)?.name || 'Select instrument'
    : 'Select instrument';
  
  const handleRefresh = async () => {
    await refreshInstruments();
    toast("Refreshed", {
      description: "Instrument list has been updated",
    });
  };
  
  const handleCreateNew = async () => {
    await createDefaultInstrument();
  };
  

  return (
    <div className="flex items-center gap-2">
      <Select value={selectedInstrument || undefined} onValueChange={selectInstrument}>
        <SelectTrigger className="w-[180px]">
          <SelectValue placeholder={selectedInstrumentName} />
        </SelectTrigger>
        <SelectContent>
          {instruments.map((instrument) => (
            <SelectItem key={instrument.id} value={instrument.id}>
              {instrument.name || `${instrument.base_currency}/${instrument.quote_currency}`}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      
      <Button size="icon" variant="ghost" onClick={handleRefresh}>
        <RefreshCw className="h-4 w-4" />
      </Button>
      
      <Button size="sm" onClick={handleCreateNew}>
        <PlusCircle className="h-4 w-4 mr-2" />
        New
      </Button>
    </div>
  );
} 