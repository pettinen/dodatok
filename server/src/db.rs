use postgres_types::FromSql;
use serde::Serialize;

use macros::sql_enum;

#[allow(non_camel_case_types)]
#[sql_enum]
pub enum Locale {
    #[name("en-US")]
    en_US,
    #[name("fi-FI")]
    fi_FI,
}

#[sql_enum]
pub enum PasswordChangeReason {
    SessionCompromise,
}

#[derive(PartialEq)]
#[sql_enum]
pub enum Permission {
    ViewUser,
    EditUser,
    DeleteUser,
    IgnoreRateLimits,
}
