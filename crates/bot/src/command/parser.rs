use super::model::{Command, DictAddOption, DictRemoveOption, TimeChannelOption};
use serenity::model::application::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOptionValue,
};

pub fn parse(cmd: &ApplicationCommandInteraction) -> Command {
    match cmd.data.name.as_str() {
        "v" => Command::VoiceToggle,
        "skip" => Command::Skip,
        "dict" => parse_dict(cmd),
        "help" => Command::Help,
        "time" => parse_time(cmd),
        _ => Command::Unknown,
    }
}

fn parse_dict(cmd: &ApplicationCommandInteraction) -> Command {
    let option_dict = match cmd.data.options.get(0) {
        Some(option) => option,
        None => return Command::Unknown,
    };

    match option_dict.name.as_str() {
        "add" => {
            let option_word = match option_dict.options.get(0) {
                Some(x) => x,
                None => return Command::Unknown,
            };
            let option_read_as = match option_dict.options.get(1) {
                Some(x) => x,
                None => return Command::Unknown,
            };
            let word = match &option_word.resolved {
                Some(CommandDataOptionValue::String(x)) => x,
                _ => return Command::Unknown,
            };
            let read_as = match &option_read_as.resolved {
                Some(CommandDataOptionValue::String(x)) => x,
                _ => return Command::Unknown,
            };

            Command::DictAdd(DictAddOption {
                word: word.clone(),
                read_as: read_as.clone(),
            })
        }
        "remove" => {
            let option_word = match option_dict.options.get(0) {
                Some(x) => x,
                None => return Command::Unknown,
            };
            let word = match &option_word.resolved {
                Some(CommandDataOptionValue::String(x)) => x,
                _ => return Command::Unknown,
            };

            Command::DictRemove(DictRemoveOption { word: word.clone() })
        }
        "list" => Command::DictList,
        _ => Command::Unknown,
    }
}

fn parse_time(cmd: &ApplicationCommandInteraction) -> Command {
    if cmd.data.options.is_empty() {
        return Command::TimeToggle;
    }

    let option = match cmd.data.options.get(0) {
        Some(option) => option,
        None => return Command::TimeToggle,
    };

    match option.name.as_str() {
        "toggle" => Command::TimeToggle,
        "channel" => {
            let channel_option = match option.options.get(0) {
                Some(x) => x,
                None => return Command::Unknown,
            };
            let channel_id = match &channel_option.resolved {
                Some(CommandDataOptionValue::Channel(channel)) => channel.id.0,
                _ => return Command::Unknown,
            };

            Command::TimeChannel(TimeChannelOption { channel_id })
        }
        _ => Command::Unknown,
    }
}
