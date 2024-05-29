//! A simple cross-platform timer app.

mod audio;
mod num_input_container;
mod styling;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

// Q: Why is there this "text as textt" thing?
// Because rustfmt tries to merge the imports which breaks using the "text" function widget.
// e.g. text::{self, LineHeight} - this then makes calling the "text" function break. Fun!
// But why not something smarter? Because working with iced is frustrating enough that I don't care
// as long as it works enough.
use iced::{
    alignment::Horizontal,
    executor, font, keyboard, theme,
    widget::{button, column, container, row, text as textt, text::LineHeight},
    window, Alignment, Application, Command, Element, Font, Length, Settings, Size, Subscription,
    Theme,
};
use num_input_container::NumInputContainer;
use rodio::{OutputStream, OutputStreamHandle, Sink};

use crate::styling::text::{DEFAULT_TEXT_COLOR, DISABLED_TEXT_COLOR};

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
    // DisableEditTimer,
    EditNewNum(u32),
    EditBackspace,
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

#[derive(Clone, Debug)]
enum EditingState {
    Editing(String),
    NotEditing,
}

fn human_duration(duration: Duration) -> (u64, u64, u64) {
    // Ugly way to make it so it doesn't immediately round down to the nearest second.
    let total_secs_f64 = duration.as_secs_f64();
    let total_secs = if total_secs_f64 - total_secs_f64.trunc() > 0.1 {
        total_secs_f64.ceil() as u64
    } else {
        total_secs_f64.floor() as u64
    };

    let hours = total_secs / (60 * 60);
    let minutes = (total_secs % (60 * 60)) / 60;
    let seconds = total_secs % 60;

    (hours, minutes, seconds)
}

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

// TODO: 2x calls to this are probably unnecessary, can dedupe it.
fn string_to_hms(s: &str) -> (Option<u32>, Option<u32>, Option<u32>) {
    let mut iter = s.chars();

    let seconds = if let Some(c) = iter.next_back() {
        let mut num = 0;
        num += c.to_digit(10).unwrap();

        if let Some(c) = iter.next_back() {
            num += c.to_digit(10).unwrap() * 10;
        }

        Some(num)
    } else {
        None
    };

    let minutes = if let Some(c) = iter.next_back() {
        let mut num = 0;
        num += c.to_digit(10).unwrap();

        if let Some(c) = iter.next_back() {
            num += c.to_digit(10).unwrap() * 10;
        }

        Some(num)
    } else {
        None
    };

    let hours = if let Some(c) = iter.next() {
        let mut num = 0;
        num += c.to_digit(10).unwrap();

        while let Some(c) = iter.next_back() {
            num *= 10;
            num += c.to_digit(10).unwrap();
        }

        Some(num)
    } else {
        None
    };

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

impl TimerApp {
    fn update_to_wait_from_str(&mut self, s: &str) {
        let (hours, minutes, seconds) = string_to_hms(s);

        self.to_wait = Duration::from_secs(
            (hours.unwrap_or(0) * 60 * 60 + minutes.unwrap_or(0) * 60 + seconds.unwrap_or(0))
                .into(),
        );
    }
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
        let duration = match self.state {
            TimerAppState::Started { time_left, .. } => time_left,
            _ => self.to_wait,
        };

        let (hours, minutes, seconds) = human_duration(duration);
        let title_time = if hours > 0 {
            format!("{hours:0>2}:{minutes:0>2}:{seconds:0>2}")
        } else {
            format!("{minutes:0>2}:{seconds:0>2}")
        };

        format!("Timerys - {title_time}",)
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
                Message::EditNewNum(new_digit) => {
                    let current = match &mut self.is_editing {
                        EditingState::Editing(old_state) => {
                            // TODO: For now, limit to 6 digits, can support more in the future.
                            if old_state.len() >= 6 {
                                return Command::none();
                            }

                            old_state.push_str(&new_digit.to_string());
                            old_state.clone()
                        }
                        EditingState::NotEditing => {
                            // This shouldn't happen, but if it does, then just flip things on.
                            let s = new_digit.to_string();
                            self.is_editing = EditingState::Editing(s.clone());
                            s
                        }
                    };

                    self.update_to_wait_from_str(&current);
                }
                Message::EditBackspace => {
                    let current = match &mut self.is_editing {
                        EditingState::Editing(old_state) => {
                            old_state.pop();
                            old_state.clone()
                        }
                        EditingState::NotEditing => {
                            // This shouldn't happen, but if it does, then it would just be the empty string anyway.
                            // Flip it on and return the empty string.
                            self.is_editing = EditingState::Editing(String::new());
                            String::new()
                        }
                    };

                    self.update_to_wait_from_str(&current);
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
                    self.is_editing = EditingState::Editing(String::new());
                }
                // Message::DisableEditTimer => {
                //     self.is_editing = EditingState::NotEditing;
                // }
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
        let mut content = column![]
            .align_items(Alignment::Center)
            .spacing(20)
            .max_width(600);

        let mut is_editing = false;

        let (left_button, right_button) = match &self.state {
            TimerAppState::Stopped => {
                if let EditingState::Editing(s) = &self.is_editing {
                    is_editing = true;

                    // Assuming the "string" is something like hh(...)mmss, where hh can be any number of digits:
                    let (hours, minutes, seconds) = string_to_hms(&s);

                    let (curr_hours, curr_minutes, curr_seconds) = human_duration(self.to_wait);
                    let mut wrapper = row!().spacing(10);

                    let h_val = match hours {
                        Some(hours) => format!("{hours:0>2}"),
                        None => format!("{curr_hours:0>2}"),
                    };

                    let m_val = match minutes {
                        Some(minutes) => format!("{minutes:0>2}"),
                        None => format!("{curr_minutes:0>2}"),
                    };

                    let s_val = match seconds {
                        Some(seconds) => format!("{seconds:0>2}"),
                        None => format!("{curr_seconds:0>2}"),
                    };

                    let hour_style = theme::Text::Color(if hours.is_none() {
                        DISABLED_TEXT_COLOR
                    } else {
                        DEFAULT_TEXT_COLOR
                    });

                    wrapper = wrapper.push(
                        row!(
                            textt(&h_val)
                                .size(TIME_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .width(TIME_FONT_SIZE)
                                .style(hour_style),
                            textt("h")
                                .size(UNIT_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                                .style(hour_style),
                        )
                        .align_items(Alignment::End),
                    );

                    let minute_style = theme::Text::Color(if minutes.is_none() {
                        DISABLED_TEXT_COLOR
                    } else {
                        DEFAULT_TEXT_COLOR
                    });

                    wrapper = wrapper.push(
                        row!(
                            textt(&m_val)
                                .size(TIME_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .width(TIME_FONT_SIZE)
                                .style(minute_style),
                            textt("m")
                                .size(UNIT_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                                .style(minute_style),
                        )
                        .align_items(Alignment::End),
                    );

                    let second_style = theme::Text::Color(if seconds.is_none() {
                        DISABLED_TEXT_COLOR
                    } else {
                        DEFAULT_TEXT_COLOR
                    });

                    // TODO: Ideally stick a cursor line here between the number and s, but that's a pain to do
                    // right now in iced.
                    wrapper = wrapper.push(
                        row!(
                            textt(&s_val)
                                .size(TIME_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .width(TIME_FONT_SIZE)
                                .style(second_style),
                            textt("s")
                                .size(UNIT_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                                .style(second_style),
                        )
                        .align_items(Alignment::End),
                    );

                    // TODO: Ideally wrap this in a container with just one border on the bottom - but you can't
                    // do that in iced right now!
                    content = content.push(wrapper);
                } else {
                    let durations = parse_duration(self.to_wait);
                    let mut displayed_duration = row!().spacing(10);
                    for (amount, unit) in durations {
                        displayed_duration = displayed_duration.push(
                            row!(
                                textt(amount).size(TIME_FONT_SIZE).font(SEMIBOLD_FONT),
                                textt(unit)
                                    .size(UNIT_FONT_SIZE)
                                    .font(SEMIBOLD_FONT)
                                    .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                            )
                            .align_items(Alignment::End),
                        );
                    }

                    // This is so jankkkkkk.
                    let edit_button_wrapper = button(displayed_duration)
                        .style(theme::Button::custom(styling::button::Transparent))
                        .padding(0)
                        .on_press(Message::EnableEditTimer);

                    content = content.push(edit_button_wrapper);
                }

                let left_button = button(
                    textt("Start")
                        .size(BUTTON_FONT_SIZE)
                        .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10)
                .on_press(Message::EnableTimer);

                let right_button = button(
                    textt("Reset")
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
                            textt(amount).size(TIME_FONT_SIZE).font(SEMIBOLD_FONT),
                            textt(unit)
                                .size(UNIT_FONT_SIZE)
                                .font(SEMIBOLD_FONT)
                                .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                        )
                        .align_items(Alignment::End),
                    );
                }

                content = content.push(displayed_duration);

                let left_button = button(
                    textt(match is_paused {
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
                    textt("Reset")
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
                    row!(row!(
                        textt("0").size(TIME_FONT_SIZE).font(SEMIBOLD_FONT),
                        textt("s")
                            .size(UNIT_FONT_SIZE)
                            .font(SEMIBOLD_FONT)
                            .line_height(LineHeight::Absolute(TIME_FONT_SIZE.into()))
                    )
                    .align_items(Alignment::End))
                    .spacing(10)
                    .align_items(Alignment::End),
                );

                let left_button = button(
                    textt("Okay")
                        .size(BUTTON_FONT_SIZE)
                        .horizontal_alignment(Horizontal::Center),
                )
                .width(90)
                .padding(10)
                .on_press(Message::StopRinging);

                let right_button = button(
                    textt("Reset")
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

        NumInputContainer::new(
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y(),
            Box::new(Message::EditNewNum),
            Box::new(|| Message::EditBackspace),
            !is_editing,
        )
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
            TimerAppState::Stopped => match self.is_editing {
                EditingState::Editing(_) => keyboard::on_key_press(|key, _modifier| match key {
                    keyboard::Key::Named(keyboard::key::Named::Enter) => Some(Message::EnableTimer),
                    _ => None,
                }),
                EditingState::NotEditing => Subscription::none(),
            },
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
            min_size: Some(Size::new(400.0, 600.0)),
            max_size: Some(Size::new(400.0, 600.0)),
            resizable: true,
            decorations: true,
            ..Default::default()
        },
        default_font: DEFAULT_FONT,
        ..Default::default()
    })
}
