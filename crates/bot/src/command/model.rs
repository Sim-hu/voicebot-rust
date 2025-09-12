#[derive(Debug, Clone)]
pub enum Command {
    VoiceToggle,
    Skip,
    DictAdd(DictAddOption),
    DictRemove(DictRemoveOption),
    DictList,
    Help,
    TimeToggle,
    TimeChannel(TimeChannelOption),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DictAddOption {
    pub word: String,
    pub read_as: String,
}

#[derive(Debug, Clone)]
pub struct DictRemoveOption {
    pub word: String,
}

#[derive(Debug, Clone)]
pub struct TimeChannelOption {
    pub channel_id: u64,
}
