//! Spaced repetition: flashcards and an FSRS-style review schedule built into
//! the app. Every card carries a stability and a difficulty; after each
//! rating the plugin recomputes when the card is due and writes the deck back
//! to the vault. No external service is involved: the schedule is computed
//! locally and stored as JSON next to the vault.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::native::{NativeCommand, Plugin, PluginContext};

/// Absolute ceiling on one interval, in days.
const MAX_INTERVAL: i64 = 36_500;
/// Vault-relative file that holds the deck.
const STORAGE: &str = ".spaced-repetition/cards.json";
/// Difficulty bounds.
const MIN_DIFFICULTY: f64 = 1.0;
const MAX_DIFFICULTY: f64 = 10.0;

/// Interval growth for a card with stability 1.0, per rating, in days.
const GROWTH: [f64; 4] = [0.0, 0.6, 1.2, 2.4];
/// Stability multiplier per rating.
const STABILITY: [f64; 4] = [0.5, 1.2, 1.5, 2.4];
/// Difficulty delta per rating.
const DIFFICULTY_DELTA: [f64; 4] = [1.0, 0.2, -0.15, -0.3];

/// Rating the user gives a reviewed card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rating {
    Again,
    Hard,
    Good,
    Easy,
}

impl Rating {
    fn ix(self) -> usize {
        match self {
            Rating::Again => 0,
            Rating::Hard => 1,
            Rating::Good => 2,
            Rating::Easy => 3,
        }
    }
}

/// A flashcard and the schedule the plugin maintains for it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Card {
    pub id: String,
    pub front: String,
    pub back: String,
    /// Days-since-epoch the card is next due for review.
    pub due: i64,
    /// Days from the last review to the next one.
    pub interval: i64,
    /// FSRS-style stability: larger decays slower.
    pub stability: f64,
    /// FSRS-style difficulty: larger is harder to retain.
    pub difficulty: f64,
    pub reps: u32,
    pub lapses: u32,
    pub last_review: Option<i64>,
    pub created_at: i64,
}

impl Card {
    /// A brand-new card, due today.
    fn new(id: String, front: String, back: String, now: i64) -> Self {
        Card {
            id,
            front,
            back,
            due: now,
            interval: 0,
            stability: 1.0,
            difficulty: 5.0,
            reps: 0,
            lapses: 0,
            last_review: None,
            created_at: now,
        }
    }
}

/// The in-vault deck: every card the plugin knows about.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    fn load(ctx: &PluginContext) -> Result<Self, String> {
        match (ctx.read_file)(STORAGE) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| format!("cannot read deck: {e}")),
            Err(_) => Ok(Deck::default()),
        }
    }

    fn save(&self, ctx: &PluginContext) -> Result<(), String> {
        let text =
            serde_json::to_string_pretty(self).map_err(|e| format!("cannot encode deck: {e}"))?;
        (ctx.write_file)(STORAGE, &text)
    }
}

/// Today as days since 1970-01-01 UTC (the clock the schedule runs on).
fn today() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_secs() as i64
        / 86_400
}

/// Stability after a review, from the previous stability and rating.
fn next_stability(stability: f64, rating: Rating) -> f64 {
    (stability * STABILITY[rating.ix()]).max(0.1)
}

/// Difficulty after a review, clamped to bounds.
fn next_difficulty(difficulty: f64, rating: Rating) -> f64 {
    (difficulty + DIFFICULTY_DELTA[rating.ix()]).clamp(MIN_DIFFICULTY, MAX_DIFFICULTY)
}

/// Days until the next review after a rating.
fn next_interval(card: &Card, rating: Rating) -> i64 {
    let ix = rating.ix();
    // New cards use a fixed first interval; later cards grow from stability.
    if card.reps == 0 {
        return match rating {
            Rating::Again => 1,
            Rating::Hard => 1,
            Rating::Good => 1,
            Rating::Easy => 4,
        };
    }
    let base = (card.stability * GROWTH[ix]).max(0.0);
    (base as i64).clamp(1, MAX_INTERVAL)
}

/// Apply a rating to a card, due on `now + interval`.
fn review(card: Card, rating: Rating, now: i64) -> Card {
    let interval = next_interval(&card, rating);
    Card {
        id: card.id,
        front: card.front,
        back: card.back,
        due: now + interval,
        interval,
        stability: next_stability(card.stability, rating),
        difficulty: next_difficulty(card.difficulty, rating),
        reps: card.reps + 1,
        lapses: card.lapses + u32::from(matches!(rating, Rating::Again)),
        last_review: Some(now),
        created_at: card.created_at,
    }
}

fn parse_rating(value: &Value) -> Result<Rating, String> {
    let text = value
        .get("rating")
        .and_then(Value::as_str)
        .ok_or_else(|| "rating required".to_string())?;
    match text.to_ascii_lowercase().as_str() {
        "again" => Ok(Rating::Again),
        "hard" => Ok(Rating::Hard),
        "good" => Ok(Rating::Good),
        "easy" => Ok(Rating::Easy),
        other => Err(format!("unknown rating: {other}")),
    }
}

pub struct SpacedRepetition;

impl Plugin for SpacedRepetition {
    fn id(&self) -> &'static str {
        "silica:spaced-repetition"
    }

    fn name(&self) -> &'static str {
        "Spaced repetition"
    }

    fn commands(&self) -> Vec<NativeCommand> {
        vec![
            NativeCommand {
                id: "spaced-repetition:add".into(),
                name: "Add flashcard".into(),
            },
            NativeCommand {
                id: "spaced-repetition:review".into(),
                name: "Review flashcard".into(),
            },
            NativeCommand {
                id: "spaced-repetition:due".into(),
                name: "Due flashcards".into(),
            },
            NativeCommand {
                id: "spaced-repetition:next".into(),
                name: "Next due flashcard".into(),
            },
        ]
    }

    fn run_command(
        &self,
        id: &str,
        value: Option<&Value>,
        ctx: &PluginContext<'_>,
    ) -> Result<(), String> {
        match id {
            "spaced-repetition:add" => self.cmd_add(value, ctx),
            "spaced-repetition:review" => self.cmd_review(value, ctx),
            "spaced-repetition:due" => self.cmd_due(ctx),
            "spaced-repetition:next" => self.cmd_next(ctx),
            _ => Err(format!("'{id}' is not a command this plugin registered")),
        }
    }
}

impl SpacedRepetition {
    fn cmd_add(&self, value: Option<&Value>, ctx: &PluginContext<'_>) -> Result<(), String> {
        let args =
            value.ok_or_else(|| "spaced-repetition:add expects a JSON object".to_string())?;
        let front = args
            .get("front")
            .and_then(Value::as_str)
            .ok_or_else(|| "front required".to_string())?;
        let back = args
            .get("back")
            .and_then(Value::as_str)
            .ok_or_else(|| "back required".to_string())?;
        let now = today();
        let mut deck = Deck::load(ctx)?;
        let id = format!("{}-{}", now, deck.cards.len());
        let card = Card::new(id, front.to_string(), back.to_string(), now);
        deck.cards.push(card);
        deck.save(ctx)?;
        (ctx.notify)(&format!("Added flashcard: {front}"));
        Ok(())
    }

    fn cmd_review(&self, value: Option<&Value>, ctx: &PluginContext<'_>) -> Result<(), String> {
        let args =
            value.ok_or_else(|| "spaced-repetition:review expects a JSON object".to_string())?;
        let card_id = args
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "id required".to_string())?;
        let rating = parse_rating(args)?;
        let now = today();
        let mut deck = Deck::load(ctx)?;
        let position = deck
            .cards
            .iter()
            .position(|c| c.id == card_id)
            .ok_or_else(|| format!("no such card: {card_id}"))?;
        let front = deck.cards[position].front.clone();
        let updated = review(std::mem::take(&mut deck.cards[position]), rating, now);
        deck.cards[position] = updated;
        deck.save(ctx)?;
        (ctx.notify)(&format!(
            "Reviewed {front}: next due in {} day(s)",
            deck.cards[position].interval
        ));
        Ok(())
    }

    fn cmd_due(&self, ctx: &PluginContext<'_>) -> Result<(), String> {
        let deck = Deck::load(ctx)?;
        let now = today();
        let due: Vec<&Card> = deck.cards.iter().filter(|c| c.due <= now).collect();
        if due.is_empty() {
            (ctx.notify)("No due flashcards.");
            return Ok(());
        }
        let mut text = format!("{} due flashcard(s):", due.len());
        for card in &due {
            text.push_str(&format!("\n- {}: {}", card.front, card.back));
        }
        (ctx.notify)(&text);
        Ok(())
    }

    fn cmd_next(&self, ctx: &PluginContext<'_>) -> Result<(), String> {
        let deck = Deck::load(ctx)?;
        let now = today();
        let next = deck
            .cards
            .iter()
            .filter(|c| c.due <= now)
            .min_by_key(|c| c.due);
        match next {
            Some(card) => (ctx.notify)(&format!("Next: {} — {}", card.front, card.back)),
            None => (ctx.notify)("No due flashcards."),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;
    use std::rc::Rc;

    struct Harness {
        files: Rc<RefCell<HashMap<String, String>>>,
        notices: Rc<RefCell<Vec<String>>>,
    }

    impl Harness {
        fn new() -> (Self, PluginContext<'static>) {
            let files = Rc::new(RefCell::new(HashMap::new()));
            let notices = Rc::new(RefCell::new(Vec::new()));
            let f = Rc::clone(&files);
            let g = Rc::clone(&files);
            let n = Rc::clone(&notices);
            let ctx = PluginContext {
                vault_root: Path::new("."),
                read_file: Box::new(move |p| {
                    f.borrow()
                        .get(p)
                        .cloned()
                        .ok_or_else(|| format!("no such file: {p}"))
                }),
                write_file: Box::new(move |p, text| {
                    g.borrow_mut().insert(p.to_string(), text.to_string());
                    Ok(())
                }),
                append_file: Box::new(|_, _| Ok(())),
                create_file: Box::new(|_, _| Ok(())),
                create_folder: Box::new(|_| Ok(())),
                delete_file: Box::new(|_| Ok(())),
                rename_file: Box::new(|_, _| Ok(())),
                list_dir: Box::new(|_| Ok(Vec::new())),
                file_exists: Box::new(|_| Ok(false)),
                metadata_cache: Box::new(|_| Err("no metadata".to_string())),
                open_note: Box::new(|_| Ok(())),
                current_note: Box::new(|| None),
                notify: Box::new(move |line: &str| n.borrow_mut().push(line.to_string())),
            };
            (Self { files, notices }, ctx)
        }

        fn written(&self, path: &str) -> Option<String> {
            self.files.borrow().get(path).cloned()
        }

        fn notices(&self) -> Vec<String> {
            self.notices.borrow().clone()
        }
    }

    #[test]
    fn new_card_good_is_due_tomorrow() {
        let now = 100;
        let card = Card::new("1".into(), "front".into(), "back".into(), now);
        let updated = review(card, Rating::Good, now);
        assert_eq!(updated.due, now + 1);
        assert_eq!(updated.interval, 1);
        assert_eq!(updated.reps, 1);
        assert_eq!(updated.last_review, Some(now));
        assert!((updated.stability - 1.5).abs() < f64::EPSILON);
        assert!((updated.difficulty - 4.85).abs() < f64::EPSILON);
        assert_eq!(updated.lapses, 0);
    }

    #[test]
    fn new_card_easy_gets_four_day_first_interval() {
        let now = 100;
        let card = Card::new("1".into(), "front".into(), "back".into(), now);
        let updated = review(card, Rating::Easy, now);
        assert_eq!(updated.due, now + 4);
        assert_eq!(updated.interval, 4);
        assert!((updated.stability - 2.4).abs() < f64::EPSILON);
        assert_eq!(updated.lapses, 0);
    }

    #[test]
    fn again_on_new_card_lapses_and_resets() {
        let now = 100;
        let card = Card::new("1".into(), "front".into(), "back".into(), now);
        let updated = review(card, Rating::Again, now);
        assert_eq!(updated.interval, 1);
        assert_eq!(updated.due, now + 1);
        assert_eq!(updated.lapses, 1);
        assert!((updated.stability - 0.5).abs() < f64::EPSILON);
        assert!((updated.difficulty - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reviewed_card_grows_interval_from_stability() {
        let now = 100;
        let card = Card {
            id: "1".into(),
            front: "f".into(),
            back: "b".into(),
            due: now,
            interval: 2,
            stability: 2.0,
            difficulty: 5.0,
            reps: 1,
            lapses: 0,
            last_review: Some(now - 2),
            created_at: now - 2,
        };
        let updated = review(card, Rating::Good, now);
        assert_eq!(updated.interval, 2);
        assert_eq!(updated.due, now + 2);
        assert!((updated.stability - 3.0).abs() < f64::EPSILON);
        assert_eq!(updated.reps, 2);
    }

    #[test]
    fn again_always_interval_one_even_for_strong_card() {
        let now = 100;
        let card = Card {
            id: "1".into(),
            front: "f".into(),
            back: "b".into(),
            due: now,
            interval: 30,
            stability: 20.0,
            difficulty: 5.0,
            reps: 5,
            lapses: 0,
            last_review: Some(now - 30),
            created_at: now - 30,
        };
        let updated = review(card, Rating::Again, now);
        assert_eq!(updated.interval, 1);
        assert_eq!(updated.due, now + 1);
        assert_eq!(updated.lapses, 1);
    }

    #[test]
    fn easy_interval_exceeds_good_exceeds_hard() {
        let now = 100;
        let base = Card {
            id: "1".into(),
            front: "f".into(),
            back: "b".into(),
            due: now,
            interval: 5,
            stability: 5.0,
            difficulty: 5.0,
            reps: 2,
            lapses: 0,
            last_review: Some(now - 5),
            created_at: now - 5,
        };
        let hard = review(base.clone(), Rating::Hard, now).interval;
        let good = review(base.clone(), Rating::Good, now).interval;
        let easy = review(base, Rating::Easy, now).interval;
        assert!(hard < good && good < easy);
        assert_eq!(hard, 3);
        assert_eq!(good, 6);
        assert_eq!(easy, 12);
    }

    #[test]
    fn difficulty_is_clamped_to_bounds() {
        let now = 100;
        let hard_top = Card {
            id: "1".into(),
            front: "f".into(),
            back: "b".into(),
            due: now,
            interval: 1,
            stability: 1.0,
            difficulty: MAX_DIFFICULTY,
            reps: 1,
            lapses: 0,
            last_review: Some(now - 1),
            created_at: now - 1,
        };
        assert_eq!(
            review(hard_top, Rating::Again, now).difficulty,
            MAX_DIFFICULTY
        );

        let easy_floor = Card {
            id: "1".into(),
            front: "f".into(),
            back: "b".into(),
            due: now,
            interval: 1,
            stability: 1.0,
            difficulty: MIN_DIFFICULTY,
            reps: 1,
            lapses: 0,
            last_review: Some(now - 1),
            created_at: now - 1,
        };
        assert_eq!(
            review(easy_floor, Rating::Easy, now).difficulty,
            MIN_DIFFICULTY
        );
    }

    #[test]
    fn deck_saves_and_loads_through_the_context() {
        let (harness, ctx) = Harness::new();
        let now = 100;
        let card = Card::new("c1".into(), "Q".into(), "A".into(), now);
        let reviewed = review(card, Rating::Good, now);
        let mut deck = Deck::default();
        deck.cards.push(reviewed);
        assert!(deck.save(&ctx).is_ok());
        let text = harness.written(STORAGE).expect("deck file was written");
        let loaded: Deck = serde_json::from_str(&text).expect("deck is valid JSON");
        assert_eq!(loaded.cards.len(), 1);
        assert_eq!(loaded.cards[0].front, "Q");
        assert_eq!(loaded.cards[0].due, now + 1);
        assert!(loaded.cards[0].last_review.is_some());
    }

    #[test]
    fn load_with_no_file_returns_empty_deck() {
        let (harness, ctx) = Harness::new();
        let deck = Deck::load(&ctx).expect("missing file yields empty deck");
        assert!(deck.cards.is_empty());
        assert!(harness.written(STORAGE).is_none());
    }

    #[test]
    fn plugin_id_and_commands_are_registered() {
        let plugin = SpacedRepetition;
        assert_eq!(plugin.id(), "silica:spaced-repetition");
        assert_eq!(plugin.name(), "Spaced repetition");
        let ids: Vec<String> = plugin.commands().iter().map(|c| c.id.clone()).collect();
        assert_eq!(
            ids,
            vec![
                "spaced-repetition:add".to_string(),
                "spaced-repetition:review".to_string(),
                "spaced-repetition:due".to_string(),
                "spaced-repetition:next".to_string(),
            ]
        );
    }

    #[test]
    fn add_command_writes_a_due_card() {
        let (harness, ctx) = Harness::new();
        let plugin = SpacedRepetition;
        plugin
            .run_command(
                "spaced-repetition:add",
                Some(&serde_json::json!({"front": "Q", "back": "A"})),
                &ctx,
            )
            .unwrap();
        let text = harness.written(STORAGE).expect("add wrote the deck");
        let deck: Deck = serde_json::from_str(&text).expect("deck is valid JSON");
        assert_eq!(deck.cards.len(), 1);
        assert_eq!(deck.cards[0].front, "Q");
        assert_eq!(deck.cards[0].due, today());
        let notices = harness.notices();
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("Added flashcard: Q"));
    }

    #[test]
    fn review_command_moves_a_card_and_persists() {
        let (harness, ctx) = Harness::new();
        let plugin = SpacedRepetition;
        let now = 100;
        plugin
            .run_command(
                "spaced-repetition:add",
                Some(&serde_json::json!({"front": "Q", "back": "A"})),
                &ctx,
            )
            .unwrap();
        // Force the freshly added card's due to `now` by reviewing it at `now`.
        // The card was stamped due == today(); set storage directly to control dates.
        let mut deck: Deck = serde_json::from_str(&harness.written(STORAGE).unwrap()).unwrap();
        deck.cards[0].due = now;
        deck.cards[0].created_at = now;
        let text = serde_json::to_string(&deck).unwrap();
        (ctx.write_file)(STORAGE, &text).unwrap();

        plugin
            .run_command(
                "spaced-repetition:review",
                Some(&serde_json::json!({"id": deck.cards[0].id, "rating": "good"})),
                &ctx,
            )
            .unwrap();
        let updated: Deck = serde_json::from_str(&harness.written(STORAGE).unwrap()).unwrap();
        assert_eq!(updated.cards[0].reps, 1);
        assert_eq!(updated.cards[0].interval, 1);
        let notices = harness.notices();
        assert!(notices.iter().any(|n| n.contains("next due in 1 day(s)")));
    }

    #[test]
    fn due_command_lists_only_due_cards() {
        let (harness, ctx) = Harness::new();
        let plugin = SpacedRepetition;
        let now = today();
        let mut deck = Deck::default();
        deck.cards.push(Card {
            id: "a".into(),
            front: "due".into(),
            back: "b".into(),
            due: now,
            interval: 1,
            stability: 1.0,
            difficulty: 5.0,
            reps: 1,
            lapses: 0,
            last_review: None,
            created_at: now,
        });
        deck.cards.push(Card {
            id: "b".into(),
            front: "future".into(),
            back: "c".into(),
            due: now + 10,
            interval: 10,
            stability: 1.0,
            difficulty: 5.0,
            reps: 1,
            lapses: 0,
            last_review: None,
            created_at: now,
        });
        deck.save(&ctx).unwrap();
        plugin
            .run_command("spaced-repetition:due", None, &ctx)
            .unwrap();
        let notices = harness.notices();
        assert!(notices.iter().any(|n| n.contains("1 due flashcard(s)")));
        assert!(notices.iter().any(|n| n.contains("due: b")));
        assert!(!notices.iter().any(|n| n.contains("future")));
    }

    #[test]
    fn next_command_picks_earliest_due_card() {
        let (harness, ctx) = Harness::new();
        let plugin = SpacedRepetition;
        let now = today();
        let mut deck = Deck::default();
        deck.cards.push(Card {
            id: "a".into(),
            front: "later".into(),
            back: "b".into(),
            due: now - 5,
            interval: 5,
            stability: 1.0,
            difficulty: 5.0,
            reps: 1,
            lapses: 0,
            last_review: None,
            created_at: now,
        });
        deck.cards.push(Card {
            id: "b".into(),
            front: "sooner".into(),
            back: "c".into(),
            due: now - 1,
            interval: 1,
            stability: 1.0,
            difficulty: 5.0,
            reps: 1,
            lapses: 0,
            last_review: None,
            created_at: now,
        });
        deck.save(&ctx).unwrap();
        plugin
            .run_command("spaced-repetition:next", None, &ctx)
            .unwrap();
        let notices = harness.notices();
        assert!(notices.iter().any(|n| n.contains("Next: later — b")));
    }

    #[test]
    fn unknown_command_is_an_error() {
        let (_harness, ctx) = Harness::new();
        let plugin = SpacedRepetition;
        let result = plugin.run_command("spaced-repetition:nope", None, &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("spaced-repetition:nope"));
    }

    #[test]
    fn rating_strings_parse_case_insensitively() {
        assert_eq!(
            parse_rating(&serde_json::json!({"rating": "GOOD"})),
            Ok(Rating::Good)
        );
        assert_eq!(
            parse_rating(&serde_json::json!({"rating": "Easy"})),
            Ok(Rating::Easy)
        );
        assert_eq!(
            parse_rating(&serde_json::json!({"rating": "again"})),
            Ok(Rating::Again)
        );
        assert!(parse_rating(&serde_json::json!({"rating": "maybe"})).is_err());
    }
}
