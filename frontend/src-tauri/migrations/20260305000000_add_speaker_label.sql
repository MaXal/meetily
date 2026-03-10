-- Add speaker_label column to transcripts table for mic/system speaker diarization
ALTER TABLE transcripts ADD COLUMN speaker_label TEXT;
