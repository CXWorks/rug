//! Manages recording and enables playback for client sent events.
//!
//! Clients can store multiple, named recordings.
use std::collections::HashMap;
use xi_trace::trace_block;
use crate::edit_types::{BufferEvent, EventDomain};
/// A container that manages and holds all recordings for the current editing session
pub(crate) struct Recorder {
    active_recording: Option<String>,
    recording_buffer: Vec<EventDomain>,
    recordings: HashMap<String, Recording>,
}
impl Recorder {
    pub(crate) fn new() -> Recorder {
        Recorder {
            active_recording: None,
            recording_buffer: Vec::new(),
            recordings: HashMap::new(),
        }
    }
    pub(crate) fn is_recording(&self) -> bool {
        self.active_recording.is_some()
    }
    /// Starts or stops the specified recording.
    ///
    ///
    /// There are three outcome behaviors:
    /// - If the current recording name is specified, the active recording is saved
    /// - If no recording name is specified, the currently active recording is saved
    /// - If a recording name other than the active recording is specified,
    /// the current recording will be thrown out and will be switched to the new name
    ///
    /// In addition to the above:
    /// - If the recording was saved, there is no active recording
    /// - If the recording was switched, there will be a new active recording
    pub(crate) fn toggle_recording(&mut self, recording_name: Option<String>) {
        let is_recording = self.is_recording();
        let last_recording = self.active_recording.take();
        match (is_recording, &last_recording, &recording_name) {
            (true, Some(last_recording), None) => {
                self.save_recording_buffer(last_recording.clone())
            }
            (true, Some(last_recording), Some(recording_name)) => {
                if last_recording != recording_name {
                    self.recording_buffer.clear();
                } else {
                    self.save_recording_buffer(last_recording.clone());
                    return;
                }
            }
            _ => {}
        }
        self.active_recording = recording_name;
    }
    /// Saves an event into the currently active recording.
    ///
    /// Every sequential `BufferEvent::Insert` event will be merged together to cut down the number of
    /// `Editor::commit_delta` calls we need to make when playing back.
    pub(crate) fn record(&mut self, current_event: EventDomain) {
        assert!(self.is_recording());
        let recording_buffer = &mut self.recording_buffer;
        if recording_buffer.last().is_none() {
            recording_buffer.push(current_event);
            return;
        }
        {
            let last_event = recording_buffer.last_mut().unwrap();
            if let (
                EventDomain::Buffer(BufferEvent::Insert(old_characters)),
                EventDomain::Buffer(BufferEvent::Insert(new_characters)),
            ) = (last_event, &current_event) {
                old_characters.push_str(new_characters);
                return;
            }
        }
        recording_buffer.push(current_event);
    }
    /// Iterates over a specified recording's buffer and runs the specified action
    /// on each event.
    pub(crate) fn play<F>(&self, recording_name: &str, action: F)
    where
        F: FnMut(&EventDomain),
    {
        let is_current_recording: bool = self
            .active_recording
            .as_ref()
            .map_or(false, |current_recording| current_recording == recording_name);
        if is_current_recording {
            warn!("Cannot play recording while it's currently active!");
            return;
        }
        if let Some(recording) = self.recordings.get(recording_name) {
            recording.play(action);
        }
    }
    /// Completely removes the specified recording from the Recorder
    pub(crate) fn clear(&mut self, recording_name: &str) {
        self.recordings.remove(recording_name);
    }
    /// Cleans the recording buffer by filtering out any undo or redo events and then saving it
    /// with the specified name.
    ///
    /// A recording should not store any undos or redos--
    /// call this once a recording is 'finalized.'
    fn save_recording_buffer(&mut self, recording_name: String) {
        let mut saw_undo = false;
        let mut saw_redo = false;
        let filtered: Vec<EventDomain> = self
            .recording_buffer
            .clone()
            .into_iter()
            .rev()
            .filter(|event| {
                if let EventDomain::Buffer(event) = event {
                    return match event {
                        BufferEvent::Undo => {
                            saw_undo = !saw_redo;
                            saw_redo = false;
                            false
                        }
                        BufferEvent::Redo => {
                            saw_redo = !saw_undo;
                            saw_undo = false;
                            false
                        }
                        _ => {
                            let ret = !saw_undo;
                            saw_undo = false;
                            saw_redo = false;
                            ret
                        }
                    };
                }
                true
            })
            .collect::<Vec<EventDomain>>()
            .into_iter()
            .rev()
            .collect();
        let current_recording = Recording::new(filtered);
        self.recordings.insert(recording_name, current_recording);
        self.recording_buffer.clear();
    }
}
struct Recording {
    events: Vec<EventDomain>,
}
impl Recording {
    fn new(events: Vec<EventDomain>) -> Recording {
        Recording { events }
    }
    /// Iterates over the recording buffer and runs the specified action
    /// on each event.
    fn play<F>(&self, action: F)
    where
        F: FnMut(&EventDomain),
    {
        let _guard = trace_block("Recording::play", &["core", "recording"]);
        self.events.iter().for_each(action)
    }
}
#[cfg(test)]
mod tests {
    use crate::edit_types::{BufferEvent, EventDomain};
    use crate::recorder::Recorder;
    #[test]
    fn play_recording() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        let mut expected_events: Vec<EventDomain> = vec![
            BufferEvent::Indent.into(), BufferEvent::Outdent.into(),
            BufferEvent::DuplicateLine.into(), BufferEvent::Transpose.into(),
        ];
        recorder.toggle_recording(Some(recording_name.clone()));
        for event in expected_events.iter().rev() {
            recorder.record(event.clone());
        }
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder
            .play(
                &recording_name,
                |event| {
                    let expected_event = expected_events.pop();
                    assert!(expected_event.is_some());
                    assert_eq!(* event, expected_event.unwrap());
                },
            );
        assert_eq!(expected_events.len(), 0);
    }
    #[test]
    fn play_only_after_saved() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        let expected_events: Vec<EventDomain> = vec![
            BufferEvent::Indent.into(), BufferEvent::Outdent.into(),
            BufferEvent::DuplicateLine.into(), BufferEvent::Transpose.into(),
        ];
        recorder.toggle_recording(Some(recording_name.clone()));
        for event in expected_events.iter().rev() {
            recorder.record(event.clone());
        }
        recorder
            .play(
                &recording_name,
                |_| {
                    assert!(false);
                },
            );
    }
    #[test]
    fn prevent_same_playback() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        let expected_events: Vec<EventDomain> = vec![
            BufferEvent::Indent.into(), BufferEvent::Outdent.into(),
            BufferEvent::DuplicateLine.into(), BufferEvent::Transpose.into(),
        ];
        recorder.toggle_recording(Some(recording_name.clone()));
        for event in expected_events.iter().rev() {
            recorder.record(event.clone());
        }
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder
            .play(
                &recording_name,
                |_| {
                    assert!(false);
                },
            );
    }
    #[test]
    fn clear_recording() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.record(BufferEvent::Transpose.into());
        recorder.record(BufferEvent::DuplicateLine.into());
        recorder.record(BufferEvent::Outdent.into());
        recorder.record(BufferEvent::Indent.into());
        recorder.toggle_recording(Some(recording_name.clone()));
        assert_eq!(recorder.recordings.get(& recording_name).unwrap().events.len(), 4);
        recorder.clear(&recording_name);
        assert!(recorder.recordings.get(& recording_name).is_none());
    }
    #[test]
    fn multiple_recordings() {
        let mut recorder = Recorder::new();
        let recording_a = "a".to_string();
        let recording_b = "b".to_string();
        recorder.toggle_recording(Some(recording_a.clone()));
        recorder.record(BufferEvent::Transpose.into());
        recorder.record(BufferEvent::DuplicateLine.into());
        recorder.toggle_recording(Some(recording_a.clone()));
        recorder.toggle_recording(Some(recording_b.clone()));
        recorder.record(BufferEvent::Outdent.into());
        recorder.record(BufferEvent::Indent.into());
        recorder.toggle_recording(Some(recording_b.clone()));
        assert_eq!(
            recorder.recordings.get(& recording_a).unwrap().events,
            vec![BufferEvent::Transpose.into(), BufferEvent::DuplicateLine.into()]
        );
        assert_eq!(
            recorder.recordings.get(& recording_b).unwrap().events,
            vec![BufferEvent::Outdent.into(), BufferEvent::Indent.into()]
        );
        recorder.clear(&recording_a);
        assert!(recorder.recordings.get(& recording_a).is_none());
        assert!(recorder.recordings.get(& recording_b).is_some());
    }
    #[test]
    fn text_playback() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.record(BufferEvent::Insert("Foo".to_owned()).into());
        recorder.record(BufferEvent::Insert("B".to_owned()).into());
        recorder.record(BufferEvent::Insert("A".to_owned()).into());
        recorder.record(BufferEvent::Insert("R".to_owned()).into());
        recorder.toggle_recording(Some(recording_name.clone()));
        assert_eq!(
            recorder.recordings.get(& recording_name).unwrap().events,
            vec![BufferEvent::Insert("FooBAR".to_owned()).into()]
        );
    }
    #[test]
    fn basic_undo() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.record(BufferEvent::Transpose.into());
        recorder.record(BufferEvent::Undo.into());
        recorder.record(BufferEvent::DuplicateLine.into());
        recorder.record(BufferEvent::Redo.into());
        recorder.toggle_recording(Some(recording_name.clone()));
        assert_eq!(
            recorder.recordings.get(& recording_name).unwrap().events,
            vec![BufferEvent::DuplicateLine.into()]
        );
    }
    #[test]
    fn basic_undo_swapped() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.record(BufferEvent::Transpose.into());
        recorder.record(BufferEvent::Redo.into());
        recorder.record(BufferEvent::DuplicateLine.into());
        recorder.record(BufferEvent::Undo.into());
        recorder.toggle_recording(Some(recording_name.clone()));
        assert_eq!(
            recorder.recordings.get(& recording_name).unwrap().events,
            vec![BufferEvent::Transpose.into()]
        );
    }
    #[test]
    fn redo_cancels_undo() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.record(BufferEvent::Transpose.into());
        recorder.record(BufferEvent::Undo.into());
        recorder.record(BufferEvent::Redo.into());
        recorder.record(BufferEvent::DuplicateLine.into());
        recorder.toggle_recording(Some(recording_name.clone()));
        assert_eq!(
            recorder.recordings.get(& recording_name).unwrap().events,
            vec![BufferEvent::Transpose.into(), BufferEvent::DuplicateLine.into()]
        );
    }
    #[test]
    fn undo_cancels_redo() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.record(BufferEvent::Transpose.into());
        recorder.record(BufferEvent::Undo.into());
        recorder.record(BufferEvent::Redo.into());
        recorder.record(BufferEvent::Undo.into());
        recorder.toggle_recording(Some(recording_name.clone()));
        assert_eq!(recorder.recordings.get(& recording_name).unwrap().events, vec![]);
    }
    #[test]
    fn undo_as_first_item() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.record(BufferEvent::Undo.into());
        recorder.record(BufferEvent::Transpose.into());
        recorder.record(BufferEvent::DuplicateLine.into());
        recorder.record(BufferEvent::Redo.into());
        recorder.toggle_recording(Some(recording_name.clone()));
        assert_eq!(
            recorder.recordings.get(& recording_name).unwrap().events,
            vec![BufferEvent::Transpose.into(), BufferEvent::DuplicateLine.into()]
        );
    }
    #[test]
    fn redo_as_first_item() {
        let mut recorder = Recorder::new();
        let recording_name = String::new();
        recorder.toggle_recording(Some(recording_name.clone()));
        recorder.record(BufferEvent::Redo.into());
        recorder.record(BufferEvent::Transpose.into());
        recorder.record(BufferEvent::DuplicateLine.into());
        recorder.record(BufferEvent::Undo.into());
        recorder.toggle_recording(Some(recording_name.clone()));
        assert_eq!(
            recorder.recordings.get(& recording_name).unwrap().events,
            vec![BufferEvent::Transpose.into()]
        );
    }
}
#[cfg(test)]
mod tests_llm_16_654 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_recording() {
        let _rug_st_tests_llm_16_654_rrrruuuugggg_test_is_recording = 0;
        let rug_fuzz_0 = "recording_name";
        let rug_fuzz_1 = "abc";
        let recorder = Recorder::new();
        debug_assert_eq!(recorder.is_recording(), false);
        let mut recorder_with_recording = Recorder {
            active_recording: Some(String::from(rug_fuzz_0)),
            recording_buffer: vec![
                EventDomain::Buffer(BufferEvent::Insert(String::from(rug_fuzz_1)))
            ],
            recordings: HashMap::new(),
        };
        debug_assert_eq!(recorder_with_recording.is_recording(), true);
        let _rug_ed_tests_llm_16_654_rrrruuuugggg_test_is_recording = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_657 {
    use super::*;
    use crate::*;
    #[test]
    fn test_play() {
        let _rug_st_tests_llm_16_657_rrrruuuugggg_test_play = 0;
        let rug_fuzz_0 = "test_recording";
        let rug_fuzz_1 = "Hello";
        let rug_fuzz_2 = false;
        let rug_fuzz_3 = true;
        let mut recorder = Recorder::new();
        let recording_name = rug_fuzz_0;
        let recording = Recording::new(
            vec![
                EventDomain::Buffer(BufferEvent::Insert(rug_fuzz_1.to_string())),
                EventDomain::Buffer(BufferEvent::Insert(", World!".to_string()))
            ],
        );
        recorder.recordings.insert(recording_name.to_string(), recording);
        let mut action_called = rug_fuzz_2;
        let action = |event: &EventDomain| {
            action_called = rug_fuzz_3;
        };
        recorder.play(recording_name, action);
        debug_assert!(action_called);
        let _rug_ed_tests_llm_16_657_rrrruuuugggg_test_play = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_661 {
    use super::*;
    use crate::*;
    #[test]
    fn test_toggle_recording() {
        let _rug_st_tests_llm_16_661_rrrruuuugggg_test_toggle_recording = 0;
        let rug_fuzz_0 = "recording1";
        let rug_fuzz_1 = "recording2";
        let rug_fuzz_2 = "recording1";
        let rug_fuzz_3 = "test";
        let rug_fuzz_4 = "recording1";
        let rug_fuzz_5 = "test";
        let rug_fuzz_6 = "recording2";
        let rug_fuzz_7 = "recording1";
        let rug_fuzz_8 = "test";
        let rug_fuzz_9 = "recording1";
        let rug_fuzz_10 = "recording1";
        let mut recorder = Recorder::new();
        recorder.active_recording = Some(rug_fuzz_0.to_string());
        recorder.toggle_recording(Some(rug_fuzz_1.to_string()));
        debug_assert_eq!(recorder.active_recording, Some("recording2".to_string()));
        debug_assert!(recorder.recording_buffer.is_empty());
        recorder.active_recording = Some(rug_fuzz_2.to_string());
        recorder
            .recording_buffer
            .push(EventDomain::Buffer(BufferEvent::Insert(rug_fuzz_3.to_string())));
        recorder.toggle_recording(None);
        debug_assert_eq!(recorder.active_recording, None);
        debug_assert_eq!(
            recorder.recording_buffer,
            vec![EventDomain::Buffer(BufferEvent::Insert("test".to_string()))]
        );
        recorder.active_recording = Some(rug_fuzz_4.to_string());
        recorder
            .recording_buffer
            .push(EventDomain::Buffer(BufferEvent::Insert(rug_fuzz_5.to_string())));
        recorder.toggle_recording(Some(rug_fuzz_6.to_string()));
        debug_assert_eq!(recorder.active_recording, Some("recording2".to_string()));
        debug_assert!(recorder.recording_buffer.is_empty());
        recorder.active_recording = Some(rug_fuzz_7.to_string());
        recorder
            .recording_buffer
            .push(EventDomain::Buffer(BufferEvent::Insert(rug_fuzz_8.to_string())));
        recorder.toggle_recording(Some(rug_fuzz_9.to_string()));
        debug_assert_eq!(recorder.active_recording, Some("recording1".to_string()));
        debug_assert_eq!(
            recorder.recording_buffer,
            vec![EventDomain::Buffer(BufferEvent::Insert("test".to_string()))]
        );
        recorder.active_recording = None;
        recorder.toggle_recording(Some(rug_fuzz_10.to_string()));
        debug_assert_eq!(recorder.active_recording, Some("recording1".to_string()));
        debug_assert!(recorder.recording_buffer.is_empty());
        recorder.active_recording = None;
        recorder.toggle_recording(None);
        debug_assert_eq!(recorder.active_recording, None);
        debug_assert!(recorder.recording_buffer.is_empty());
        let _rug_ed_tests_llm_16_661_rrrruuuugggg_test_toggle_recording = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_663_llm_16_662 {
    use super::*;
    use crate::*;
    use crate::recorder::Recording;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_663_llm_16_662_rrrruuuugggg_test_new = 0;
        let events: Vec<EventDomain> = vec![];
        let recording = Recording::new(events.clone());
        debug_assert_eq!(recording.events, events);
        let _rug_ed_tests_llm_16_663_llm_16_662_rrrruuuugggg_test_new = 0;
    }
}
