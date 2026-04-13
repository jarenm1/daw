use crate::instrument::VirtualInstrument;
use crate::midi_event::{MidiEvent, MidiEventType, MidiNote};
use crate::transport::Transport;
use std::sync::Arc;

/// A MIDI clip containing notes
#[derive(Clone)]
pub struct MidiClip {
    pub name: String,
    pub notes: Vec<MidiNote>,
    pub start_time: f64, // Clip position on timeline (seconds)
    pub duration: f64,   // Clip length
    pub channel: u8,
    pub color: u32, // RGB color for UI
}

impl MidiClip {
    pub fn new(name: &str, start: f64, duration: f64) -> Self {
        Self {
            name: name.to_string(),
            notes: Vec::new(),
            start_time: start,
            duration: duration.max(0.1),
            channel: 0,
            color: 0x3B82F6, // Default blue
        }
    }

    /// Add a note to the clip
    pub fn add_note(&mut self, note: MidiNote) {
        self.notes.push(note);
    }

    /// Get all events at current transport position
    pub fn get_events_at(&self, transport_pos: f64, sample_rate: u32) -> Vec<MidiEvent> {
        let mut events = Vec::new();

        // Check if transport is within this clip
        let clip_pos = transport_pos - self.start_time;
        if clip_pos < 0.0 || clip_pos > self.duration {
            return events;
        }

        // Calculate the sample window (how many samples per callback)
        let window_seconds = 256.0 / sample_rate as f64; // One buffer ahead
        let window_end = clip_pos + window_seconds;

        for note in &self.notes {
            // Check if note starts within this window
            if note.start_time >= clip_pos && note.start_time < window_end {
                events.push(note.to_note_on_event(sample_rate));
            }

            // Check if note ends within this window
            let note_end = note.start_time + note.duration;
            if note_end >= clip_pos && note_end < window_end {
                events.push(note.to_note_off_event(sample_rate));
            }
        }

        events
    }

    /// Get notes active at a given time (for piano roll display)
    pub fn active_notes_at(&self, time: f64) -> Vec<&MidiNote> {
        let clip_time = time - self.start_time;
        self.notes
            .iter()
            .filter(|n| n.is_active_at(clip_time))
            .collect()
    }

    /// Move clip on timeline
    pub fn set_position(&mut self, start: f64) {
        self.start_time = start;
    }

    /// Trim/extend clip
    pub fn set_duration(&mut self, duration: f64) {
        self.duration = duration.max(0.1);
    }
}

/// A track containing multiple MIDI clips
pub struct MidiTrack {
    pub name: String,
    pub clips: Vec<MidiClip>,
    pub instrument: Arc<dyn VirtualInstrument>,
    pub channel: u8,
    pub muted: bool,
    pub soloed: bool,
    pub volume: f32, // 0.0 - 1.0
}

impl MidiTrack {
    pub fn new(name: &str, instrument: Arc<dyn VirtualInstrument>) -> Self {
        Self {
            name: name.to_string(),
            clips: Vec::new(),
            instrument,
            channel: 0,
            muted: false,
            soloed: false,
            volume: 0.8,
        }
    }

    pub fn add_clip(&mut self, clip: MidiClip) {
        self.clips.push(clip);
    }

    /// Get all MIDI events from all clips at current position
    pub fn get_events_at(&self, transport_pos: f64, sample_rate: u32) -> Vec<MidiEvent> {
        if self.muted {
            return Vec::new();
        }

        let mut events = Vec::new();
        for clip in &self.clips {
            events.extend(clip.get_events_at(transport_pos, sample_rate));
        }
        events
    }
}

/// The sequencer manages all tracks and generates MIDI events
pub struct Sequencer {
    tracks: Vec<MidiTrack>,
    transport: Transport,
    sample_rate: u32,
}

impl Sequencer {
    pub fn new(transport: Transport, sample_rate: u32) -> Self {
        Self {
            tracks: Vec::new(),
            transport,
            sample_rate,
        }
    }

    /// Add a track
    pub fn add_track(&mut self, track: MidiTrack) {
        self.tracks.push(track);
    }

    /// Process one audio buffer - generates MIDI events and advances transport
    pub fn process(&mut self, buffer_size: usize) -> Vec<MidiEvent> {
        let mut all_events = Vec::new();

        // Only process when playing
        if !self.transport.is_playing() {
            // Still need to clear any stuck voices when stopped/paused
            return all_events;
        }

        // Get current position
        let current_pos = self.transport.position();

        // Collect events from all tracks
        for track in &self.tracks {
            if !track.muted {
                for clip in &track.clips {
                    let events = clip.get_events_at(current_pos, self.sample_rate);
                    all_events.extend(events);
                }
            }
        }

        // Advance transport
        self.transport.advance(buffer_size);

        all_events
    }

    /// Get reference to transport
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    /// Get mutable reference to transport
    pub fn transport_mut(&mut self) -> &mut Transport {
        &mut self.transport
    }

    /// Get all tracks
    pub fn tracks(&self) -> &[MidiTrack] {
        &self.tracks
    }

    /// Get all tracks mutable
    pub fn tracks_mut(&mut self) -> &mut [MidiTrack] {
        &mut self.tracks
    }

    /// Get track by index
    pub fn track(&self, index: usize) -> Option<&MidiTrack> {
        self.tracks.get(index)
    }

    pub fn track_mut(&mut self, index: usize) -> Option<&mut MidiTrack> {
        self.tracks.get_mut(index)
    }

    /// Get total duration of all clips
    pub fn total_duration(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| c.start_time + c.duration)
            .fold(0.0, f64::max)
    }
}

/// Timeline is the high-level container for the entire project
pub struct Timeline {
    pub sequencer: Sequencer,
    pub name: String,
}

impl Timeline {
    pub fn new(name: &str, sample_rate: u32) -> Self {
        let transport = crate::transport::TransportBuilder::new(sample_rate).build();

        Self {
            sequencer: Sequencer::new(transport, sample_rate),
            name: name.to_string(),
        }
    }

    /// Create a timeline with specific BPM
    pub fn with_bpm(name: &str, sample_rate: u32, bpm: f64) -> Self {
        let transport = crate::transport::TransportBuilder::new(sample_rate)
            .bpm(bpm)
            .build();

        Self {
            sequencer: Sequencer::new(transport, sample_rate),
            name: name.to_string(),
        }
    }

    /// Play from beginning
    pub fn play(&mut self) {
        self.sequencer.transport_mut().play();
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.sequencer.transport_mut().stop();
    }

    /// Pause
    pub fn pause(&mut self) {
        self.sequencer.transport_mut().pause();
    }

    /// Toggle play/pause
    pub fn toggle(&mut self) {
        self.sequencer.transport_mut().toggle_playback();
    }

    /// Seek to position
    pub fn seek(&mut self, seconds: f64) {
        self.sequencer.transport_mut().seek_to(seconds);
    }

    /// Get current formatted position
    pub fn position(&self) -> String {
        self.sequencer.transport().position_formatted()
    }

    /// Process audio callback - returns MIDI events for instruments
    pub fn process(&mut self, buffer_size: usize) -> Vec<MidiEvent> {
        self.sequencer.process(buffer_size)
    }
}
