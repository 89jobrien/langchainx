use std::fmt;

/// Qwen model options
#[allow(non_camel_case_types)]
pub enum QwenModel {
    /// Qwen-Max
    QwenMax,
    /// Qwen-Turbo
    QwenTurbo,
    /// Qwen-Plus
    QwenPlus,
    /// Qwen-Long
    QwenLong,
    /// Qwen-72B-Chat (Open Source Version)
    Qwen1_72B_Chat,
    /// Qwen-14B-Chat (Open Source Version)
    Qwen1_14B_Chat,
    /// Qwen-7B-Chat (Open Source Version)
    Qwen1_7B_Chat,
    /// Qwen-1.8B-Chat (Open Source Version)
    Qwen1_1_8B_Chat,
    /// Qwen1.5-110B-Chat (Open Source Version)
    Qwen1_5_110B_Chat,
    /// Qwen1.5-72B-Chat (Open Source Version)
    Qwen1_5_72B_Chat,
    /// Qwen1.5-32B-Chat (Open Source Version)
    Qwen1_5_32B_Chat,
    /// Qwen1.5-14B-Chat (Open Source Version)
    Qwen1_5_14B_Chat,
    /// Qwen1.5-7B-Chat (Open Source Version)
    Qwen1_5_7B_Chat,
    /// Qwen1.5-1.8B-Chat (Open Source Version)
    Qwen1_5_1_8B_Chat,
    /// Qwen1.5-0.5B-Chat (Open Source Version)
    Qwen1_5_0_5B_Chat,
    /// Qwen2-72b-Instruct (Open Source Version)
    QWEN2_72B_INSTRUCT,
    /// Qwen2-57b-a14b-Instruct (Open Source Version)
    QWEN2_57B_A14B_INSTRUCT,
    /// Qwen2-7b-Instruct (Open Source Version)
    QWEN2_7B_INSTRUCT,
    /// Qwen2-1.5b-Instruct (Open Source Version)
    QWEN2_1_5B_INSTRUCT,
    /// Qwen2-0.5b-Instruct (Open Source Version)
    QWEN2_0_5B_INSTRUCT,
    /// Qwen2.5-14B-Instruct-1M (Open Source Version)
    Qwen2_5_14B_INSTRUCT_1M,
    /// Qwen2.5-7B-Instruct-1M (Open Source Version)
    Qwen2_5_7B_INSTRUCT_1M,
    /// Qwen2.5-72B-Instruct (Open Source Version)
    Qwen2_5_72B_INSTRUCT,
    /// Qwen2.5-32B-Instruct (Open Source Version)
    Qwen2_5_32B_INSTRUCT,
    /// Qwen2.5-14B-Instruct (Open Source Version)
    Qwen2_5_14B_INSTRUCT,
    /// Qwen2.5-7B-Instruct (Open Source Version)
    Qwen2_5_7B_INSTRUCT,
    /// Qwen2.5-3B-Instruct (Open Source Version)
    Qwen2_5_3B_INSTRUCT,
    /// Qwen2.5-1.5B-Instruct (Open Source Version)
    Qwen2_5_1_5B_INSTRUCT,
    /// Qwen2.5-0.5B-Instruct (Open Source Version)
    Qwen2_5_0_5B_INSTRUCT,
}

impl fmt::Display for QwenModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            QwenModel::QwenMax => "qwen-max",
            QwenModel::QwenTurbo => "qwen-turbo",
            QwenModel::QwenPlus => "qwen-plus",
            QwenModel::QwenLong => "qwen-long",
            QwenModel::Qwen1_72B_Chat => "qwen-72b-chat",
            QwenModel::Qwen1_14B_Chat => "qwen-14b-chat",
            QwenModel::Qwen1_7B_Chat => "qwen-7b-chat",
            QwenModel::Qwen1_1_8B_Chat => "qwen-1.8b-chat",
            QwenModel::Qwen1_5_110B_Chat => "qwen1.5-110b-chat",
            QwenModel::Qwen1_5_72B_Chat => "qwen-1.72b-chat",
            QwenModel::Qwen1_5_32B_Chat => "qwen1.5-32b-chat",
            QwenModel::Qwen1_5_14B_Chat => "qwen1.5-14b-chat",
            QwenModel::Qwen1_5_7B_Chat => "qwen1.5-7b-chat",
            QwenModel::Qwen1_5_1_8B_Chat => "qwen1.5-1.8b-chat",
            QwenModel::Qwen1_5_0_5B_Chat => "qwen1.5-0.5b-chat",
            QwenModel::QWEN2_72B_INSTRUCT => "qwen2-72b-instruct",
            QwenModel::QWEN2_57B_A14B_INSTRUCT => "qwen2-57b-a14b-instruct",
            QwenModel::QWEN2_7B_INSTRUCT => "qwen2-7b-instruct",
            QwenModel::QWEN2_1_5B_INSTRUCT => "qwen2-1.5-b-instruct",
            QwenModel::QWEN2_0_5B_INSTRUCT => "qwen2-0.5-b-instruct",
            QwenModel::Qwen2_5_14B_INSTRUCT_1M => "qwen2.5-14b-instruct-1m",
            QwenModel::Qwen2_5_7B_INSTRUCT_1M => "qwen2.5-7b-instruct-1m",
            QwenModel::Qwen2_5_72B_INSTRUCT => "qwen2.5-72b-instruct",
            QwenModel::Qwen2_5_32B_INSTRUCT => "qwen2.5-32b-instruct",
            QwenModel::Qwen2_5_14B_INSTRUCT => "qwen2.5-14b-instruct",
            QwenModel::Qwen2_5_7B_INSTRUCT => "qwen2.5-7b-instruct",
            QwenModel::Qwen2_5_3B_INSTRUCT => "qwen2.5-3b-instruct",
            QwenModel::Qwen2_5_1_5B_INSTRUCT => "qwen2.5-1.5b-instruct",
            QwenModel::Qwen2_5_0_5B_INSTRUCT => "qwen2.5-0.5b-instruct",
        };
        write!(f, "{s}")
    }
}
