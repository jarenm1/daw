/// A virtual instrument that converts MIDI events to audio
pub trait VirtualInstrument: Send + Sync {
    /// Process MIDI event (called in real-time audio thread)
    fn handle_event(&mut self, event: &crate::midi_event::MidiEvent);

    /// Render audio output for the given buffer size
    /// This is called every audio callback
    fn render(&mut self, output: &mut [f32], channels: usize, sample_rate: u32);

    /// Get the instrument name
    fn name(&self) -> &str;

    /// Reset/clear all state
    fn reset(&mut self);
}
