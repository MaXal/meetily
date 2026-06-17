'use client';

// Settings section for speaker identification (diarization).
// Toggle persists via the diarization_set_enabled Tauri command; the embedding
// model (~28 MB) downloads on demand with progress. Labels apply to the system
// audio stream only — the local microphone is always "You".

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Button } from './ui/button';
import { Label } from './ui/label';
import { Switch } from './ui/switch';
import { Progress } from './ui/progress';
import { Download, Mic2 } from 'lucide-react';
import { toast } from 'sonner';

interface DiarizationStatus {
  enabled: boolean;
  model_present: boolean;
  model_filename: string;
}

interface DownloadProgressEvent {
  downloaded_bytes: number;
  total_bytes: number;
  percent: number;
}

export function SpeakerIdentificationSettings() {
  const [status, setStatus] = useState<DiarizationStatus | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadPercent, setDownloadPercent] = useState(0);

  const refreshStatus = useCallback(async () => {
    try {
      const result = await invoke<DiarizationStatus>('diarization_get_status');
      setStatus(result);
    } catch (err) {
      console.error('Failed to fetch diarization status:', err);
    }
  }, []);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const setup = async () => {
      unlisten = await listen<DownloadProgressEvent>(
        'diarization-model-download-progress',
        (event) => {
          setDownloadPercent(event.payload.percent);
        }
      );
    };
    setup();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleToggle = async (enabled: boolean) => {
    try {
      await invoke('diarization_set_enabled', { enabled });
      setStatus((prev) => (prev ? { ...prev, enabled } : prev));
      if (enabled && status && !status.model_present) {
        toast.info('Download the speaker model to activate speaker labels.');
      } else if (enabled) {
        toast.success('Speaker identification will be active on your next recording.');
      }
    } catch (err) {
      console.error('Failed to update speaker identification setting:', err);
      toast.error('Failed to update speaker identification setting');
    }
  };

  const handleDownload = async () => {
    setIsDownloading(true);
    setDownloadPercent(0);
    try {
      await invoke('diarization_download_model');
      toast.success('Speaker model downloaded');
      await refreshStatus();
    } catch (err) {
      console.error('Speaker model download failed:', err);
      toast.error(`Speaker model download failed: ${err}`);
    } finally {
      setIsDownloading(false);
    }
  };

  if (!status) return null;

  return (
    <div className="mt-6 border-t pt-4">
      <div className="flex items-center justify-between">
        <div>
          <Label className="flex items-center gap-2 text-sm font-medium text-gray-700">
            <Mic2 className="h-4 w-4" />
            Speaker identification
            <span className="text-xs font-normal text-amber-600 bg-amber-50 px-1.5 py-0.5 rounded">
              Experimental
            </span>
          </Label>
          <p className="text-xs text-gray-500 mt-1">
            Split other participants into &quot;Speaker 1/2/…&quot; in transcripts. Runs fully
            on-device; voice data never leaves your computer. Your microphone is always
            labeled &quot;You&quot;.
          </p>
        </div>
        <Switch checked={status.enabled} onCheckedChange={handleToggle} />
      </div>

      {status.enabled && !status.model_present && (
        <div className="mt-3">
          {isDownloading ? (
            <div className="space-y-1">
              <Progress value={downloadPercent} />
              <p className="text-xs text-gray-500">
                Downloading speaker model… {downloadPercent}%
              </p>
            </div>
          ) : (
            <Button variant="outline" size="sm" onClick={handleDownload}>
              <Download className="h-4 w-4 mr-2" />
              Download speaker model (~28 MB)
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
