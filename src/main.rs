use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use iced::{
    executor, keyboard, widget::{
        button, column, container, horizontal_space, pick_list, row, text, text_editor, tooltip,
    }, Application, Command, Element, Font, Length, Settings, Theme
};

use iced::highlighter::{self, Highlighter};
use iced::theme;

fn main() -> iced::Result {
    Editor::run(Settings {
        default_font: Font::MONOSPACE,
        fonts: vec![include_bytes!("../iced-editor-icons.ttf").as_slice().into()],
        ..Settings::default()
    })
}

#[derive(Debug, Clone)]
enum EditorError {
    DialogClosed,
    IO(io::ErrorKind),
}

#[derive(Debug, Clone)]
enum Message {
    New,
    Edit(text_editor::Action),
    Open,
    Save,
    FileOpened(Result<(PathBuf, Arc<String>), EditorError>),
    FileSaved(Result<PathBuf, EditorError>),
    ThemeSelected(highlighter::Theme),
}

struct Editor {
    path: Option<PathBuf>,
    content: text_editor::Content,
    error: Option<EditorError>,
    theme: highlighter::Theme,
    is_dirty: bool,
}
impl Application for Editor {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                path: None,
                content: text_editor::Content::new(),
                error: None,
                theme: highlighter::Theme::SolarizedDark,
                is_dirty: true,
            },
            Command::perform(load_file(default_file()), Message::FileOpened),
        )
    }

    fn title(&self) -> String {
        String::from("ur mom lol")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::Edit(action) => {
                self.is_dirty = self.is_dirty || action.is_edit();
                self.content.edit(action);
                self.error = None;
                Command::none()
            }
            Message::Open => Command::perform(pick_file(), Message::FileOpened),
            Message::FileOpened(Ok((path, content))) => {
                self.is_dirty = false;
                self.path = Some(path);
                self.content = text_editor::Content::with(&content);
                Command::none()
            }
            Message::FileOpened(Err(error)) => {
                println!("{:?}", &error);
                self.error = Some(error);
                Command::none()
            }
            Message::New => {
                self.path = None;
                self.is_dirty = true;
                self.content = text_editor::Content::new();
                Command::none()
            }
            Message::Save => {
                let content = self.content.text();
                Command::perform(save_file(self.path.clone(), content), Message::FileSaved)
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
                self.is_dirty = false;

                Command::none()
            }
            Message::FileSaved(Err(error)) => {
                self.error = Some(error);
                Command::none()
            }
            Message::ThemeSelected(theme) => {
                self.theme = theme;

                Command::none()
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        keyboard::on_key_press(|key_code, modifiers| {
            match key_code {
                keyboard::KeyCode::N if modifiers.command() => Some(Message::New),
                keyboard::KeyCode::O if modifiers.command() => Some(Message::Open),
                keyboard::KeyCode::S if modifiers.command() => Some(Message::Save),
                _ => None,
            }
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![
            action(get_icon(Icon::New), "New...", Some(Message::New)),
            action(get_icon(Icon::Open), "Open...", Some(Message::Open)),
            action(
                get_icon(Icon::Save),
                "Save...",
                self.is_dirty.then_some(Message::Save)
            ),
            horizontal_space(Length::Fill),
            pick_list(
                highlighter::Theme::ALL,
                Some(self.theme),
                Message::ThemeSelected
            )
        ]
        .spacing(10);
        let input = text_editor(&self.content)
            .on_edit(Message::Edit)
            .highlight::<Highlighter>(
                highlighter::Settings {
                    theme: self.theme,
                    extension: self
                        .path
                        .as_ref()
                        .and_then(|path| path.extension()?.to_str())
                        .unwrap_or("rs")
                        .to_string(),
                },
                |highlight, _theme| highlight.to_format(),
            );

        let status_bar = {
            let status = if let Some(EditorError::IO(error)) = self.error.as_ref() {
                text(error.to_string())
            } else {
                match self.path.as_deref().and_then(Path::to_str) {
                    Some(path) => text(path).size(14),
                    None => text("(New File)"),
                }
            };

            let position = {
                let (line, column) = self.content.cursor_position();
                text(format!("{}:{}", line + 1, column + 1))
            };
            row![status, horizontal_space(Length::Fill), position]
        };

        container(column![controls, input, status_bar])
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        if self.theme.is_dark() {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

async fn save_file(path: Option<PathBuf>, text: String) -> Result<PathBuf, EditorError> {
    let path = if let Some(path) = path {
        path
    } else {
        rfd::AsyncFileDialog::new()
            .set_title("Save As...")
            .save_file()
            .await
            .ok_or(EditorError::DialogClosed)
            .map(|handle| handle.path().to_owned())?
    };

    tokio::fs::write(&path, &text)
        .await
        .map_err(|err| EditorError::IO(err.kind()))?;
    Ok(path)
}

fn action<'a>(
    content: Element<'a, Message>,
    label: &str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let is_disabled = on_press.is_none();
    tooltip(
        button(container(content).width(30).center_x())
            .on_press_maybe(on_press)
            .padding([5, 10])
            .style(if is_disabled {
                theme::Button::Secondary
            } else {
                theme::Button::Primary
            }),
        label,
        tooltip::Position::FollowCursor,
    )
    .style(theme::Container::Box)
    .into()
}

enum Icon {
    New,
    Open,
    Save,
}

fn icon<'a>(codepoint: char) -> Element<'a, Message> {
    const ICON_FONT: Font = Font::with_name("iced-editor-icons");

    text(codepoint).font(ICON_FONT).into()
}

fn get_icon<'a>(i: Icon) -> Element<'a, Message> {
    match i {
        Icon::New => icon('\u{E800}'),
        Icon::Open => icon('\u{F114}'),
        Icon::Save => icon('\u{E801}'),
    }
}

fn default_file() -> PathBuf {
    PathBuf::from(format!("{}/src/main.rs", env!("CARGO_MANIFEST_DIR")))
}

async fn pick_file() -> Result<(PathBuf, Arc<String>), EditorError> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Choose a text file...")
        .pick_file()
        .await
        .ok_or(EditorError::DialogClosed)?;

    load_file(handle.path().to_owned()).await
}

async fn load_file(path: PathBuf) -> Result<(PathBuf, Arc<String>), EditorError> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map(Arc::new)
        .map_err(|error| error.kind())
        .map_err(EditorError::IO)?;

    Ok((path, content))
}
