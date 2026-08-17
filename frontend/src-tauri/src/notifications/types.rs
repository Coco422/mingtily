use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Option<String>,
    pub title: String,
    pub body: String,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub timeout: NotificationTimeout,
    pub icon: Option<String>,
    pub sound: bool,
    pub actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    RecordingStarted,
    RecordingStopped,
    RecordingPaused,
    RecordingResumed,
    TranscriptionComplete,
    SummaryCompleted,
    SummaryFailed,
    MeetingReminder(u64), // Duration in minutes
    SystemError(String),
    Test, // For testing notifications
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationTimeout {
    Never,
    Seconds(u64),
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub title: String,
    pub action_type: NotificationActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationActionType {
    Button,
    Reply,
}

impl Notification {
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        notification_type: NotificationType,
    ) -> Self {
        Self {
            id: None,
            title: title.into(),
            body: body.into(),
            notification_type,
            priority: NotificationPriority::Normal,
            timeout: NotificationTimeout::Default,
            icon: None,
            sound: true,
            actions: vec![],
        }
    }

    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_timeout(mut self, timeout: NotificationTimeout) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_sound(mut self, sound: bool) -> Self {
        self.sound = sound;
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn add_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }
}

impl Default for NotificationPriority {
    fn default() -> Self {
        NotificationPriority::Normal
    }
}

impl Default for NotificationTimeout {
    fn default() -> Self {
        NotificationTimeout::Default
    }
}

// Helper functions for creating common notifications
impl Notification {
    pub fn recording_started(meeting_name: Option<String>) -> Self {
        Self::recording_started_localized(meeting_name, false)
    }

    pub fn recording_started_localized(meeting_name: Option<String>, zh_cn: bool) -> Self {
        let body = match (meeting_name, zh_cn) {
            (Some(name), true) => format!("已开始录制会议：{name}"),
            (Some(name), false) => format!("Recording started for meeting: {name}"),
            (None, true) => "录音已开始，请告知其他参会者。".to_string(),
            (None, false) => {
                "Recording has started. Please inform others in the meeting that you are recording."
                    .to_string()
            }
        };

        Notification::new("Mingtily", body, NotificationType::RecordingStarted)
            .with_priority(NotificationPriority::High)
            .with_timeout(NotificationTimeout::Seconds(5))
    }

    pub fn recording_stopped() -> Self {
        Self::recording_stopped_localized(false)
    }

    pub fn recording_stopped_localized(zh_cn: bool) -> Self {
        Notification::new(
            "Mingtily",
            if zh_cn {
                "录音已停止并保存"
            } else {
                "Recording has been stopped and saved"
            },
            NotificationType::RecordingStopped,
        )
        .with_priority(NotificationPriority::Normal)
        .with_timeout(NotificationTimeout::Seconds(3))
    }

    pub fn recording_paused() -> Self {
        Self::recording_paused_localized(false)
    }

    pub fn recording_paused_localized(zh_cn: bool) -> Self {
        Notification::new(
            "Mingtily",
            if zh_cn {
                "录音已暂停"
            } else {
                "Recording has been paused"
            },
            NotificationType::RecordingPaused,
        )
        .with_priority(NotificationPriority::Normal)
        .with_timeout(NotificationTimeout::Seconds(3))
    }

    pub fn recording_resumed() -> Self {
        Self::recording_resumed_localized(false)
    }

    pub fn recording_resumed_localized(zh_cn: bool) -> Self {
        Notification::new(
            "Mingtily",
            if zh_cn {
                "录音已继续"
            } else {
                "Recording has been resumed"
            },
            NotificationType::RecordingResumed,
        )
        .with_priority(NotificationPriority::Normal)
        .with_timeout(NotificationTimeout::Seconds(3))
    }

    pub fn transcription_complete(file_path: Option<String>) -> Self {
        Self::transcription_complete_localized(file_path, false)
    }

    pub fn transcription_complete_localized(file_path: Option<String>, zh_cn: bool) -> Self {
        let body = match (file_path, zh_cn) {
            (Some(path), true) => format!("转写已完成并保存至：{path}"),
            (Some(path), false) => format!("Transcription completed and saved to: {path}"),
            (None, true) => "转写已完成".to_string(),
            (None, false) => "Transcription has been completed".to_string(),
        };

        Notification::new("Mingtily", body, NotificationType::TranscriptionComplete)
            .with_priority(NotificationPriority::Normal)
            .with_timeout(NotificationTimeout::Seconds(5))
    }

    pub fn summary_completed_localized(meeting_name: Option<String>, zh_cn: bool) -> Self {
        let body = match (meeting_name, zh_cn) {
            (Some(name), true) => format!("“{name}”的 AI 摘要已生成"),
            (Some(name), false) => format!("The AI summary for “{name}” is ready"),
            (None, true) => "AI 摘要已生成".to_string(),
            (None, false) => "Your AI summary is ready".to_string(),
        };
        Notification::new("Mingtily", body, NotificationType::SummaryCompleted)
            .with_priority(NotificationPriority::Normal)
            .with_timeout(NotificationTimeout::Seconds(5))
    }

    pub fn summary_failed_localized(meeting_name: Option<String>, zh_cn: bool) -> Self {
        let body = match (meeting_name, zh_cn) {
            (Some(name), true) => format!("“{name}”的 AI 摘要生成失败，请返回应用重试"),
            (Some(name), false) => {
                format!("The AI summary for “{name}” failed. Return to Mingtily to retry.")
            }
            (None, true) => "AI 摘要生成失败，请返回应用重试".to_string(),
            (None, false) => {
                "AI summary generation failed. Return to Mingtily to retry.".to_string()
            }
        };
        Notification::new("Mingtily", body, NotificationType::SummaryFailed)
            .with_priority(NotificationPriority::Normal)
            .with_timeout(NotificationTimeout::Seconds(7))
    }

    pub fn meeting_reminder(minutes_until: u64, meeting_title: Option<String>) -> Self {
        Self::meeting_reminder_localized(minutes_until, meeting_title, false)
    }

    pub fn meeting_reminder_localized(
        minutes_until: u64,
        meeting_title: Option<String>,
        zh_cn: bool,
    ) -> Self {
        let body = match (meeting_title, zh_cn) {
            (Some(title), true) => format!("会议“{title}”将在 {minutes_until} 分钟后开始"),
            (Some(title), false) => format!("Meeting '{title}' starts in {minutes_until} minutes"),
            (None, true) => format!("会议将在 {minutes_until} 分钟后开始"),
            (None, false) => format!("Meeting starts in {minutes_until} minutes"),
        };

        Notification::new(
            "Mingtily",
            body,
            NotificationType::MeetingReminder(minutes_until),
        )
        .with_priority(NotificationPriority::High)
        .with_timeout(NotificationTimeout::Seconds(10))
    }

    pub fn system_error(error: impl Into<String>) -> Self {
        let error_string = error.into();
        Notification::new(
            "Mingtily Error",
            error_string.clone(),
            NotificationType::SystemError(error_string),
        )
        .with_priority(NotificationPriority::Critical)
        .with_timeout(NotificationTimeout::Never)
    }

    pub fn test_notification() -> Self {
        Self::test_notification_localized(false)
    }

    pub fn test_notification_localized(zh_cn: bool) -> Self {
        Notification::new(
            "Mingtily",
            if zh_cn {
                "这是一条测试通知，用于确认通知功能正常。"
            } else {
                "This is a test notification to verify the system is working correctly"
            },
            NotificationType::Test,
        )
        .with_priority(NotificationPriority::Normal)
        .with_timeout(NotificationTimeout::Seconds(5))
    }
}
