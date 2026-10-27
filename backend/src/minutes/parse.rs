// SPDX-License-Identifier: AGPL-3.0-only

//! 会議録のテキストを発言に分ける（決まった手順のコード。LLM は使わない）。
//!
//! 会議録検索システムの多くは、発言の始まりを「記号 + 役職（氏名 + 敬称）」で示す。
//!
//! ```text
//! ○議長（高橋一郎君） ただいまから本日の会議を開きます。
//! ◆12番（鈴木花子君） 通告に従い質問します。
//! ◎市長（山田一郎君） お答えいたします。
//! ◎保健福祉部長（佐藤次郎君） ご質問にお答えします。
//! ```
//!
//! 形式は自治体ごとに違うので、見出しの正規表現は設定（`collector_sources.config` の
//! `speaker_pattern`）で差し替えられる。名前付きグループ `role` と `name` を持つこと。

use std::sync::LazyLock;

use regex::Regex;

/// 既定の見出しの形: 行頭の記号、役職、括弧の中の氏名
pub const DEFAULT_SPEAKER_PATTERN: &str =
    r"^[○◆◎△▲□◇]\s*(?P<role>[^（(\s]+?)\s*[（(](?P<name>[^）)]+?)[）)]\s*";

/// 発言者の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakerKind {
    /// 議長・副議長・委員長（議員が務める）
    Chair,
    /// 議員（「12番」など）
    Councilor,
    /// 首長（市長・町長・村長・区長）
    Mayor,
    /// 職員（副市長・部長など。選挙で選ばれていないので、氏名は出さない）
    Staff,
}

impl SpeakerKind {
    /// 公人の名簿と対応付ける種類か
    pub fn is_elected(self) -> bool {
        !matches!(self, Self::Staff)
    }

    /// `public_officials.role` の値
    pub fn official_role(self) -> Option<&'static str> {
        match self {
            Self::Chair | Self::Councilor => Some("councilor"),
            Self::Mayor => Some("mayor"),
            Self::Staff => None,
        }
    }
}

/// 発言1件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utterance {
    /// 会議録の中で何番目の発言か（1から）
    pub no: usize,
    pub kind: SpeakerKind,
    /// 役職（「議長」「12番」「市長」「保健福祉部長」）
    pub role: String,
    /// 氏名（空白と敬称を除いたもの）
    pub name: String,
    pub body: String,
}

impl Utterance {
    /// 画面や AI に示す発言者。公人は氏名つき、職員は職名だけ
    pub fn speaker_label(&self) -> String {
        match self.kind {
            SpeakerKind::Councilor => format!("{}議員", self.name),
            SpeakerKind::Chair | SpeakerKind::Mayor => format!("{}{}", self.name, self.role),
            SpeakerKind::Staff => self.role.clone(),
        }
    }
}

/// 氏名の比較用の形: 空白（全角を含む）と敬称を除く
pub fn normalize_name(raw: &str) -> String {
    let mut name: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    for suffix in ["君", "議員", "さん", "氏", "様"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            name = stripped.to_owned();
            break;
        }
    }
    name
}

static CHAIR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(副?議長|.*委員長)$").unwrap());
static COUNCILOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9０-９]+番|.*議員)$").unwrap());
static MAYOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(市長|町長|村長|区長)$").unwrap());

fn classify(role: &str) -> SpeakerKind {
    if CHAIR.is_match(role) {
        SpeakerKind::Chair
    } else if COUNCILOR.is_match(role) {
        SpeakerKind::Councilor
    } else if MAYOR.is_match(role) {
        SpeakerKind::Mayor
    } else {
        SpeakerKind::Staff
    }
}

/// テキストを発言に分ける。最初の見出しより前（表紙・出席者の一覧など）は使わない
pub fn parse(text: &str, pattern: &Regex) -> Vec<Utterance> {
    let mut out: Vec<Utterance> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start_matches([' ', '\u{3000}', '\t']);
        if let Some(caps) = pattern.captures(trimmed) {
            let role = caps["role"].trim().to_owned();
            let name = normalize_name(&caps["name"]);
            let rest = trimmed[caps.get(0).unwrap().end()..].trim().to_owned();
            out.push(Utterance {
                no: out.len() + 1,
                kind: classify(&role),
                role,
                name,
                body: rest,
            });
        } else if let Some(last) = out.last_mut() {
            let line = line.trim();
            if !line.is_empty() {
                if !last.body.is_empty() {
                    last.body.push('\n');
                }
                last.body.push_str(line);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
令和7年第3回大田原市議会定例会会議録（架空）
出席議員（20名）
   1番  鈴木花子      2番  田中次郎

○議長（高橋　一郎君）　ただいまから本日の会議を開きます。
◆12番（鈴木　花子君）　通告に従い質問します。
放課後児童クラブの待機児童について伺います。
◎市長（山田一郎君）　お答えいたします。
◎保健福祉部長（佐藤次郎君）　ご質問にお答えします。
";

    #[test]
    fn splits_and_classifies() {
        let pattern = Regex::new(DEFAULT_SPEAKER_PATTERN).unwrap();
        let u = parse(SAMPLE, &pattern);
        assert_eq!(u.len(), 4);
        assert_eq!(
            (u[0].kind, u[0].name.as_str(), u[0].role.as_str()),
            (SpeakerKind::Chair, "高橋一郎", "議長")
        );
        assert_eq!(u[1].kind, SpeakerKind::Councilor);
        assert_eq!(u[1].name, "鈴木花子");
        assert_eq!(
            u[1].body,
            "通告に従い質問します。\n放課後児童クラブの待機児童について伺います。"
        );
        assert_eq!(u[2].kind, SpeakerKind::Mayor);
        assert_eq!(u[3].kind, SpeakerKind::Staff);
        assert_eq!(u[3].no, 4);
    }

    #[test]
    fn labels_hide_staff_names() {
        let pattern = Regex::new(DEFAULT_SPEAKER_PATTERN).unwrap();
        let u = parse(SAMPLE, &pattern);
        let labels: Vec<String> = u.iter().map(Utterance::speaker_label).collect();
        assert_eq!(
            labels,
            [
                "高橋一郎議長",
                "鈴木花子議員",
                "山田一郎市長",
                "保健福祉部長"
            ]
        );
    }

    #[test]
    fn normalize_removes_spaces_and_honorifics() {
        assert_eq!(normalize_name("鈴木　花子 君"), "鈴木花子");
        assert_eq!(normalize_name("山田一郎議員"), "山田一郎");
    }
}
