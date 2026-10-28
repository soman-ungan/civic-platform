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
//! 発言ではない行（「△開議の宣告」のような節の見出し、「（市長　〇〇登壇）」のようなト書き）は
//! 飛ばす。
//!
//! 形式は自治体ごとに違うので、正規表現は設定（`collector_sources.config` の `speaker_pattern`
//! と `skip_pattern`）で差し替えられる。`speaker_pattern` は名前付きグループ `role` と `name` を持つこと。

use std::sync::LazyLock;

use regex::Regex;

/// 既定の見出しの形: 行頭の記号、役職、括弧の中の氏名
pub const DEFAULT_SPEAKER_PATTERN: &str =
    r"^[○◆◎▲□◇]\s*(?P<role>[^（(\s]+?)\s*[（(](?P<name>[^）)]+?)[）)]\s*";
/// 既定の、発言ではない行: △の節の見出し、括弧だけのト書き
pub const DEFAULT_SKIP_PATTERN: &str = r"^(△|[（(][^）)]*[）)]\s*$)";

/// 既定の、議事日程の行（最初の発言より前にある「日程第１　一般質問」など）
pub const DEFAULT_AGENDA_PATTERN: &str = r"^(追加)?日程第";
/// 既定の、トピックを作る議事。これを含む日程が無い会議録（会期の決定・議案の上程・採決だけの日など
/// 形式的な議事）は取り込まない
pub const DEFAULT_TOPICAL_AGENDA_PATTERN: &str = "一般質問|代表質問";

/// 会議録の読み取りの形
pub struct Patterns {
    pub speaker: Regex,
    pub skip: Regex,
    pub agenda: Regex,
    pub topical_agenda: Regex,
}

impl Default for Patterns {
    fn default() -> Self {
        Self {
            speaker: Regex::new(DEFAULT_SPEAKER_PATTERN).unwrap(),
            skip: Regex::new(DEFAULT_SKIP_PATTERN).unwrap(),
            agenda: Regex::new(DEFAULT_AGENDA_PATTERN).unwrap(),
            topical_agenda: Regex::new(DEFAULT_TOPICAL_AGENDA_PATTERN).unwrap(),
        }
    }
}

/// 議事日程（最初の発言より前にある日程の行）
pub fn agenda(text: &str, patterns: &Patterns) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start_matches([' ', '\u{3000}', '\t']);
        if patterns.speaker.is_match(trimmed) {
            break;
        }
        if patterns.agenda.is_match(trimmed) {
            out.push(trimmed.trim_end_matches([' ', '\u{3000}']).to_owned());
        }
    }
    out
}

/// 議長の短い進行の発言（「〇〇市長。」「休憩します。」など）。AI に送らない
pub fn is_procedural(u: &Utterance) -> bool {
    u.kind == SpeakerKind::Chair && u.body.chars().count() <= 40
}

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
pub fn parse(text: &str, patterns: &Patterns) -> Vec<Utterance> {
    let mut out: Vec<Utterance> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start_matches([' ', '\u{3000}', '\t']);
        if patterns.skip.is_match(trimmed) {
            continue;
        }
        if let Some(caps) = patterns.speaker.captures(trimmed) {
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

△開議の宣告
○議長（高橋　一郎君）　ただいまから本日の会議を開きます。
△鈴木花子議員
◆12番（鈴木　花子君）　通告に従い質問します。
放課後児童クラブの待機児童について伺います。
○議長（高橋　一郎君）　山田一郎市長。
　　　　　　　（市長　山田一郎登壇）
◎市長（山田一郎）　お答えいたします。
◎保健福祉部長（佐藤次郎君）　ご質問にお答えします。
";

    #[test]
    fn splits_and_classifies() {
        let u = parse(SAMPLE, &Patterns::default());
        assert_eq!(u.len(), 5);
        assert_eq!(
            (u[0].kind, u[0].name.as_str(), u[0].role.as_str()),
            (SpeakerKind::Chair, "高橋一郎", "議長")
        );
        assert_eq!(u[1].kind, SpeakerKind::Councilor);
        assert_eq!(u[1].name, "鈴木花子");
        // 節の見出し（△）は前の発言に入らない
        assert_eq!(u[0].body, "ただいまから本日の会議を開きます。");
        assert_eq!(
            u[1].body,
            "通告に従い質問します。\n放課後児童クラブの待機児童について伺います。"
        );
        // ト書きは本文に入らない
        assert_eq!(u[2].body, "山田一郎市長。");
        assert_eq!(u[3].kind, SpeakerKind::Mayor);
        assert_eq!(u[3].name, "山田一郎");
        assert_eq!(u[4].kind, SpeakerKind::Staff);
        assert_eq!(u[4].no, 5);
    }

    #[test]
    fn labels_hide_staff_names() {
        let u = parse(SAMPLE, &Patterns::default());
        let labels: Vec<String> = u.iter().map(Utterance::speaker_label).collect();
        assert_eq!(
            labels,
            [
                "高橋一郎議長",
                "鈴木花子議員",
                "高橋一郎議長",
                "山田一郎市長",
                "保健福祉部長"
            ]
        );
    }

    #[test]
    fn committee_chair_is_councilor() {
        assert_eq!(classify("民生文教常任委員長"), SpeakerKind::Chair);
        assert_eq!(classify("予算審査特別委員長"), SpeakerKind::Chair);
        assert_eq!(classify("教育長"), SpeakerKind::Staff);
        assert_eq!(classify("副市長"), SpeakerKind::Staff);
        assert_eq!(classify("１番"), SpeakerKind::Councilor);
    }

    #[test]
    fn agenda_lines_before_first_speech() {
        let text = "議事日程\n日程第　１　代表質問\n日程第　２　一般質問\n○議長（高橋一郎）　開会します。\n日程第　３　ではない\n";
        let p = Patterns::default();
        assert_eq!(
            agenda(text, &p),
            ["日程第　１　代表質問", "日程第　２　一般質問"]
        );
    }

    #[test]
    fn normalize_removes_spaces_and_honorifics() {
        assert_eq!(normalize_name("鈴木　花子 君"), "鈴木花子");
        assert_eq!(normalize_name("山田一郎議員"), "山田一郎");
    }
}
