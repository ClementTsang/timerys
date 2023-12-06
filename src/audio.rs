use std::{
    fs::File,
    io::{BufReader, Cursor},
    time::Duration,
};

use rodio::{Decoder, OutputStream, Sink, Source};

use crate::TimerApp;

impl TimerApp {
    pub(crate) fn play_audio(&mut self) -> eyre::Result<()> {
        if self.alarm_stream.is_none() {
            let (stream, handle) = OutputStream::try_default()?;
            let sink = Sink::try_new(&handle)?;
            self.alarm_stream = Some((stream, handle, sink));
        }

        // Unwrap is safe here.
        let (_, _, sink) = self.alarm_stream.as_ref().unwrap();

        match &self.alarm_path {
            Some(path) => {
                let file = BufReader::new(File::open(path)?);
                let source = Decoder::new_looped(file)?.delay(Duration::from_millis(50));
                sink.append(source.convert_samples::<f32>());
            }
            None => {
                let default_alarm = include_bytes!("../assets/sound/in_call_alarm.ogg");
                let cursor = Cursor::new(default_alarm);

                let source = Decoder::new_looped(cursor)?.delay(Duration::from_millis(50));
                sink.append(source.convert_samples::<f32>());
            }
        }
        sink.play();

        Ok(())
    }

    pub(crate) fn stop_audio(&mut self) {
        if let Some((_, _, sink)) = self.alarm_stream.as_ref() {
            sink.stop();
            sink.sleep_until_end();
        }
    }
}
