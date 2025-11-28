//! Demonstrates the enhanced text_input widget with focus/blur messages.
//!
//! This example shows:
//! - `on_focus(Fn(String) -> Message)` - receive the current value when focused
//! - `on_blur(Message)` - emit a message when focus is lost
//! - Form validation with inline error display
//! - Tab navigation between fields
//!
//! Run with: `cargo run --example text_input`

use iced::keyboard;
use iced::widget::{
    Id, button, center, column, container, operation, row, text,
};
use iced::{Center, Element, Fill, Subscription, Task};

use sweeten::text_input;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size((500.0, 300.0))
        .title("sweeten • text_input with focus handling")
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Username,
    Password,
}

impl Field {
    fn id(&self) -> Id {
        Id::from(match self {
            Field::Username => "username",
            Field::Password => "password",
        })
    }

    fn placeholder(&self) -> &'static str {
        match self {
            Field::Username => "Enter username",
            Field::Password => "Enter password",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Field::Username => "USERNAME",
            Field::Password => "PASSWORD",
        }
    }

    fn validation_hint(&self) -> &'static str {
        match self {
            Field::Username => "Letters and numbers only",
            Field::Password => "Go to town, but min length is 12!",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    field: Field,
    value: String,
    error: Option<String>,
}

impl Input {
    fn new(field: Field) -> Self {
        Self {
            field,
            value: String::new(),
            error: None,
        }
    }

    fn field(&self) -> Field {
        self.field
    }

    fn value(&self) -> &str {
        &self.value
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn with_value(mut self, value: String) -> Self {
        self.value = value;
        self
    }

    fn validate(mut self) -> Self {
        match self.field {
            Field::Username => {
                if self.value.is_empty() {
                    self.error = Some("Username is required".to_string());
                } else if !self.value.chars().all(|c| c.is_alphanumeric()) {
                    self.error = Some("Letters and numbers only".to_string());
                } else {
                    self.error = None;
                }
            }
            Field::Password => {
                if self.value.is_empty() {
                    self.error = Some("Password is required".to_string());
                } else if self.value.len() < 12 {
                    self.error = Some(
                        "Password must be at least 12 characters".to_string(),
                    );
                } else {
                    self.error = None;
                }
            }
        }
        self
    }
}

#[derive(Debug)]
struct App {
    username: Input,
    password: Input,
    focused_field: Option<Field>,
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(Field, String),
    InputFocused(Field),
    InputBlurred(Field),
    SubmitForm,
    FocusNext,
    FocusPrevious,
    FocusedId(Id),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                username: Input::new(Field::Username),
                password: Input::new(Field::Password),
                focused_field: None,
            },
            Task::done(Message::FocusNext),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(field, value) => match field {
                Field::Username => {
                    self.username =
                        self.username.clone().with_value(value).validate();
                }
                Field::Password => {
                    self.password =
                        self.password.clone().with_value(value).validate();
                }
            },
            Message::InputFocused(field) => {
                self.focused_field = Some(field);
            }
            Message::InputBlurred(field) => {
                if self.focused_field == Some(field) {
                    self.focused_field = None;
                }
            }
            Message::SubmitForm => {
                self.username = self.username.clone().validate();
                self.password = self.password.clone().validate();

                if self.form_is_valid() {
                    println!("Form submitted successfully!");
                } else {
                    // Focus the first invalid field
                    let field_to_focus = if self.username.error().is_some() {
                        Field::Username
                    } else {
                        Field::Password
                    };

                    return operation::focus(field_to_focus.id());
                }
            }
            Message::FocusNext => {
                return sweeten::text_input::focus_next().discard();
            }
            Message::FocusPrevious => {
                return sweeten::text_input::focus_previous()
                    .map(Message::FocusedId);
            }
            Message::FocusedId(id) => {
                println!("focused: {id:?}");
            }
        }
        Task::none()
    }

    fn form_is_valid(&self) -> bool {
        !self.username.value().is_empty()
            && !self.password.value().is_empty()
            && self.username.error().is_none()
            && self.password.error().is_none()
    }

    fn view(&self) -> Element<'_, Message> {
        let create_field_view = |input: &Input| {
            let field = input.field();
            let value = input.value();
            let is_focused = self.focused_field == Some(field);

            let input_widget = text_input(field.placeholder(), value)
                .id(field.id())
                .on_input(move |text| Message::InputChanged(field, text))
                .on_focus(Message::InputFocused(field))
                .on_blur(Message::InputBlurred(field))
                .width(Fill)
                .secure(field == Field::Password);

            let status_text_content = if let Some(error) = input.error() {
                format!("Error: {error}")
            } else if is_focused {
                field.validation_hint().to_string()
            } else {
                String::default()
            };

            let status_text = text(status_text_content).size(10.0).style(
                if input.error().is_some() {
                    text::danger
                } else {
                    text::primary
                },
            );

            column![text(field.label()), input_widget, status_text].spacing(5)
        };

        let submit_button = button(text("Submit").center())
            .on_press_maybe(self.form_is_valid().then_some(Message::SubmitForm))
            .width(120);

        let form_status_content = if self.username.error().is_some()
            || self.password.error().is_some()
        {
            "Please fix the errors above"
        } else if self.form_is_valid() {
            "Form is valid!"
        } else {
            ""
        };

        let form_status =
            text(form_status_content).style(if self.form_is_valid() {
                text::success
            } else {
                text::danger
            });

        center(
            column![
                create_field_view(&self.username),
                create_field_view(&self.password),
                row![form_status, container(submit_button).align_right(Fill)]
                    .spacing(20)
                    .align_y(Center)
            ]
            .width(400)
            .align_x(Center)
            .spacing(20),
        )
        .padding(20)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        use iced::event::{self, Event};
        use iced::keyboard::{Key, key::Named};

        event::listen_with(|event, _, _| match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                ..
            }) => match key {
                Key::Named(Named::Tab) => {
                    if modifiers.shift() {
                        Some(Message::FocusPrevious)
                    } else {
                        Some(Message::FocusNext)
                    }
                }
                Key::Named(Named::Enter) => Some(Message::SubmitForm),
                _ => None,
            },
            _ => None,
        })
    }
}
