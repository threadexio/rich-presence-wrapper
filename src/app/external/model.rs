use serde::Deserialize;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum Action {
    Update(Box<Activity>),
    Clear,
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub struct Activity {
    pub name: Option<String>,

    pub state: Option<String>,
    pub state_url: Option<String>,

    pub details: Option<String>,
    pub details_url: Option<String>,

    pub timestamps: Option<Timestamps>,
    pub party: Option<Party>,
    pub assets: Option<Assets>,
    pub buttons: Option<Vec<Button>>,

    pub activity_type: Option<ActivityType>,
    pub status_display_type: Option<StatusDisplayType>,
}

#[derive(Debug, Deserialize)]
pub struct Timestamps {
    pub start: Option<i64>,
    pub end: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct Party {
    pub id: Option<String>,
    pub size: Option<[i32; 2]>,
}

#[derive(Debug, Deserialize)]
pub struct Assets {
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub large_url: Option<String>,

    pub small_image: Option<String>,
    pub small_text: Option<String>,
    pub small_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Button {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub enum ActivityType {
    Playing,
    Listening,
    Watching,
    Competing,
}

#[derive(Debug, Deserialize)]
pub enum StatusDisplayType {
    Name,
    State,
    Details,
}

impl From<Activity> for crate::discord::Activity<'static> {
    fn from(activity: Activity) -> Self {
        use crate::discord::*;

        let mut x = Activity::new();

        if let Some(name) = activity.name {
            x = x.name(name);
        }

        if let Some(state) = activity.state {
            x = x.state(state);
        }

        if let Some(state_url) = activity.state_url {
            x = x.state_url(state_url);
        }

        if let Some(details) = activity.details {
            x = x.details(details);
        }

        if let Some(details_url) = activity.details_url {
            x = x.details_url(details_url);
        }

        if let Some(timestamps) = activity.timestamps {
            let mut y = Timestamps::new();

            if let Some(start) = timestamps.start {
                y = y.start(start);
            }

            if let Some(end) = timestamps.end {
                y = y.end(end);
            }

            x = x.timestamps(y);
        }

        if let Some(party) = activity.party {
            let mut y = Party::new();

            if let Some(id) = party.id {
                y = y.id(id);
            }

            if let Some(size) = party.size {
                y = y.size(size);
            }

            x = x.party(y);
        }

        if let Some(assets) = activity.assets {
            let mut y = Assets::new();

            if let Some(large_image) = assets.large_image {
                y = y.large_image(large_image);
            }

            if let Some(large_text) = assets.large_text {
                y = y.large_text(large_text);
            }

            if let Some(large_url) = assets.large_url {
                y = y.large_url(large_url);
            }

            if let Some(small_image) = assets.small_image {
                y = y.small_image(small_image);
            }

            if let Some(small_text) = assets.small_text {
                y = y.small_text(small_text);
            }

            if let Some(small_url) = assets.small_url {
                y = y.small_url(small_url);
            }

            x = x.assets(y);
        }

        if let Some(buttons) = activity.buttons {
            x = x.buttons(
                buttons
                    .into_iter()
                    .map(|x| Button::new(x.label, x.url))
                    .collect(),
            );
        }

        if let Some(activity_type) = activity.activity_type {
            x = x.activity_type(match activity_type {
                self::ActivityType::Playing => ActivityType::Playing,
                self::ActivityType::Listening => ActivityType::Listening,
                self::ActivityType::Watching => ActivityType::Watching,
                self::ActivityType::Competing => ActivityType::Competing,
            });
        }

        if let Some(status_display_type) = activity.status_display_type {
            x = x.status_display_type(match status_display_type {
                self::StatusDisplayType::Name => StatusDisplayType::Name,
                self::StatusDisplayType::State => StatusDisplayType::State,
                self::StatusDisplayType::Details => StatusDisplayType::Details,
            });
        }

        x
    }
}
