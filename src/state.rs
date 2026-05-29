use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightState {
    Idle = 1,
    Running = 2,
    ToolError = 3,
    ErrorFinal = 4,
    NeedConfirm = 5,
}

impl LightState {
    pub fn priority(&self) -> u8 {
        *self as u8
    }

    pub fn should_blink(&self) -> bool {
        matches!(
            self,
            LightState::Running | LightState::ToolError | LightState::NeedConfirm
        )
    }

    pub fn blink_interval_ms(&self) -> u64 {
        match self {
            LightState::Running => 900,
            LightState::NeedConfirm => 380,
            LightState::ToolError => 280,
            _ => 500,
        }
    }
}

#[derive(Debug)]
struct Session {
    state: LightState,
}

#[derive(Debug)]
pub struct StateMachine {
    sessions: HashMap<String, Session>,
    session_counter: u32,
    current_state: LightState,
    should_exit: bool,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_counter: 0,
            current_state: LightState::Idle,
            should_exit: false,
        }
    }

    pub fn current_state(&self) -> LightState {
        self.current_state
    }

    pub fn session_count(&self) -> u32 {
        self.session_counter
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub fn handle_event(&mut self, session_id: &str, event: &str) {
        match event {
            "session_start" => {
                let is_new = !self.sessions.contains_key(session_id);
                if is_new {
                    // 真正的新 session：计数 +1，初始化为 Idle
                    self.session_counter += 1;
                    self.sessions.insert(
                        session_id.to_string(),
                        Session { state: LightState::Idle },
                    );
                }
                // is_new=false：running 比 session_start 先到，
                // session 已存在且状态正确，不覆盖。
                // compact/clear 场景下 Stop 事件会负责重置为 Idle。
                if self.session_counter > 0 {
                    self.should_exit = false;
                }
            }

            "session_end" => {
                let existed = self.sessions.remove(session_id).is_some();
                if existed && self.session_counter > 0 {
                    self.session_counter -= 1;
                }
                if self.session_counter == 0 {
                    self.should_exit = true;
                }
            }

            "stop" => {
                if let Some(session) = self.sessions.get_mut(session_id) {
                    if !matches!(session.state, LightState::ErrorFinal) {
                        session.state = LightState::Idle;
                    }
                }
            }

            "running"      => self.set_session_state(session_id, LightState::Running),
            "need_confirm" => self.set_session_state(session_id, LightState::NeedConfirm),
            "tool_error"   => self.set_session_state(session_id, LightState::ToolError),
            "error_final"  => self.set_session_state(session_id, LightState::ErrorFinal),
            "idle"         => self.set_session_state(session_id, LightState::Idle),

            _ => {}
        }

        self.update_current_state();
    }

    fn set_session_state(&mut self, session_id: &str, state: LightState) {
        // 若 session 不存在（事件乱序：running 比 session_start 先到），
        // 隐式创建并同步增加 counter，保证 session_end 时计数对称。
        let is_new = !self.sessions.contains_key(session_id);
        if is_new {
            self.session_counter += 1;
            self.should_exit = false;
        }
        let session = self.sessions
            .entry(session_id.to_string())
            .or_insert(Session { state: LightState::Idle });
        session.state = state;
    }

    fn update_current_state(&mut self) {
        self.current_state = self
            .sessions
            .values()
            .map(|s| s.state)
            .max_by_key(|s| s.priority())
            .unwrap_or(LightState::Idle);
    }

    #[cfg(test)]
    pub fn reset_all(&mut self) {
        self.sessions.clear();
        self.session_counter = 0;
        self.should_exit = false;
        self.current_state = LightState::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_running_before_session_start() {
        // running 先到，session_start 后到
        let mut sm = StateMachine::new();
        sm.handle_event("s1", "running");
        assert_eq!(sm.session_count(), 1);
        assert_eq!(sm.current_state(), LightState::Running); // 闪烁绿灯

        sm.handle_event("s1", "session_start"); // 后到，不覆盖状态
        assert_eq!(sm.session_count(), 1);      // counter 不重复 +1
        assert_eq!(sm.current_state(), LightState::Running); // 仍然闪烁！

        sm.handle_event("s1", "session_end");
        assert_eq!(sm.session_count(), 0);
        assert!(sm.should_exit());
    }

    #[test]
    fn test_session_start_before_running() {
        // session_start 先到（正常顺序）
        let mut sm = StateMachine::new();
        sm.handle_event("s1", "session_start");
        assert_eq!(sm.session_count(), 1);
        assert_eq!(sm.current_state(), LightState::Idle);

        sm.handle_event("s1", "running");
        assert_eq!(sm.current_state(), LightState::Running); // 闪烁绿灯

        sm.handle_event("s1", "stop");
        assert_eq!(sm.current_state(), LightState::Idle);

        sm.handle_event("s1", "session_end");
        assert_eq!(sm.session_count(), 0);
        assert!(sm.should_exit());
    }

    #[test]
    fn test_session_counter_no_leak_on_compact() {
        let mut sm = StateMachine::new();
        sm.handle_event("s1", "session_start");
        assert_eq!(sm.session_count(), 1);
        sm.handle_event("s1", "session_start"); // compact/clear 重复触发
        assert_eq!(sm.session_count(), 1);      // 不累加
        sm.handle_event("s1", "session_end");
        assert_eq!(sm.session_count(), 0);
        assert!(sm.should_exit());
    }

    #[test]
    fn test_multi_session_priority() {
        let mut sm = StateMachine::new();
        sm.handle_event("s1", "session_start");
        sm.handle_event("s2", "session_start");
        sm.handle_event("s1", "running");
        sm.handle_event("s2", "need_confirm");
        assert_eq!(sm.current_state(), LightState::NeedConfirm);
        sm.handle_event("s2", "stop");
        assert_eq!(sm.current_state(), LightState::Running);
    }

    #[test]
    fn test_error_final_not_overwritten_by_stop() {
        let mut sm = StateMachine::new();
        sm.handle_event("s1", "session_start");
        sm.handle_event("s1", "error_final");
        sm.handle_event("s1", "stop");
        assert_eq!(sm.current_state(), LightState::ErrorFinal);
    }

    #[test]
    fn test_two_sessions_end_independently() {
        let mut sm = StateMachine::new();
        sm.handle_event("s1", "session_start");
        sm.handle_event("s2", "session_start");
        assert_eq!(sm.session_count(), 2);
        sm.handle_event("s1", "session_end");
        assert_eq!(sm.session_count(), 1);
        assert!(!sm.should_exit());
        sm.handle_event("s2", "session_end");
        assert_eq!(sm.session_count(), 0);
        assert!(sm.should_exit());
    }
}
