use clap::Args;
use clap::builder::NonEmptyStringValueParser;

#[cfg_attr(test, derive(Default))]
#[derive(Args, Debug)]
#[group(multiple = false)]
pub(super) struct SessionArgs {
    /// Session name to create
    #[arg(short,
        long = "new-name",
        conflicts_with = "attach_name",
        value_parser = NonEmptyStringValueParser::new()
    )]
    pub(super) new_name: Option<String>,
    /// Session name to attach to
    #[arg(short,
        long = "attach",
        conflicts_with = "new_name",
        value_parser = NonEmptyStringValueParser::new()
    )]
    pub(super) attach_name: Option<String>,
}
