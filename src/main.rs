//! A simple cross-platform timer app.

mod audio;
mod styling;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use iced::{
    alignment::Horizontal,
    executor, font, theme,
    widget::{button, column, container, row, text, text::LineHeight, text_input},
    window, Alignment, Application, Command, Element, Font, Length, Settings, Size, Subscription,
    Theme,
};
use rodio::{OutputStream, OutputStreamHandle, Sink};

use crate::styling::{DEFAULT_TEXT_COLOR, DISABLED_TEXT_COLOR};

const DEFAULT_FONT: Font = Font {
    family: font::Family::Name("Source Sans 3"),
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};

const SEMIBOLD_FONT: Font = Font {
    family: font::Family::Name("Source Sans 3"),
    weight: font::Weight::Semibold,
    stretch: font::Stretch::Normal,
    style: font::Style::Normal,
};

const TIME_FONT_SIZE: u16 = 80;
const UNIT_FONT_SIZE: u16 = 30;
const BUTTON_FONT_SIZE: u16 = 18;

#[derive(Clone, Debug)]
enum Message {
    EnableEditTimer,
    UpdateTimer(EditTimerState),
    Tick,
    EnableTimer,
    TogglePause,
    ResetTimer,
    StopRinging,
    FontLoaded(Result<(), font::Error>),

    // Dummy message for some cases.
    Ignore,
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

#[derive(Clone, Copy, Debug)]
struct EditTimerState {
    hours: Option<u64>,
    minutes: Option<u64>,
    seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
enum EditingState {
    Editing(EditTimerState),
    NotEditing,
}

fn human_duration(duration: Duration) -> (u64, u64, u64) {
    let total_secs = duration.as_secs();

    let hours = total_secs / (60 * 60);
    let minutes = (total_secs % (60 * 60)) / 60;
    let seconds = total_secs % 60;

    (hours, minutes, seconds)
}

struct TimerApp {
    state: TimerAppState,
    is_editing: EditingState,

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
            is_editing: EditingState::NotEditing,
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
                Message::UpdateTimer(new_state) => {
                    match &mut self.is_editing {
                        EditingState::Editing(old_state) => {
                            *old_state = new_state;
                        }
                        EditingState::NotEditing => {
                            // This shouldn't happen, but if it does, then just flip things on.
                            self.is_editing = EditingState::Editing(new_state);
                        }
                    }

                    self.to_wait = Duration::from_secs(
                        new_state.hours.unwrap_or(0) * 60 * 60
                            + new_state.minutes.unwrap_or(0) * 60
                            + new_state.seconds.unwrap_or(0),
                    );
                }
                Message::EnableTimer => {
                    self.state = TimerAppState::Started {
                        start_instant: Instant::now(),
                        time_left: self.to_wait.clone(),
                        total_wait: self.to_wait.clone(),
                        is_paused: IsPaused::NotPaused,
                    };
                    self.is_editing = EditingState::NotEditing;
                }
                Message::EnableEditTimer => {
                    self.is_editing = EditingState::Editing(EditTimerState {
                        hours: None,
                        minutes: None,
                        seconds: None,
                    });
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

    fn view(&self) -> Element<Self::Message> {
        fn parse_duration(duration: Duration) -> Vec<(String, &'static str)> {
            let (hours, minutes, seconds) = human_duration(duration);

            let mut ret = vec![];

            if hours > 0 {
                ret.push((format!("{hours}"), "h"));
                ret.push((format!("{minutes:0>2}"), "m"));
                ret.push((format!("{seconds:0>2}"), "s"));
            } else if minutes > 0 {
                ret.push((format!("{minutes}"), "m"));
                ret.push((format!("{seconds:0>2}"), "s"));
            } else {
                ret.push((format!("{seconds}"), "s"));
            }

            ret
        }

        let mut content = column![]
            .align_items(Alignment::Center)
            .spacing(20)
            .max_width(600);

        let (left_button, right_button) = match &self.state {
            TimerAppState::Stopped => {
                if let EditingState::Editing(EditTimerState {
                    hours,
                    minutes,
                    seconds,
                }) = self.is_editing
                {
                    let (curr_hours, curr_minutes, curr_seconds) = human_duration(self.to_wait);
                    let mut wrapper = row!().spacing(10);

                    let (h_placeholder, h_val) = match hours {
                        Some(hours) => (String::default(), format!("{hours:0>2}")),
                        None => (format!("{curr_hours:0>2}"), String::default()),
                    };

                    let (m_placeholder, m_val) = match minutes {
                        Some(minutes) => (String::default(), format!("{minutes:0>2}")),
                        None => (format!("{curr_minutes:0>2}"), String::default()),
                    };

                    let (s_placeholder, s_val) = match seconds {
                        Some(seconds) => (String::default(), format!("{seconds:0>2}")),
                        None => (format!("{curr_seconds:0>2}"), String::default()),
                    };

                    wrapper = wrapper.push(
                        row!(
                            text_input(&h_placeholder, &h_val)
                                .size(TIME_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .style(styling::text_input::transparent_style())
                                .width(TIME_FONT_SIZE)
                                .padding(0)
                                .on_input(move |new| {
                                    if new.is_empty() {
                                        Message::UpdateTimer(EditTimerState {
                                            hours: None,
                                            minutes,
                                            seconds,
                                        })
                                    } else if let Ok(new) = new.parse::<u64>() {
                                        Message::UpdateTimer(EditTimerState {
                                            hours: Some(new),
                                            minutes: minutes.or(Some(0)),
                                            seconds: seconds.or(Some(0)),
                                        })
                                    } else {
                                        Message::Ignore
                                    }
                                }),
                            text("h")
                                .size(UNIT_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                                .style(theme::Text::Color(if h_val.is_empty() {
                                    DISABLED_TEXT_COLOR
                                } else {
                                    DEFAULT_TEXT_COLOR
                                }))
                        )
                        .align_items(Alignment::End),
                    );

                    wrapper = wrapper.push(
                        row!(
                            text_input(&m_placeholder, &m_val)
                                .size(TIME_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .style(styling::text_input::transparent_style())
                                .width(TIME_FONT_SIZE)
                                .padding(0)
                                .on_input(move |new| {
                                    if new.is_empty() {
                                        Message::UpdateTimer(EditTimerState {
                                            hours,
                                            minutes: if hours.is_some() { Some(0) } else { None },
                                            seconds,
                                        })
                                    } else if let Ok(new) = new.parse::<u64>() {
                                        Message::UpdateTimer(EditTimerState {
                                            hours,
                                            minutes: Some(new),
                                            seconds: seconds.or(Some(0)),
                                        })
                                    } else {
                                        Message::Ignore
                                    }
                                }),
                            text("m")
                                .size(UNIT_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                                .style(theme::Text::Color(if m_val.is_empty() {
                                    DISABLED_TEXT_COLOR
                                } else {
                                    DEFAULT_TEXT_COLOR
                                }))
                        )
                        .align_items(Alignment::End),
                    );

                    wrapper = wrapper.push(
                        row!(
                            text_input(&s_placeholder, &s_val)
                                .size(TIME_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .style(styling::text_input::transparent_style())
                                .width(TIME_FONT_SIZE)
                                .padding(0)
                                .on_input(move |new| {
                                    if new.is_empty() {
                                        Message::UpdateTimer(EditTimerState {
                                            hours,
                                            minutes,
                                            seconds: if hours.is_some() || minutes.is_some() {
                                                Some(0)
                                            } else {
                                                None
                                            },
                                        })
                                    } else if let Ok(new) = new.parse::<u64>() {
                                        Message::UpdateTimer(EditTimerState {
                                            hours,
                                            minutes,
                                            seconds: Some(new),
                                        })
                                    } else {
                                        Message::Ignore
                                    }
                                }),
                            text("s")
                                .size(UNIT_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                                .style(theme::Text::Color(if s_val.is_empty() {
                                    DISABLED_TEXT_COLOR
                                } else {
                                    DEFAULT_TEXT_COLOR
                                }))
                        )
                        .align_items(Alignment::End),
                    );

                    content = content.push(wrapper);
                } else {
                    let durations = parse_duration(self.to_wait);
                    let mut displayed_duration = row!().spacing(10);
                    for (amount, unit) in durations {
                        displayed_duration = displayed_duration.push(
                            row!(
                                text(amount).size(TIME_FONT_SIZE).font(SEMIBOLD_FONT),
                                text(unit)
                                    .size(UNIT_FONT_SIZE)
                                    .font(SEMIBOLD_FONT)
                                    .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                            )
                            .align_items(Alignment::End),
                        );
                    }

                    let wrapper = button(displayed_duration)
                        .style(theme::Button::custom(styling::button::Transparent))
                        .padding(0)
                        .on_press(Message::EnableEditTimer);

                    content = content.push(wrapper);
                }

                let left_button = button(
                    text("Start")
                        .size(BUTTON_FONT_SIZE)
                        .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10)
                .on_press(Message::EnableTimer);

                let right_button = button(
                    text("Reset")
                        .size(BUTTON_FONT_SIZE)
                        .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10);

                (left_button, right_button)
            }
            TimerAppState::Started {
                time_left,
                is_paused,
                ..
            } => {
                let durations = parse_duration(*time_left);
                let mut displayed_duration = row!().spacing(10);
                for (amount, unit) in durations {
                    displayed_duration = displayed_duration.push(
                        row!(
                            text(amount).size(TIME_FONT_SIZE).font(SEMIBOLD_FONT),
                            text(unit)
                                .size(UNIT_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                        )
                        .align_items(Alignment::End),
                    );
                }

                content = content.push(displayed_duration);

                let left_button = button(
                    text(match is_paused {
                        IsPaused::Paused { .. } => "Resume",
                        IsPaused::NotPaused { .. } => "Pause",
                    })
                    .size(BUTTON_FONT_SIZE)
                    .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10)
                .on_press(Message::TogglePause);

                let right_button = button(
                    text("Reset")
                        .size(BUTTON_FONT_SIZE)
                        .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10)
                .on_press(Message::ResetTimer);

                (left_button, right_button)
            }
            TimerAppState::Ringing => {
                content = content.push(
                    row!(
                        text("0").size(TIME_FONT_SIZE).font(SEMIBOLD_FONT),
                        text("s")
                            .size(UNIT_FONT_SIZE)
                            .font(SEMIBOLD_FONT)
                            .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                    )
                    .spacing(10)
                    .align_items(Alignment::End),
                );

                let left_button = button(
                    text("Okay")
                        .size(BUTTON_FONT_SIZE)
                        .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10)
                .on_press(Message::StopRinging);

                let right_button = button(
                    text("Reset")
                        .size(BUTTON_FONT_SIZE)
                        .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10)
                .on_press(Message::ResetTimer);

                (left_button, right_button)
            }
        };

        let buttons = row!(
            left_button.style(theme::Button::Primary),
            right_button.style(theme::Button::Secondary),
        )
        .spacing(40);

        content = content.push(buttons);

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
            size: Size::new(400.0, 600.0),
            resizable: true,
            decorations: true,
            ..Default::default()
        },
        default_font: DEFAULT_FONT,
        ..Default::default()
    })
}
