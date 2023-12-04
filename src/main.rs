//! A simple cross-platform timer app.

mod audio;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use iced::{
    alignment::Horizontal,
    executor, font, theme,
    widget::{button, column, container, row, text},
    window, Alignment, Application, Command, Element, Font, Length, Renderer, Settings,
    Subscription, Theme,
};
use rodio::{OutputStream, OutputStreamHandle, Sink};

const DEFAULT_FONT: Font = Font {
    family: font::Family::Name("Source Sans 3"),
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
    monospaced: false,
};

const SEMIBOLD_FONT: Font = Font {
    family: font::Family::Name("Source Sans 3"),
    weight: font::Weight::Semibold,
    stretch: font::Stretch::Normal,
    monospaced: false,
};

#[derive(Clone, Debug)]
enum Message {
    UpdateTimer(Duration),
    Tick,
    EnableTimer,
    TogglePause,
    ResetTimer,
    StopRinging,
    FontLoaded(Result<(), font::Error>),
}

#[derive(Debug)]
enum IsPaused {
    Paused { pause_start: Instant },
    NotPaused,
}

#[derive(Debug)]
enum TimerAppState {
    Started {
        start_instant: Instant,
        time_left: Duration,
        total_wait: Duration,
        is_paused: IsPaused,
    },
    Stopped,
    Ringing,
}

struct TimerApp {
    state: TimerAppState,

    // TODO: Things to be loaded from settings/state.
    to_wait: Duration,
    alarm_path: Option<PathBuf>,
    alarm_stream: Option<(OutputStream, OutputStreamHandle, Sink)>,
}

impl Application for TimerApp {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let app = TimerApp {
            state: TimerAppState::Stopped {},
            to_wait: Duration::from_secs(5 * 60), // Default to 5 minutes
            alarm_path: None,
            alarm_stream: None,
        };

        let command = Command::batch(vec![
            font::load(include_bytes!("../assets/fonts/SourceSans3-Regular.ttf").as_slice())
                .map(Message::FontLoaded),
            font::load(include_bytes!("../assets/fonts/SourceSans3-SemiBold.ttf").as_slice())
                .map(Message::FontLoaded),
        ]);

        (app, command)
    }

    fn title(&self) -> String {
        "Timers".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::FontLoaded(res) => {
                match res {
                    Ok(_) => {}
                    Err(err) => {
                        println!("Failed to load font: {err:?}");
                    }
                }

                return Command::none();
            }
            Message::ResetTimer => {
                self.state = TimerAppState::Stopped;
                self.stop_audio();
                return Command::none();
            }
            _ => (),
        }

        match &mut self.state {
            TimerAppState::Stopped => match message {
                Message::UpdateTimer(new_duration) => {
                    self.to_wait = new_duration;
                }
                Message::EnableTimer => {
                    self.state = TimerAppState::Started {
                        start_instant: Instant::now(),
                        time_left: self.to_wait.clone(),
                        total_wait: self.to_wait.clone(),
                        is_paused: IsPaused::NotPaused,
                    }
                }
                _ => {}
            },
            TimerAppState::Started {
                start_instant,
                time_left,
                total_wait,
                is_paused,
            } => match message {
                Message::Tick => {
                    let new_duration = total_wait.saturating_sub(start_instant.elapsed());
                    *time_left = new_duration;

                    if new_duration.is_zero() {
                        self.play_audio().unwrap();
                        self.state = TimerAppState::Ringing;
                    }
                }
                Message::TogglePause => match is_paused {
                    IsPaused::Paused { pause_start } => {
                        *start_instant += pause_start.elapsed();
                        *is_paused = IsPaused::NotPaused;
                    }
                    IsPaused::NotPaused => {
                        *is_paused = IsPaused::Paused {
                            pause_start: Instant::now(),
                        }
                    }
                },
                _ => {}
            },
            TimerAppState::Ringing => match message {
                Message::StopRinging => {
                    self.stop_audio();
                }
                _ => {}
            },
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        fn parse_duration(duration: Duration) -> String {
            let total_secs = duration.as_secs();

            let hours = total_secs / (60 * 60);
            let minutes = (total_secs % (60 * 60)) / 60;
            let seconds = total_secs % 60;

            if hours > 0 {
                format!("{hours}h {minutes}m {seconds:0>2}s")
            } else if minutes > 0 {
                format!("{minutes}m {seconds:0>2}s")
            } else {
                format!("{seconds}s")
            }
        }

        let (duration, left_button, right_button) = match &self.state {
            TimerAppState::Started {
                time_left,
                is_paused,
                ..
            } => {
                let duration = text(parse_duration(*time_left));

                let left_button = button(
                    text(match is_paused {
                        IsPaused::Paused { .. } => "Resume",
                        IsPaused::NotPaused { .. } => "Pause",
                    })
                    .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10)
                .on_press(Message::TogglePause);

                let right_button = button(text("Reset").horizontal_alignment(Horizontal::Center))
                    .width(90)
                    .padding(10)
                    .on_press(Message::ResetTimer);

                (duration, left_button, right_button)
            }
            TimerAppState::Stopped => {
                let duration = text(parse_duration(self.to_wait));

                let left_button = button(text("Start").horizontal_alignment(Horizontal::Center))
                    .width(90)
                    .padding(10)
                    .on_press(Message::EnableTimer);

                let right_button = button(text("Reset").horizontal_alignment(Horizontal::Center))
                    .width(90)
                    .padding(10);

                (duration, left_button, right_button)
            }
            TimerAppState::Ringing => {
                let duration = text("0s");

                let left_button = button(text("Okay").horizontal_alignment(Horizontal::Center))
                    .width(90)
                    .padding(10)
                    .on_press(Message::StopRinging);

                let right_button = button(text("Reset").horizontal_alignment(Horizontal::Center))
                    .width(90)
                    .padding(10)
                    .on_press(Message::ResetTimer);

                (duration, left_button, right_button)
            }
        };

        let buttons = row!(
            left_button.style(theme::Button::Primary),
            right_button.style(theme::Button::Secondary),
        )
        .spacing(40);

        let content = column![duration.size(60).font(SEMIBOLD_FONT), buttons]
            .align_items(Alignment::Center)
            .spacing(20)
            .max_width(600);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        match &self.state {
            TimerAppState::Started { is_paused, .. } => match is_paused {
                IsPaused::Paused { .. } => Subscription::none(),
                IsPaused::NotPaused => {
                    iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
                }
            },
            TimerAppState::Stopped => Subscription::none(),
            TimerAppState::Ringing => {
                // This is a bit silly but this is a fast way to not have to import more crates on my end so...

                iced::time::every(Duration::from_secs(60)).map(|_| Message::StopRinging)
            }
        }
    }
}

fn main() -> iced::Result {
    TimerApp::run(Settings {
        antialiasing: true,
        window: window::Settings {
            size: (400, 600),
            resizable: true,
            decorations: true,
            ..Default::default()
        },
        default_font: DEFAULT_FONT,
        ..Default::default()
    })
}
