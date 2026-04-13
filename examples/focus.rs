//! Demonstrates focus handling with sweeten's text_input and button widgets.
//!
//! This example shows:
//! - `on_focus(Message)` / `on_blur(Message)` on text inputs
//! - `on_focus(Message)` / `on_blur(Message)` on buttons
//! - Tab / Shift+Tab navigation between text inputs and buttons
//! - Enter / Space to activate a focused button
//! - Form validation with inline error display
//!
//! Run with: `cargo run --example focus`

use iced::keyboard;
use iced::widget::{Id, center, column, container, row, text};
use iced::{Center, Element, Fill, Subscription, Task};

use sweeten::widget::operation;
use sweeten::widget::{button, text_input};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size((500.0, 350.0))
        .title("sweeten • focus handling")
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Username,
    Password,
}

impl Field {
    fn id(self) -> Id {
        Id::from(match self {
            Field::Username => "username",
            Field::Password => "password",
        })
    }

    fn placeholder(self) -> &'static str {
        match self {
            Field::Username => "Enter username",
            Field::Password => "Enter password",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Field::Username => "USERNAME",
            Field::Password => "PASSWORD",
        }
    }

    fn validation_hint(self) -> &'static str {
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
                } else if !self.value.chars().all(char::is_alphanumeric) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focused {
    Field(Field),
    SubmitButton,
}

#[derive(Debug)]
struct App {
    username: Input,
    password: Input,
    focused: Option<Focused>,
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(Field, String),
    InputFocused(Field),
    InputBlurred(Field),
    ButtonFocused,
    ButtonBlurred,
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
                focused: None,
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
                self.focused = Some(Focused::Field(field));
            }
            Message::InputBlurred(field) => {
                if self.focused == Some(Focused::Field(field)) {
                    self.focused = None;
                }
            }
            Message::ButtonFocused => {
                self.focused = Some(Focused::SubmitButton);
            }
            Message::ButtonBlurred => {
                if self.focused == Some(Focused::SubmitButton) {
                    self.focused = None;
                }
            }
            Message::SubmitForm => {
                self.username = Input::new(Field::Username);
                self.password = Input::new(Field::Password);
                return operation::focus(Field::Username.id());
            }
            Message::FocusNext => {
                return operation::focus_next().discard();
            }
            Message::FocusPrevious => {
                return operation::focus_previous().map(Message::FocusedId);
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
        let valid = self.form_is_valid();

        let create_field_view = |input: &Input| {
            let field = input.field();
            let value = input.value();
            let is_focused = self.focused == Some(Focused::Field(field));

            let input_widget = text_input(field.placeholder(), value)
                .id(field.id())
                .on_input(move |text| Message::InputChanged(field, text))
                .on_focus(Message::InputFocused(field))
                .on_blur(Message::InputBlurred(field))
                .on_submit_maybe(valid.then_some(Message::SubmitForm))
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
            .on_press_maybe(valid.then_some(Message::SubmitForm))
            .on_focus(Message::ButtonFocused)
            .on_blur(Message::ButtonBlurred)
            .width(120);

        let has_errors =
            self.username.error().is_some() || self.password.error().is_some();

        let form_status_content = if has_errors {
            "Please fix the errors above"
        } else if valid {
            "Form is valid!"
        } else {
            ""
        };

        let form_status = text(form_status_content).style(if valid {
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
                    .align_y(Center),
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
        use iced::keyboard::Key;
        use iced::keyboard::key::Named;

        event::listen_with(|event, _, _| match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Tab),
                modifiers,
                ..
            }) => {
                if modifiers.shift() {
                    Some(Message::FocusPrevious)
                } else {
                    Some(Message::FocusNext)
                }
            }
            _ => None,
        })
    }
}
