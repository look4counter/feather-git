use crate::theme;
use gpui::{prelude::*, *};

fn is_keys(keys: &str) -> bool {
    !keys.is_empty()
        && keys.split(" + ").all(|key| {
            key.chars().count() == 1
                || [
                    "ctrl",
                    "shift",
                    "alt",
                    "enter",
                    "esc",
                    "space",
                    "tab",
                    "home",
                    "end",
                    "방향키",
                ]
                .contains(&key)
                || key
                    .strip_prefix('f')
                    .and_then(|n| n.parse::<u8>().ok())
                    .is_some_and(|n| (1..=24).contains(&n))
        })
}

pub fn keys(keys: &str) -> Div {
    let mut row = div().flex().items_center().gap_1();
    for (index, key) in keys.split(" + ").enumerate() {
        if index > 0 {
            row = row.child(div().text_xs().child("+"));
        }
        row = row.child(
            div()
                .px_1()
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .rounded_sm()
                .border_1()
                .border_color(rgb(theme::LINE))
                .bg(rgb(theme::BUTTON))
                .text_color(rgb(theme::TEXT))
                .text_size(px(11.))
                .child(key.to_owned()),
        );
    }
    row
}

pub fn badge(label: &str) -> Div {
    div()
        .px_2()
        .h(px(20.))
        .min_w(px(20.))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .rounded_sm()
        .bg(rgb(theme::SELECT))
        .text_color(rgb(theme::ACCENT))
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.to_owned())
}

pub fn hint(description: &str, shortcut: &str) -> Div {
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(description.to_owned());
    for (index, shortcut) in shortcut
        .split(" / ")
        .filter(|key| !key.is_empty())
        .enumerate()
    {
        if index > 0 {
            row = row.child(div().text_color(rgb(theme::MUTED)).child("/"));
        }
        row = row.child(keys(shortcut));
    }
    row
}

pub fn label(text: &str) -> Div {
    if let Some((shortcut, description)) = text.split_once(" - ")
        && is_keys(shortcut)
    {
        return div()
            .flex()
            .items_center()
            .gap_2()
            .child(keys(shortcut))
            .child(description.to_owned());
    }
    if let Some((description, shortcut)) = text.split_once(" · ")
        && ["확인", "취소", "닫기", "저장", "삭제", "Clone 실행"].contains(&description)
        && is_keys(shortcut)
    {
        return hint(description, shortcut);
    }
    div().child(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn combinations_accept_keys_and_reject_descriptions() {
        for keys in ["ctrl + s", "shift + ↑", "f12", "?"] {
            assert!(is_keys(keys));
        }
        for text in ["", "Local", "저장 위치 선택", "더블클릭"] {
            assert!(!is_keys(text));
        }
    }
}

pub const GROUPS: [&str; 5] = ["공통", "브랜치", "커밋히스토리", "파일목록", "뷰어"];

pub struct Command {
    pub id: String,
    pub group: &'static str,
    pub scope: &'static str,
    pub name: String,
    pub target: String,
    pub defaults: [String; 2],
}

pub fn subsections(group: &str) -> &'static [&'static str] {
    match group {
        "브랜치" => &["공통", "Local", "Remote"],
        "파일목록" => &["공통", "Stage"],
        _ => &[""],
    }
}

pub fn in_subsection(command: &Command, subsection: &str) -> bool {
    match command.group {
        "브랜치" => match command.id.as_str() {
            "branch-upstream" => subsection == "Local",
            "branch-delete-server" => subsection == "Remote",
            _ => subsection == "공통",
        },
        "파일목록" => {
            if command.id == "files-tree" {
                subsection == "공통"
            } else {
                subsection == "Stage"
            }
        }
        _ => subsection.is_empty(),
    }
}

pub fn list_visible(command: &Command) -> bool {
    !command.id.starts_with("select-") && !["checkout", "reset"].contains(&command.scope)
}

fn scope_name(scope: &str) -> &str {
    match scope {
        "modal" => "도움말·저장소 작업",
        "branch" => "브랜치·태그 작업",
        "checkout" => "체크아웃 후보",
        "reset" => "Reset",
        "stash-modal" => "스태시",
        "commit-modal" => "커밋",
        "discard-modal" => "변경 폐기",
        "settings-modal" => "세팅",
        _ => scope,
    }
}

pub fn commands() -> Vec<Command> {
    let mut result = Vec::new();
    let mut add = |id: &str, group, scope, name: &str, target: &str, first: &str, second: &str| {
        result.push(Command {
            id: id.into(),
            group,
            scope,
            name: name.into(),
            target: target.into(),
            defaults: [first.into(), second.into()],
        });
    };
    for (id, name, key, second) in [
        ("terminal", "터미널", "f9", ""),
        ("explorer", "탐색기", "f10", ""),
        ("shortcuts", "단축키 설정", "f11", ""),
        ("help", "도움말", "?", ""),
        ("settings", "세팅", "f12", ""),
        ("refresh", "새로고침", "f5", "ctrl-r"),
        ("focus-branches", "브랜치 포커스", "1", ""),
        ("focus-history", "커밋히스토리 포커스", "2", ""),
        ("focus-files", "파일목록 포커스", "3", ""),
        ("focus-viewer", "뷰어 포커스", "4", ""),
        ("fetch", "패치", "f", ""),
        ("pull", "풀", "p", ""),
        ("push", "푸시", "shift-p", ""),
    ] {
        let first = match id {
            "pull" => "ctrl-p",
            "fetch" => "ctrl-f",
            "push" => "ctrl-u",
            _ => key,
        };
        add(id, "공통", "*", name, key, first, second);
    }
    for (id, name, key) in [("stash", "스태시", "s"), ("commit", "커밋", "c")] {
        add(id, "파일목록", "stage-files", name, key, key, "");
    }
    add(
        "errors",
        "공통",
        "error-history",
        "오류 기록 열기 / 닫기",
        "f1",
        "f1",
        "",
    );
    for (direction, alias, name) in [
        ("up", "i", "방향키 위"),
        ("left", "j", "방향키 왼쪽"),
        ("down", "k", "방향키 아래"),
        ("right", "l", "방향키 오른쪽"),
    ] {
        add(
            &format!("nav-{direction}"),
            "공통",
            if ["left", "right"].contains(&direction) {
                "horizontal-navigation"
            } else {
                "navigation"
            },
            name,
            direction,
            direction,
            alias,
        );
        add(
            &format!("select-{direction}"),
            "공통",
            "*",
            &format!("{name} / 선택 확장"),
            &format!("shift-{direction}"),
            &format!("shift-{direction}"),
            &format!("shift-{alias}"),
        );
    }
    for (id, group, scope, name, key) in [
        (
            "shortcuts-reset",
            "공통",
            "shortcuts-modal",
            "단축키 기본값 복원",
            "ctrl-d",
        ),
        (
            "project-tab-previous",
            "공통",
            "project-tabs",
            "이전 프로젝트 탭",
            "ctrl-[",
        ),
        (
            "project-tab-next",
            "공통",
            "project-tabs",
            "다음 프로젝트 탭",
            "ctrl-]",
        ),
        (
            "branch-jump",
            "브랜치",
            "branches",
            "선택 참조 위치로 이동",
            "space",
        ),
        (
            "branch-upstream",
            "브랜치",
            "local-branches",
            "원격 브랜치 연결",
            "r",
        ),
        (
            "branch-checkout",
            "공통",
            "checkout-action",
            "체크아웃",
            "o",
        ),
        ("branch-delete", "브랜치", "branches", "선택 참조 삭제", "d"),
        (
            "branch-delete-server",
            "브랜치",
            "branches",
            "서버 원격 브랜치도 삭제",
            "shift-d",
        ),
        (
            "history-branch",
            "커밋히스토리",
            "history",
            "Add Branch",
            "b",
        ),
        ("history-tag", "커밋히스토리", "history", "Add Tag", "t"),
        ("history-reset", "커밋히스토리", "history", "Reset", "x"),
        ("history-revert", "커밋히스토리", "history", "Revert", "v"),
        ("history-merge", "커밋히스토리", "history", "Merge", "m"),
        ("history-rebase", "커밋히스토리", "history", "Rebase", "r"),
        ("files-all", "공통", "selection", "전체 선택", "ctrl-a"),
        ("files-tree", "파일목록", "files", "트리 모드 전환", "`"),
        (
            "files-stage",
            "파일목록",
            "files",
            "Stage / Unstage",
            "space",
        ),
        (
            "files-discard",
            "파일목록",
            "files",
            "선택 파일 변경 폐기 확인",
            "d",
        ),
        ("viewer-home", "뷰어", "viewer", "줄 처음", "home"),
        ("viewer-end", "뷰어", "viewer", "줄 끝", "end"),
        (
            "viewer-select-home",
            "뷰어",
            "viewer",
            "줄 처음까지 선택",
            "shift-home",
        ),
        (
            "viewer-select-end",
            "뷰어",
            "viewer",
            "줄 끝까지 선택",
            "shift-end",
        ),
        ("viewer-copy", "뷰어", "viewer", "복사", "ctrl-c"),
        (
            "viewer-toggle-line",
            "뷰어",
            "viewer",
            "선택 범위 / 현재 변경 줄 체크·해제",
            "space",
        ),
        (
            "viewer-stage-lines",
            "뷰어",
            "viewer",
            "선택한 줄 Stage",
            "enter",
        ),
    ] {
        let (first, second) = match id {
            "project-tab-previous" => ("ctrl-left", "ctrl-j"),
            "project-tab-next" => ("ctrl-right", "ctrl-l"),
            _ => (key, ""),
        };
        add(id, group, scope, name, key, first, second);
    }
    for (id, name, key, second) in [
        ("confirm", "확인", "enter", ""),
        ("cancel", "취소", "escape", ""),
        ("next-input", "다음입력", "tab", ""),
        ("previous-input", "이전입력", "shift-tab", ""),
    ] {
        add(id, "공통", "dialog", name, key, key, second);
    }
    add(
        "input-toggle",
        "공통",
        "toggle",
        "입력전환",
        "space",
        "space",
        "",
    );
    for scope in ["checkout", "reset"] {
        for digit in 0..=9 {
            if scope == "reset" && !(1..=3).contains(&digit) {
                continue;
            }
            let key = digit.to_string();
            add(
                &format!("{scope}-{digit}"),
                "공통",
                scope,
                &format!(
                    "{}: {}번 선택",
                    scope_name(scope),
                    if digit == 0 { 10 } else { digit }
                ),
                &key,
                &key,
                "",
            );
        }
    }
    result
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Bindings(pub std::collections::BTreeMap<String, [String; 2]>);

pub fn chord(key: &Keystroke) -> String {
    if key.key_char.as_deref() == Some("?")
        && !key.modifiers.control
        && !key.modifiers.alt
        && !key.modifiers.platform
    {
        return "?".into();
    }
    let mut value = String::new();
    for (enabled, name) in [
        (key.modifiers.control, "ctrl-"),
        (key.modifiers.alt, "alt-"),
        (key.modifiers.shift, "shift-"),
        (key.modifiers.platform, "cmd-"),
        (key.modifiers.function, "fn-"),
    ] {
        if enabled {
            value.push_str(name);
        }
    }
    value.push_str(&key.key.to_lowercase());
    value
}

const DIALOG_SCOPES: [&str; 12] = [
    "upstream",
    "dialog",
    "modal",
    "branch",
    "checkout",
    "reset",
    "stash-modal",
    "commit-modal",
    "discard-modal",
    "settings-modal",
    "clone",
    "shortcuts-modal",
];

fn applies(command_scope: &str, scope: &str) -> bool {
    command_scope == "error-history"
        || command_scope == scope
        || (command_scope == "selection" && ["files", "stage-files", "viewer"].contains(&scope))
        || (command_scope == "checkout-action"
            && ["branches", "local-branches", "history"].contains(&scope))
        || (command_scope == "project-tabs"
            && [
                "main",
                "branches",
                "local-branches",
                "history",
                "files",
                "stage-files",
                "viewer",
                "clone",
            ]
            .contains(&scope))
        || (command_scope == "branches" && scope == "local-branches")
        || (command_scope == "files" && scope == "stage-files")
        || (command_scope == "dialog" && DIALOG_SCOPES.contains(&scope))
        || (command_scope == "navigation"
            && [
                "main",
                "branches",
                "local-branches",
                "history",
                "files",
                "stage-files",
                "viewer",
                "reset",
                "upstream",
            ]
            .contains(&scope))
        || (command_scope == "horizontal-navigation"
            && [
                "main",
                "history",
                "viewer",
                "reset",
                "upstream",
                "branches",
                "local-branches",
                "files",
                "stage-files",
                "shortcuts-modal",
            ]
            .contains(&scope))
        || (command_scope == "toggle"
            && [
                "branch",
                "reset",
                "commit-modal",
                "upstream",
                "settings-modal",
            ]
            .contains(&scope))
        || (command_scope == "*"
            && [
                "branches",
                "local-branches",
                "history",
                "files",
                "stage-files",
                "viewer",
                "main",
            ]
            .contains(&scope))
}

fn overlap(a: &str, b: &str) -> bool {
    [
        "main",
        "branches",
        "local-branches",
        "history",
        "files",
        "stage-files",
        "viewer",
        "modal",
        "branch",
        "upstream",
        "checkout",
        "reset",
        "stash-modal",
        "commit-modal",
        "discard-modal",
        "settings-modal",
        "clone",
        "shortcuts-modal",
    ]
    .into_iter()
    .any(|scope| applies(a, scope) && applies(b, scope))
}

fn branch_priority_pair(a: &Command, b: &Command) -> bool {
    ![
        "errors",
        "nav-left",
        "nav-right",
        "project-tab-previous",
        "project-tab-next",
        "branch-checkout",
    ]
    .contains(&a.id.as_str())
        && ![
            "errors",
            "nav-left",
            "nav-right",
            "project-tab-previous",
            "project-tab-next",
            "branch-checkout",
        ]
        .contains(&b.id.as_str())
        && ((a.group == "브랜치" && b.group == "공통")
            || (b.group == "브랜치" && a.group == "공통"))
}

impl Bindings {
    pub fn button_hint(&self, description: &str, id: &str) -> Div {
        hint(description, &self.label(id))
    }

    pub fn slots(&self, command: &Command) -> [String; 2] {
        self.0
            .get(&command.id)
            .or_else(|| {
                let previous = match command.id.as_str() {
                    "files-all" => "viewer-all",
                    "branch-checkout" => "history-checkout",
                    _ => return None,
                };
                self.0.get(previous)
            })
            .cloned()
            .unwrap_or_else(|| command.defaults.clone())
    }

    pub fn set(&mut self, id: &str, slot: usize, value: String) -> Result<(), String> {
        let commands = commands();
        let command = commands
            .iter()
            .find(|c| c.id == id)
            .ok_or("기능을 찾을 수 없습니다.")?;
        if slot > 1 {
            return Err("잘못된 슬롯입니다.".into());
        }
        if !value.is_empty() {
            Keystroke::parse(&value).map_err(|_| "지원하지 않는 키입니다.")?;
            if value == "escape" && id != "cancel" {
                return Err("Esc는 취소/닫기에 사용합니다. 다른 키를 선택하세요.".into());
            }
            for other in &commands {
                if !overlap(command.scope, other.scope) {
                    continue;
                }
                for (index, key) in self.slots(other).iter().enumerate() {
                    if other.id == id && index == slot {
                        continue;
                    }
                    if key == &value && !branch_priority_pair(command, other) {
                        return Err(format!(
                            "{} / {}에 이미 지정된 키입니다.",
                            other.group, other.name
                        ));
                    }
                }
            }
        }
        let mut slots = self.slots(command);
        slots[slot] = value;
        self.0.insert(id.into(), slots);
        Ok(())
    }

    pub fn resolve(&self, scope: &str, input: &Keystroke) -> Option<Keystroke> {
        let value = chord(input);
        commands()
            .iter()
            .filter(|c| applies(c.scope, scope) && self.slots(c).contains(&value))
            .max_by_key(|c| u8::from(c.group == "브랜치"))
            .and_then(|c| {
                let target = if c.id == "confirm"
                    && ["commit-modal", "discard-modal", "settings-modal"].contains(&scope)
                {
                    "ctrl-s"
                } else {
                    &c.target
                };
                Keystroke::parse(target).ok()
            })
    }

    pub fn label(&self, id: &str) -> String {
        commands()
            .iter()
            .find(|c| c.id == id)
            .map(|c| {
                self.slots(c)
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" / ")
            })
            .unwrap_or_default()
            .replace("-", " + ")
    }
}

#[cfg(test)]
mod binding_tests {
    #[::core::prelude::v1::test]
    fn terminal_shortcut_remaps_and_stays_out_of_popups() {
        let mut bindings = super::Bindings::default();
        assert_eq!(bindings.label("terminal"), "f9");
        for scope in [
            "main",
            "branches",
            "local-branches",
            "history",
            "files",
            "stage-files",
            "viewer",
        ] {
            assert_eq!(
                super::chord(
                    &bindings
                        .resolve(scope, &gpui::Keystroke::parse("f9").unwrap())
                        .unwrap()
                ),
                "f9"
            );
        }
        for scope in super::DIALOG_SCOPES {
            assert!(
                bindings
                    .resolve(scope, &gpui::Keystroke::parse("f9").unwrap())
                    .is_none()
            );
        }
        bindings.set("terminal", 0, "alt-t".into()).unwrap();
        assert_eq!(bindings.label("terminal"), "alt + t");
        assert!(
            bindings
                .resolve("main", &gpui::Keystroke::parse("f9").unwrap())
                .is_none()
        );
        assert_eq!(
            super::chord(
                &bindings
                    .resolve("main", &gpui::Keystroke::parse("alt-t").unwrap())
                    .unwrap()
            ),
            "f9"
        );
        assert!(bindings.set("terminal", 0, "f10".into()).is_err());
    }

    #[::core::prelude::v1::test]
    fn explorer_shortcut_remaps_and_does_not_run_in_popups() {
        let mut bindings = super::Bindings::default();
        assert_eq!(bindings.label("explorer"), "f10");
        for scope in [
            "main",
            "branches",
            "local-branches",
            "files",
            "stage-files",
            "viewer",
            "history",
        ] {
            assert_eq!(
                super::chord(
                    &bindings
                        .resolve(scope, &gpui::Keystroke::parse("f10").unwrap())
                        .unwrap()
                ),
                "f10"
            );
        }
        for scope in super::DIALOG_SCOPES {
            assert!(
                bindings
                    .resolve(scope, &gpui::Keystroke::parse("f10").unwrap())
                    .is_none()
            );
        }
        bindings.set("explorer", 0, "alt-e".into()).unwrap();
        assert_eq!(bindings.label("explorer"), "alt + e");
        assert!(
            bindings
                .resolve("main", &gpui::Keystroke::parse("f10").unwrap())
                .is_none()
        );
        assert_eq!(
            super::chord(
                &bindings
                    .resolve("main", &gpui::Keystroke::parse("alt-e").unwrap())
                    .unwrap()
            ),
            "f10"
        );
    }

    #[::core::prelude::v1::test]
    fn reset_shortcuts_is_popup_only_remappable_and_checks_conflicts() {
        let mut bindings = super::Bindings::default();
        let key = gpui::Keystroke::parse("ctrl-d").unwrap();
        assert_eq!(
            super::chord(&bindings.resolve("shortcuts-modal", &key).unwrap()),
            "ctrl-d"
        );
        for scope in [
            "main",
            "branches",
            "files",
            "viewer",
            "history",
            "modal",
            "settings-modal",
        ] {
            assert!(bindings.resolve(scope, &key).is_none());
        }
        assert!(bindings.set("shortcuts-reset", 0, "enter".into()).is_err());
        assert!(bindings.set("shortcuts-reset", 0, "left".into()).is_err());
        bindings.set("shortcuts-reset", 0, "alt-d".into()).unwrap();
        assert!(bindings.resolve("shortcuts-modal", &key).is_none());
        assert_eq!(
            super::chord(
                &bindings
                    .resolve("shortcuts-modal", &gpui::Keystroke::parse("alt-d").unwrap())
                    .unwrap()
            ),
            "ctrl-d"
        );
    }
    use super::*;

    #[::core::prelude::v1::test]
    fn shared_actions_use_one_common_setting_across_their_supported_areas() {
        let mut bindings = Bindings::default();
        let definitions = commands();
        for (id, scopes, old, custom, alternate) in [
            (
                "files-all",
                ["files", "stage-files", "viewer"],
                "ctrl-a",
                "alt-a",
                "ctrl-alt-a",
            ),
            (
                "branch-checkout",
                ["branches", "local-branches", "history"],
                "o",
                "alt-o",
                "ctrl-alt-o",
            ),
        ] {
            let command = definitions.iter().find(|command| command.id == id).unwrap();
            assert_eq!(command.group, "공통");
            bindings.set(id, 0, custom.into()).unwrap();
            bindings.set(id, 1, alternate.into()).unwrap();
            for scope in scopes {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(old).unwrap())
                        .is_none()
                );
                for key in [custom, alternate] {
                    assert_eq!(
                        chord(
                            &bindings
                                .resolve(scope, &Keystroke::parse(key).unwrap())
                                .unwrap()
                        ),
                        old
                    );
                }
            }
            for scope in ["main", "modal", "commit-modal", "clone"] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(custom).unwrap())
                        .is_none()
                );
            }
        }
        assert!(
            !definitions
                .iter()
                .any(|command| ["viewer-all", "history-checkout"].contains(&command.id.as_str()))
        );
    }

    #[::core::prelude::v1::test]
    fn shared_actions_inherit_legacy_slots_and_explicit_canonical_values_take_precedence() {
        let mut bindings = Bindings::default();
        bindings
            .0
            .insert("viewer-all".into(), ["alt-a".into(), "ctrl-alt-a".into()]);
        bindings.0.insert(
            "history-checkout".into(),
            ["alt-o".into(), "ctrl-alt-o".into()],
        );
        for scope in ["files", "stage-files", "viewer"] {
            for key in ["alt-a", "ctrl-alt-a"] {
                assert_eq!(
                    chord(
                        &bindings
                            .resolve(scope, &Keystroke::parse(key).unwrap())
                            .unwrap()
                    ),
                    "ctrl-a"
                );
            }
        }
        for scope in ["branches", "local-branches", "history"] {
            assert_eq!(
                bindings
                    .resolve(scope, &Keystroke::parse("alt-o").unwrap())
                    .unwrap()
                    .key,
                "o"
            );
        }
        bindings
            .0
            .insert("files-all".into(), ["ctrl-a".into(), String::new()]);
        assert!(
            bindings
                .resolve("viewer", &Keystroke::parse("alt-a").unwrap())
                .is_none()
        );
        assert_eq!(bindings.label("files-all"), "ctrl + a");
        bindings
            .0
            .insert("branch-checkout".into(), [String::new(), String::new()]);
        assert!(
            bindings
                .resolve("history", &Keystroke::parse("alt-o").unwrap())
                .is_none()
        );
        assert!(bindings.label("branch-checkout").is_empty());
        bindings.set("branch-checkout", 0, "o".into()).unwrap();
        assert_eq!(
            bindings
                .resolve("history", &Keystroke::parse("o").unwrap())
                .unwrap()
                .key,
            "o"
        );
    }

    #[::core::prelude::v1::test]
    fn common_direction_keys_drive_internal_tabs_and_navigation_with_one_setting() {
        let mut bindings = Bindings::default();
        let scopes = [
            "branches",
            "local-branches",
            "files",
            "stage-files",
            "shortcuts-modal",
            "main",
            "history",
            "viewer",
            "reset",
            "upstream",
        ];
        for scope in scopes {
            for (input, target) in [
                ("left", "left"),
                ("j", "left"),
                ("right", "right"),
                ("l", "right"),
            ] {
                assert_eq!(
                    bindings
                        .resolve(scope, &Keystroke::parse(input).unwrap())
                        .unwrap()
                        .key,
                    target
                );
            }
            for old in ["[", "]"] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(old).unwrap())
                        .is_none()
                );
            }
        }
        assert!(
            !commands()
                .iter()
                .any(|c| ["tab-previous", "tab-next"].contains(&c.id.as_str()))
        );
        bindings
            .0
            .insert("tab-previous".into(), ["alt-q".into(), "alt-w".into()]);
        for scope in scopes {
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("alt-q").unwrap())
                    .is_none()
            );
        }
        bindings.set("nav-left", 0, "alt-u".into()).unwrap();
        bindings.set("nav-left", 1, "ctrl-alt-u".into()).unwrap();
        for scope in scopes {
            for old in ["left", "j"] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(old).unwrap())
                        .is_none()
                );
            }
            for input in ["alt-u", "ctrl-alt-u"] {
                assert_eq!(
                    bindings
                        .resolve(scope, &Keystroke::parse(input).unwrap())
                        .unwrap()
                        .key,
                    "left"
                );
            }
        }
        assert!(bindings.set("nav-right", 0, "alt-u".into()).is_err());
        assert!(bindings.set("branch-upstream", 0, "alt-u".into()).is_err());
        assert!(bindings.set("files-all", 0, "alt-u".into()).is_err());
        assert!(bindings.set("nav-left", 0, "enter".into()).is_err());
        for key in ["enter", "escape"] {
            assert_eq!(
                bindings
                    .resolve("shortcuts-modal", &Keystroke::parse(key).unwrap())
                    .unwrap()
                    .key,
                key
            );
        }
    }

    #[::core::prelude::v1::test]
    fn project_tab_keys_include_start_input_but_exclude_dialogs_and_can_be_remapped() {
        let mut bindings = Bindings::default();
        let scopes = [
            "main",
            "branches",
            "local-branches",
            "history",
            "files",
            "stage-files",
            "viewer",
            "clone",
        ];
        for scope in scopes {
            for (key, target) in [
                ("ctrl-left", "ctrl-["),
                ("ctrl-j", "ctrl-["),
                ("ctrl-right", "ctrl-]"),
                ("ctrl-l", "ctrl-]"),
            ] {
                assert_eq!(
                    chord(
                        &bindings
                            .resolve(scope, &Keystroke::parse(key).unwrap())
                            .unwrap()
                    ),
                    target
                );
            }
        }
        for scope in [
            "modal",
            "shortcuts-modal",
            "branch",
            "commit-modal",
            "settings-modal",
        ] {
            for key in ["ctrl-left", "ctrl-right", "ctrl-j", "ctrl-l"] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(key).unwrap())
                        .is_none()
                );
            }
        }
        bindings
            .set("project-tab-previous", 0, "alt-y".into())
            .unwrap();
        for scope in scopes {
            assert_eq!(
                chord(
                    &bindings
                        .resolve(scope, &Keystroke::parse("ctrl-j").unwrap())
                        .unwrap()
                ),
                "ctrl-["
            );
            for old in ["ctrl-[", "ctrl-]"] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(old).unwrap())
                        .is_none()
                );
            }
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("ctrl-left").unwrap())
                    .is_none()
            );
            assert_eq!(
                chord(
                    &bindings
                        .resolve(scope, &Keystroke::parse("alt-y").unwrap())
                        .unwrap()
                ),
                "ctrl-["
            );
        }
        assert!(bindings.set("nav-left", 0, "alt-y".into()).is_err());
        assert!(bindings.set("branch-checkout", 0, "alt-y".into()).is_err());
    }

    #[::core::prelude::v1::test]
    fn remote_defaults_use_control_chords_and_keep_the_existing_execution_targets() {
        let mut bindings = Bindings::default();
        for scope in [
            "main",
            "local-branches",
            "branches",
            "history",
            "files",
            "stage-files",
            "viewer",
        ] {
            for (key, target) in [("ctrl-p", "p"), ("ctrl-f", "f"), ("ctrl-u", "shift-p")] {
                assert_eq!(
                    chord(
                        &bindings
                            .resolve(scope, &Keystroke::parse(key).unwrap())
                            .unwrap()
                    ),
                    target
                );
            }
            for key in ["p", "f", "shift-p"] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(key).unwrap())
                        .is_none()
                );
            }
        }
        for (id, default, custom, target) in [
            ("pull", "ctrl-p", "ctrl-alt-p", "p"),
            ("fetch", "ctrl-f", "ctrl-alt-f", "f"),
            ("push", "ctrl-u", "ctrl-alt-u", "shift-p"),
        ] {
            bindings.set(id, 0, custom.into()).unwrap();
            assert!(
                bindings
                    .resolve("main", &Keystroke::parse(default).unwrap())
                    .is_none()
            );
            assert_eq!(
                chord(
                    &bindings
                        .resolve("main", &Keystroke::parse(custom).unwrap())
                        .unwrap()
                ),
                target
            );
        }
    }

    #[::core::prelude::v1::test]
    fn error_history_key_is_global_remappable_and_confirmation_defaults_to_enter_only() {
        let mut bindings = Bindings::default();
        assert_eq!(bindings.label("errors"), "f1");
        assert!(
            bindings
                .resolve("error-history", &Keystroke::parse("f10").unwrap())
                .is_none()
        );
        assert_eq!(bindings.label("confirm"), "enter");
        for scope in DIALOG_SCOPES {
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("ctrl-s").unwrap())
                    .is_none()
            );
        }
        for scope in ["main", "viewer", "branch", "commit-modal", "error-history"] {
            assert_eq!(
                bindings
                    .resolve(scope, &Keystroke::parse("f1").unwrap())
                    .unwrap()
                    .key,
                "f1"
            );
        }
        assert!(bindings.set("errors", 0, "enter".into()).is_err());
        assert!(bindings.set("errors", 0, "r".into()).is_err());
        bindings.set("errors", 0, "alt-z".into()).unwrap();
        assert_eq!(bindings.label("errors"), "alt + z");
        for scope in ["main", "viewer", "branch", "commit-modal", "error-history"] {
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("f1").unwrap())
                    .is_none()
            );
            assert_eq!(
                bindings
                    .resolve(scope, &Keystroke::parse("alt-z").unwrap())
                    .unwrap()
                    .key,
                "f1"
            );
        }
        assert!(bindings.set("branch-checkout", 0, "alt-z".into()).is_err());
        assert!(bindings.set("confirm", 1, "alt-z".into()).is_err());
    }
    #[::core::prelude::v1::test]
    fn tree_toggle_is_scoped_to_both_file_tabs_and_can_be_remapped() {
        let mut bindings = Bindings::default();
        for scope in ["files", "stage-files"] {
            assert_eq!(
                chord(
                    &bindings
                        .resolve(scope, &Keystroke::parse("`").unwrap())
                        .unwrap()
                ),
                "`"
            );
        }
        for scope in [
            "main",
            "branches",
            "history",
            "viewer",
            "commit-modal",
            "settings-modal",
            "clone",
        ] {
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("`").unwrap())
                    .is_none()
            );
        }
        bindings.set("files-tree", 0, "alt-y".into()).unwrap();
        for scope in ["files", "stage-files"] {
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("`").unwrap())
                    .is_none()
            );
            assert_eq!(
                chord(
                    &bindings
                        .resolve(scope, &Keystroke::parse("alt-y").unwrap())
                        .unwrap()
                ),
                "`"
            );
        }
        assert!(bindings.set("files-all", 1, "alt-y".into()).is_err());
    }
    #[::core::prelude::v1::test]
    fn remaps_both_slots_and_disables_old_keys() {
        let mut bindings = Bindings::default();
        bindings.set("refresh", 0, "alt-z".into()).unwrap();
        bindings.set("refresh", 1, "ctrl-y".into()).unwrap();
        assert_eq!(
            bindings
                .resolve("history", &Keystroke::parse("alt-z").unwrap())
                .unwrap()
                .key,
            "f5"
        );
        assert_eq!(
            bindings
                .resolve("files", &Keystroke::parse("ctrl-y").unwrap())
                .unwrap()
                .key,
            "f5"
        );
        assert!(
            bindings
                .resolve("history", &Keystroke::parse("f5").unwrap())
                .is_none()
        );
        assert!(bindings.set("files-all", 1, "alt-z".into()).is_err());
        bindings.set("viewer-copy", 1, "o".into()).unwrap();
        assert!(bindings.set("history-tag", 1, "o".into()).is_err());
        bindings.set("refresh", 0, String::new()).unwrap();
        assert!(
            bindings
                .resolve("main", &Keystroke::parse("alt-z").unwrap())
                .is_none()
        );
        let restored: Bindings =
            serde_json::from_str(&serde_json::to_string(&bindings).unwrap()).unwrap();
        assert_eq!(restored.0, bindings.0);
    }
    #[::core::prelude::v1::test]
    fn dialog_controls_share_bindings_and_contextual_confirmation() {
        let mut bindings = Bindings::default();
        let commands = commands();
        for (id, name) in [
            ("confirm", "확인"),
            ("cancel", "취소"),
            ("next-input", "다음입력"),
            ("previous-input", "이전입력"),
        ] {
            assert_eq!(
                commands
                    .iter()
                    .filter(|c| c.id == id && c.name == name)
                    .count(),
                1
            );
        }
        assert!(!commands.iter().any(|c| c.id.ends_with("-submit")
            || c.id.ends_with("-close")
            || c.id.ends_with("-tab")
            || c.id.ends_with("-backtab")));
        for (id, key) in [
            ("confirm", "alt-y"),
            ("cancel", "alt-n"),
            ("next-input", "alt-j"),
            ("previous-input", "alt-k"),
        ] {
            bindings.set(id, 0, key.into()).unwrap();
        }
        for scope in DIALOG_SCOPES {
            let confirm = bindings
                .resolve(scope, &Keystroke::parse("alt-y").unwrap())
                .unwrap();
            assert_eq!(
                chord(&confirm),
                if ["commit-modal", "discard-modal", "settings-modal"].contains(&scope) {
                    "ctrl-s"
                } else {
                    "enter"
                }
            );
            for (key, target) in [
                ("alt-n", "escape"),
                ("alt-j", "tab"),
                ("alt-k", "shift-tab"),
            ] {
                assert_eq!(
                    chord(
                        &bindings
                            .resolve(scope, &Keystroke::parse(key).unwrap())
                            .unwrap()
                    ),
                    target
                );
            }
            for old in ["enter", "escape", "tab", "shift-tab"] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(old).unwrap())
                        .is_none()
                );
            }
        }
        assert!(bindings.set("input-toggle", 1, "alt-y".into()).is_err());
        assert!(
            bindings
                .resolve("history", &Keystroke::parse("alt-y").unwrap())
                .is_none()
        );
        bindings.set("cancel", 0, "escape".into()).unwrap();
    }

    #[::core::prelude::v1::test]
    fn common_navigation_and_toggle_share_keys_without_changing_other_sections() {
        let mut bindings = Bindings::default();
        let definitions = commands();
        for (id, name) in [
            ("nav-up", "방향키 위"),
            ("nav-down", "방향키 아래"),
            ("nav-left", "방향키 왼쪽"),
            ("nav-right", "방향키 오른쪽"),
            ("input-toggle", "입력전환"),
        ] {
            assert_eq!(
                definitions
                    .iter()
                    .filter(|c| c.id == id && c.group == "공통" && c.name == name)
                    .count(),
                1
            );
        }
        assert!(!definitions.iter().any(|c| c.id == "commit-push"
            || c.id == "branch-toggle"
            || c.id == "reset-toggle"
            || ["reset-up", "reset-down", "reset-left", "reset-right"].contains(&c.id.as_str())));
        bindings.set("nav-up", 0, "alt-u".into()).unwrap();
        bindings.set("input-toggle", 0, "alt-t".into()).unwrap();
        for scope in ["main", "branches", "history", "files", "viewer", "reset"] {
            assert_eq!(
                bindings
                    .resolve(scope, &Keystroke::parse("alt-u").unwrap())
                    .unwrap()
                    .key,
                "up"
            );
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("up").unwrap())
                    .is_none()
            );
        }
        for scope in ["branch", "reset", "commit-modal"] {
            assert_eq!(
                bindings
                    .resolve(scope, &Keystroke::parse("alt-t").unwrap())
                    .unwrap()
                    .key,
                "space"
            );
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("space").unwrap())
                    .is_none()
            );
        }
        for scope in ["branches", "files"] {
            assert_eq!(
                bindings
                    .resolve(scope, &Keystroke::parse("space").unwrap())
                    .unwrap()
                    .key,
                "space"
            );
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("alt-t").unwrap())
                    .is_none()
            );
        }
        assert!(bindings.set("refresh", 0, "alt-u".into()).is_err());
        assert!(bindings.set("confirm", 0, "alt-t".into()).is_err());
        assert!(bindings.set("input-toggle", 1, "alt-u".into()).is_err());
    }

    #[::core::prelude::v1::test]
    fn commit_and_stash_require_stage_files_and_preserve_existing_bindings() {
        let mut bindings = Bindings::default();
        for (id, key, custom) in [("stash", "s", "alt-s"), ("commit", "c", "alt-c")] {
            let command = commands().into_iter().find(|c| c.id == id).unwrap();
            assert_eq!(command.group, "파일목록");
            assert_eq!(
                bindings
                    .resolve("stage-files", &Keystroke::parse(key).unwrap())
                    .unwrap()
                    .key,
                key
            );
            for scope in [
                "files",
                "main",
                "history",
                "branches",
                "viewer",
                "branch",
                "commit-modal",
            ] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(key).unwrap())
                        .is_none()
                );
            }
            bindings.set(id, 0, custom.into()).unwrap();
            assert_eq!(
                bindings
                    .resolve("stage-files", &Keystroke::parse(custom).unwrap())
                    .unwrap()
                    .key,
                key
            );
            for scope in ["files", "main", "history", "branches", "viewer"] {
                assert!(
                    bindings
                        .resolve(scope, &Keystroke::parse(custom).unwrap())
                        .is_none()
                );
            }
            assert!(bindings.set("files-all", 1, custom.into()).is_err());
            assert!(bindings.set("refresh", 1, custom.into()).is_err());
        }
        for key in ["space", "ctrl-a", "d", "f5", "up", "shift-up"] {
            assert!(
                bindings
                    .resolve("stage-files", &Keystroke::parse(key).unwrap())
                    .is_some()
            );
        }
    }

    #[::core::prelude::v1::test]
    fn subsections_share_commands_and_internal_number_keys_remain_executable() {
        let commands = commands();
        assert_eq!(subsections("브랜치"), ["공통", "Local", "Remote"]);
        assert_eq!(subsections("파일목록"), ["공통", "Stage"]);
        for command in commands.iter().filter(|c| list_visible(c)) {
            assert_eq!(
                subsections(command.group)
                    .iter()
                    .filter(|subsection| in_subsection(command, subsection))
                    .count(),
                1,
                "{}",
                command.id
            );
        }
        let jump = commands.iter().find(|c| c.id == "branch-jump").unwrap();
        assert!(in_subsection(jump, "공통"));
        assert!(!in_subsection(jump, "Local"));
        assert!(!in_subsection(jump, "Remote"));
        assert_eq!(commands.iter().filter(|c| c.id == jump.id).count(), 1);
        let upload = commands.iter().find(|c| c.id == "branch-upstream").unwrap();
        assert!(in_subsection(upload, "Local"));
        assert!(!in_subsection(upload, "Remote"));
        assert_eq!(
            commands
                .iter()
                .filter(|c| c.group == "파일목록" && in_subsection(c, "공통"))
                .map(|c| c.id.as_str())
                .collect::<Vec<_>>(),
            vec!["files-tree"]
        );
        let mut bindings = Bindings::default();
        for command in commands
            .iter()
            .filter(|c| ["checkout", "reset"].contains(&c.scope))
        {
            assert!(!list_visible(command));
            assert_eq!(
                bindings
                    .resolve(
                        command.scope,
                        &Keystroke::parse(&command.defaults[0]).unwrap()
                    )
                    .unwrap()
                    .key,
                command.target
            );
        }
        bindings.set("checkout-1", 0, "alt-1".into()).unwrap();
        assert_eq!(
            bindings
                .resolve("checkout", &Keystroke::parse("alt-1").unwrap())
                .unwrap()
                .key,
            "1"
        );
    }

    #[::core::prelude::v1::test]
    fn local_upstream_replaces_push_and_branch_shortcuts_override_common_keys() {
        let mut bindings = Bindings::default();
        let definition = commands()
            .into_iter()
            .find(|c| c.id == "branch-upstream")
            .unwrap();
        assert_eq!(definition.name, "원격 브랜치 연결");
        assert_eq!(definition.defaults, ["r", ""]);
        assert!(!commands().iter().any(|c| c.id == "branch-upload"));
        bindings
            .0
            .insert("branch-upload".into(), ["u".into(), "alt-z".into()]);
        assert!(
            bindings
                .resolve("local-branches", &Keystroke::parse("u").unwrap())
                .is_none()
        );
        assert!(
            bindings
                .resolve("local-branches", &Keystroke::parse("alt-z").unwrap())
                .is_none()
        );
        for scope in ["branches", "files", "viewer", "main"] {
            assert!(
                bindings
                    .resolve(scope, &Keystroke::parse("r").unwrap())
                    .is_none()
            );
        }
        assert_eq!(
            bindings
                .resolve("history", &Keystroke::parse("r").unwrap())
                .unwrap()
                .key,
            "r"
        );
        assert_eq!(
            chord(
                &bindings
                    .resolve("local-branches", &Keystroke::parse("r").unwrap())
                    .unwrap()
            ),
            "r"
        );
        for scope in [
            "local-branches",
            "branches",
            "history",
            "files",
            "stage-files",
            "viewer",
            "main",
        ] {
            assert_eq!(
                chord(
                    &bindings
                        .resolve(scope, &Keystroke::parse("ctrl-u").unwrap())
                        .unwrap()
                ),
                "shift-p"
            );
        }
        bindings.set("branch-upstream", 0, "f5".into()).unwrap();
        assert_eq!(
            chord(
                &bindings
                    .resolve("local-branches", &Keystroke::parse("f5").unwrap())
                    .unwrap()
            ),
            "r"
        );
        assert_eq!(
            chord(
                &bindings
                    .resolve("history", &Keystroke::parse("f5").unwrap())
                    .unwrap()
            ),
            "f5"
        );
        bindings.set("refresh", 1, "f5".into()).unwrap_err();
        bindings.set("branch-delete", 1, "f5".into()).unwrap_err();
        bindings.set("branch-upstream", 0, "alt-u".into()).unwrap();
        bindings.set("refresh", 1, "alt-u".into()).unwrap();
        assert_eq!(
            bindings
                .resolve("local-branches", &Keystroke::parse("alt-u").unwrap())
                .unwrap()
                .key,
            "r"
        );
        assert_eq!(
            bindings
                .resolve("history", &Keystroke::parse("alt-u").unwrap())
                .unwrap()
                .key,
            "f5"
        );
    }

    #[::core::prelude::v1::test]
    fn defaults_have_no_conflicts_and_every_slot_resolves() {
        let bindings = Bindings::default();
        let commands = commands();
        for (index, command) in commands.iter().enumerate() {
            for key in command.defaults.iter().filter(|s| !s.is_empty()) {
                let scope = if command.scope == "*" {
                    "main"
                } else {
                    command.scope
                };
                assert_eq!(
                    bindings
                        .resolve(scope, &Keystroke::parse(key).unwrap())
                        .unwrap()
                        .key,
                    Keystroke::parse(&command.target).unwrap().key,
                    "{}",
                    command.id
                );
                for other in commands
                    .iter()
                    .skip(index + 1)
                    .filter(|c| overlap(command.scope, c.scope))
                {
                    assert!(
                        !other.defaults.contains(key) || branch_priority_pair(command, other),
                        "{} / {}",
                        command.id,
                        other.id
                    );
                }
            }
        }
    }

    #[::core::prelude::v1::test]
    fn viewer_line_shortcuts_remap_both_slots_without_changing_other_scopes() {
        let mut bindings = Bindings::default();
        for (id, default, custom, alternate) in [
            ("viewer-toggle-line", "space", "alt-y", "ctrl-alt-y"),
            ("viewer-stage-lines", "enter", "alt-z", "ctrl-alt-z"),
        ] {
            let command = commands()
                .into_iter()
                .find(|command| command.id == id)
                .unwrap();
            assert_eq!(command.group, "뷰어");
            assert_eq!(
                bindings
                    .resolve("viewer", &Keystroke::parse(default).unwrap())
                    .unwrap()
                    .key,
                default
            );
            bindings.set(id, 0, custom.into()).unwrap();
            bindings.set(id, 1, alternate.into()).unwrap();
            assert!(
                bindings
                    .resolve("viewer", &Keystroke::parse(default).unwrap())
                    .is_none()
            );
            for key in [custom, alternate] {
                assert_eq!(
                    bindings
                        .resolve("viewer", &Keystroke::parse(key).unwrap())
                        .unwrap()
                        .key,
                    default
                );
                for scope in [
                    "main",
                    "files",
                    "stage-files",
                    "history",
                    "branches",
                    "commit-modal",
                    "branch",
                ] {
                    assert!(
                        bindings
                            .resolve(scope, &Keystroke::parse(key).unwrap())
                            .is_none()
                    );
                }
            }
        }
        assert_eq!(
            bindings
                .resolve("stage-files", &Keystroke::parse("space").unwrap())
                .unwrap()
                .key,
            "space"
        );
        assert_eq!(
            bindings
                .resolve("branch", &Keystroke::parse("enter").unwrap())
                .unwrap()
                .key,
            "enter"
        );
        assert_eq!(
            bindings
                .resolve("commit-modal", &Keystroke::parse("enter").unwrap())
                .unwrap()
                .key,
            "s"
        );
    }
}
