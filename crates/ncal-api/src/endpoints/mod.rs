mod auth;
mod calendars;
mod contacts;
mod events;
mod sync;
mod user;

pub use auth::{create_notion_session, get_notion_login_url, CreateNotionSessionRequest, LoginUrlResponse};
pub use calendars::{
    get_calendar_lists, get_colors, CalendarListOk, CalendarListQuery, CalendarListResult,
    ColorsResponse, GetCalendarListsRequest,
};
pub use contacts::get_contacts;
pub use events::{
    create_event, delete_events, get_event, get_events, update_events, CreateEventMutation,
    CreateEventRequest, DeleteEventMutation, DeleteEventsRequest, EventQuery, GetEventQuery,
    GetEventRequest, GetEventsOk, GetEventsRequest, GetEventsResult, UpdateEventMutation,
    UpdateEventsRequest,
};
pub use sync::{incremental_sync, IncrementalSyncRequest, SyncTokenInput};
pub use user::{
    get_user, get_user_preferences, remove_account, update_primary_account, UpdatePrimaryAccountResponse,
};
