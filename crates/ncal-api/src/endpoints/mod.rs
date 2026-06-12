mod auth;
mod calendars;
mod contacts;
mod events;
mod holds;
mod sync;
mod user;

pub use auth::{
    create_notion_session, get_notion_login_url, CreateNotionSessionRequest, LoginUrlResponse,
};
pub use calendars::{
    get_calendar_lists, get_colors, CalendarListQuery, CalendarListResult, ColorsResponse,
    GetCalendarListsRequest,
};
pub use contacts::get_contacts;
pub use events::{
    create_event, delete_events, get_event, get_events, update_events, CreateEventMutation,
    CreateEventRequest, DeleteEventMutation, DeleteEventsRequest, EventQuery, GetEventQuery,
    GetEventRequest, GetEventsOk, GetEventsRequest, GetEventsResult, UpdateEventMutation,
    UpdateEventsRequest,
};
pub use holds::{
    create_hold, create_hold_event, delete_hold, get_hold, get_hold_alias_available, get_holds,
    update_hold, update_hold_event, CheckAliasRequest, CheckAliasResponse, CreateHoldEventRequest,
    CreateHoldRequest, CreateHoldResponse, DeleteHoldRequest, GetHoldRequest, GetHoldResponse,
    GetHoldsResponse, UpdateHoldEventRequest,
};
pub use sync::{incremental_sync, IncrementalSyncRequest, SyncTokenInput};
pub use user::{
    get_user, get_user_preferences, remove_account, update_primary_account,
    UpdatePrimaryAccountResponse,
};
