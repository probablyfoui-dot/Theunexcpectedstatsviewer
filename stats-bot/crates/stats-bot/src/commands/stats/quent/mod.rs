// crates/stats-bot/src/commands/stats/quent/mod.rs — /quent command with 26 categories

// ---- Imports ---- //

use anyhow::Result;
use serde_json::Value;
use serenity::all::*;
use std::sync::Arc;

use crate::api::fetch_hypixel_player;
use crate::commands::stats::resolve_player_or_link;
use crate::framework::BotData;
use hypixel::{calculate_network_level, extract_rank_prefix};
use render::cards::format_number;

// ---- Color helpers ---- //

fn rank_embed_color(player: &Value) -> u32 {
    let prefix = extract_rank_prefix(player).unwrap_or_default();
    if prefix.contains("§c") {
        return 0xFF5555;
    }
    if prefix.contains("§6") {
        return 0xFFAA00;
    }
    if prefix.contains("§b") {
        return 0x55FFFF;
    }
    if prefix.contains("§a") {
        return 0x55FF55;
    }
    if prefix.contains("§2") {
        return 0x00AA00;
    }
    if prefix.contains("§5") {
        return 0xAA00AA;
    }
    0x808080
}

fn face_url(uuid: &str) -> String {
    format!("https://mc-heads.net/avatar/{}/64", uuid)
}

fn rank_name(player: &Value, username: &str) -> String {
    let prefix = extract_rank_prefix(player).unwrap_or_else(|| "§7".to_string());
    let display = player["displayname"].as_str().unwrap_or(username);
    strip_mc(&format!("{prefix}{display}"))
}

fn strip_mc(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

fn u64_val(v: &Value) -> u64 {
    v.as_u64()
        .or_else(|| v.as_f64().map(|f| f as u64))
        .unwrap_or(0)
}

fn str_val<'a>(v: &'a Value, default: &'a str) -> &'a str {
    v.as_str().unwrap_or(default)
}

type Stats = Vec<(&'static str, String)>;

// ── Categories ─────────────────────────────────────────────────────────────────

// ---- bwminion() ---- //

fn bwminion(p: &Value) -> (&'static str, Stats) {
    let m = &p["stats"]["Bedwars"]["slumber"]["minion"];
    (
        "Slumber Hotel Minion",
        vec![
            (
                "Ender Dust Collected",
                format_number(u64_val(&m["ender_dust_collected"])),
            ),
            (
                "Tickets Collected",
                format_number(u64_val(&m["tickets_collected"])),
            ),
        ],
    )
}

// ---- gamblegeorge() ---- //

fn gamblegeorge(p: &Value) -> (&'static str, Stats) {
    let w =
        u64_val(&p["stats"]["Bedwars"]["slumber"]["quest"]["gambler_george"]["gamble_games_won"]);
    (
        "Gamble George",
        vec![("Gamble Games Won", format_number(w))],
    )
}

// ---- privategames() ---- //

fn privategames(p: &Value) -> (&'static str, Stats) {
    let pg = &p["stats"]["Bedwars"]["privategames"];
    (
        "Private Games",
        vec![("Event Time", str_val(&pg["event_time"], "Normal").into())],
    )
}

// ---- practice() ---- //

fn practice(p: &Value) -> (&'static str, Stats) {
    let b = &p["stats"]["Bedwars"]["practice"]["bridging"];
    (
        "Practice",
        vec![
            (
                "Bridging Blocks Placed",
                format_number(u64_val(&b["blocks_placed"])),
            ),
            (
                "Selected Practice",
                str_val(&p["stats"]["Bedwars"]["practice"]["selected"], "None").into(),
            ),
        ],
    )
}

// ---- favoritemaps() ---- //

fn favoritemaps(p: &Value) -> (&'static str, Stats) {
    let mut maps: Vec<String> = p["stats"]["Bedwars"]["packages"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .filter(|s| s.starts_with("favoritemap_"))
                .map(|s| {
                    s.strip_prefix("favoritemap_")
                        .unwrap_or(s)
                        .replace('_', " ")
                })
                .collect()
        })
        .unwrap_or_default();
    maps.sort();
    let mut items = vec![("Total Favorite Maps", maps.len().to_string())];
    for m in &maps {
        items.push(("Map", m.clone()));
    }
    ("Favorite Maps", items)
}

// ---- ultimate() ---- //

fn ultimate(p: &Value) -> (&'static str, Stats) {
    (
        "Ultimate",
        vec![(
            "Selected Ultimate (Dream BW)",
            str_val(&p["stats"]["Bedwars"]["selected_ultimate"], "None").into(),
        )],
    )
}

// ---- bwchallenges() ---- //

fn bwchallenges(p: &Value) -> (&'static str, Stats) {
    let bw = &p["stats"]["Bedwars"];
    let selected = str_val(&bw["selected_challenge_type"], "None");
    let total = u64_val(&bw["total_challenges_completed"]);
    let unique = u64_val(&bw["bw_unique_challenges_completed"]);
    let mut best_time: u64 = 0;
    let mut best_name = String::new();
    if let Some(ch) = bw["challenges"].as_object() {
        for (k, v) in ch {
            if let Some(t) = v.as_u64() {
                if t > best_time && k.ends_with("_best_time") {
                    best_time = t;
                    best_name = k.strip_suffix("_best_time").unwrap_or(k).to_string();
                }
            }
        }
    }
    let longest = if best_time > 0 {
        let mins = best_time / 60000;
        let secs = (best_time % 60000) / 1000;
        format!("{best_name}: {mins}m{secs}s")
    } else {
        "None".into()
    };
    (
        "BW Challenges",
        vec![
            ("Selected", selected.into()),
            ("Completed (total)", format_number(total)),
            ("Uncompleted", format_number(unique.saturating_sub(total))),
            ("Longest Completion", longest),
        ],
    )
}

// ---- vanity() ---- //

fn vanity(p: &Value) -> (&'static str, Stats) {
    let sled_type = str_val(&p["vanityMeta"]["gadgetSledType"], "None").replace('_', " ");
    let glow = if p["battlePassGlowStatus"].as_bool().unwrap_or(false) {
        "Yes ✨".into()
    } else {
        "No".into()
    };
    let fav = str_val(&p["vanityFavorites"], "").to_string();
    let fav_count = if fav.is_empty() {
        0
    } else {
        fav.split(';').count()
    };
    (
        "Vanity",
        vec![
            ("Sled", sled_type),
            ("Glow Effect in Lobby", glow),
            ("Vanity Favorites", format_number(fav_count as u64)),
        ],
    )
}

// ---- collectibles() ---- //

fn collectibles(p: &Value) -> (&'static str, Stats) {
    let n = p["vanityMeta"]["packages"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    ("Collectibles", vec![("Collectibles Owned", n.to_string())])
}

// ---- language() ---- //

fn language(p: &Value) -> (&'static str, Stats) {
    (
        "Language",
        vec![(
            "User Language",
            str_val(&p["userLanguage"], "unknown").into(),
        )],
    )
}

// ---- tracked() ---- //

fn tracked(p: &Value) -> (&'static str, Stats) {
    let n = p["achievementTracking"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let first: Vec<String> = p["achievementsOneTime"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .take(3)
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    let mut items = vec![("Achievements Being Tracked", n.to_string())];
    if !first.is_empty() {
        items.push(("First 3 Achievements", first.join(", ")));
    }
    ("Tracked & First Achievements", items)
}

// ---- level() ---- //

fn level(p: &Value) -> (&'static str, Stats) {
    let exp = p["networkExp"].as_f64().unwrap_or(0.0);
    let lvl = calculate_network_level(exp).floor() as u64;
    let mult = exp / 79_680_000.0;
    (
        "Level Breakdown",
        vec![
            ("Network Level", format_number(lvl)),
            ("Network XP", format_number(exp as u64)),
            ("× Level 250", format!("{:.3}×", mult)),
        ],
    )
}

// ---- housingadvanced() ---- //

fn housingadvanced(p: &Value) -> (&'static str, Stats) {
    let on = str_val(
        &p["housingMeta"]["playerSettings"]["ADVANCED_VARIABLE_OPERATIONS"],
        "",
    ) == "BooleanState-true";
    let channels: Vec<String> = p["housingMeta"]["selectedChannels_v3"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    let mut items = vec![(
        "Advanced Variable Operations",
        if on { "Enabled ✅" } else { "Disabled ❌" }.into(),
    )];
    if !channels.is_empty() {
        items.push(("Selected Housing Channels", channels.join(", ")));
    }
    ("Housing Settings", items)
}

// ---- cookies() ---- //

fn cookies(p: &Value) -> (&'static str, Stats) {
    let total: usize = p["housingMeta"]
        .as_object()
        .map(|m| {
            m.iter()
                .filter(|(k, _)| k.starts_with("given_cookies_"))
                .filter_map(|(_, v)| v.as_array())
                .map(|a| a.len())
                .sum()
        })
        .unwrap_or(0);
    (
        "Cookies Donated",
        vec![("Housing Cookies Donated", format_number(total as u64))],
    )
}

// ---- soulwell() ---- //

fn soulwell(p: &Value) -> (&'static str, Stats) {
    let sw = &p["stats"]["SkyWars"];
    (
        "SkyWars Soul Well",
        vec![
            (
                "Legendaries Opened",
                format_number(u64_val(&sw["soul_well_legendaries"])),
            ),
            (
                "Rares Opened",
                format_number(u64_val(&sw["soul_well_rares"])),
            ),
            (
                "Souls Gathered",
                format_number(u64_val(&sw["souls_gathered"])),
            ),
            (
                "Times Bought Souls",
                format_number(u64_val(&sw["paid_souls"])),
            ),
        ],
    )
}

// ---- coins() ---- //

fn coins(p: &Value) -> (&'static str, Stats) {
    let stats = &p["stats"];
    let mut entries: Vec<(&str, u64)> = Vec::new();
    let games = [
        "Arcade",
        "Arena",
        "Battleground",
        "Bedwars",
        "BuildBattle",
        "Duels",
        "GingerBread",
        "Housing",
        "HungerGames",
        "Legacy",
        "MCGO",
        "MurderMystery",
        "Paintball",
        "Quake",
        "SkyClash",
        "SkyWars",
        "SpeedUHC",
        "SuperSmash",
        "TNTGames",
        "TrueCombat",
        "UHC",
        "VampireZ",
        "Walls",
        "Walls3",
        "WoolGames",
    ];
    for game in &games {
        let v = u64_val(&stats[game]["coins"]);
        if v > 0 {
            entries.push((game, v));
        }
    }
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    let mut result = Vec::new();
    for (g, c) in entries {
        result.push(("Coin", format!("{g}: {}", format_number(c))));
    }
    ("Coins Per Game", result)
}

// ---- pet() ---- //

fn pet(p: &Value) -> (&'static str, Stats) {
    let cons = p["petConsumables"]
        .as_object()
        .map(|o| {
            let total: u64 = o.values().map(|v| u64_val(v)).sum();
            let count = o.len();
            (total, count)
        })
        .unwrap_or((0, 0));
    let ts = p["petJourneyTimestamp"].as_i64().unwrap_or(0);
    let journey = if ts > 0 {
        let secs = ts / 1000;
        let days = (chrono::Utc::now().timestamp() - secs) / 86400;
        format!("{days} days ago")
    } else {
        "Never".into()
    };
    (
        "Pet Info",
        vec![
            ("Consumable Types", cons.1.to_string()),
            ("Total Consumables", format_number(cons.0)),
            ("Last Pet Journey", journey),
        ],
    )
}

// ---- seasonalminigames() ---- //

fn seasonalminigames(p: &Value) -> (&'static str, Stats) {
    let s = &p["seasonal"];
    let mut entries = Vec::new();
    for season in &["halloween", "christmas", "easter", "summer"] {
        let years = s[season].as_object();
        if let Some(years) = years {
            let mut best: i64 = 0;
            for (_year, data) in years {
                let xp = data["levelling"]["experience"].as_i64().unwrap_or(0);
                if xp > best {
                    best = xp;
                }
            }
            if best > 0 {
                let name = match *season {
                    "halloween" => "Halloween",
                    "christmas" => "Christmas",
                    "easter" => "Easter",
                    "summer" => "Summer",
                    _ => season,
                };
                entries.push((name, format_number(best as u64)));
            }
        }
    }
    entries.sort();
    let mut items = vec![("Seasons With XP", entries.len().to_string())];
    for (name, xp) in entries {
        items.push(("Season", format!("{name}: {xp} XP")));
    }
    ("Seasonal Minigames (Highest XP)", items)
}

// ---- duels() ---- //

fn duels(p: &Value) -> (&'static str, Stats) {
    let d = &p["stats"]["Duels"];
    (
        "Duels Settings",
        vec![
            (
                "Chat Enabled",
                str_val(&d["chat_enabled"], "unknown").into(),
            ),
            ("Longest Combo", format_number(u64_val(&d["longest_combo"]))),
            ("", String::new()),
            ("── Arena Preferences ──", String::new()),
            ("Arena Bow", str_val(&d["arena_mode_bow"], "NORMAL").into()),
            (
                "Arena Classic",
                str_val(&d["arena_mode_classic"], "NORMAL").into(),
            ),
            ("Arena OP", str_val(&d["arena_mode_op"], "NORMAL").into()),
            ("Arena UHC", str_val(&d["arena_mode_uhc"], "NORMAL").into()),
        ],
    )
}

// ---- deliveryman() ---- //

fn deliveryman(p: &Value) -> (&'static str, Stats) {
    let tokens = u64_val(&p["adsense_tokens"]);
    let last = p["lastAdsenseGenerateTime"].as_i64().unwrap_or(0);
    let gen = if last > 0 {
        let secs = last / 1000;
        let days = (chrono::Utc::now().timestamp() - secs) / 86400;
        format!("{days} days ago")
    } else {
        "Never".into()
    };
    (
        "Delivery Man",
        vec![
            ("Daily Reward Tokens", format_number(tokens)),
            ("Last Token Generated", gen),
        ],
    )
}

// ---- warlord() ---- //

fn warlord(p: &Value) -> (&'static str, Stats) {
    let bg = &p["stats"]["Battleground"];
    let mvp = u64_val(&bg["mvp_count"]);
    let wins = u64_val(&bg["wins"]);
    let kills = u64_val(&bg["kills"]);
    let prestiged: Vec<String> = bg["prestiged"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    let mut items = vec![
        ("MVP Count", format_number(mvp)),
        ("Wins", format_number(wins)),
        ("Kills", format_number(kills)),
    ];
    if !prestiged.is_empty() {
        items.push(("Prestiged Classes", prestiged.join(", ")));
    }
    ("Warlords", items)
}

// ---- wool() ---- //

fn wool(p: &Value) -> (&'static str, Stats) {
    let wg = &p["stats"]["WoolGames"];
    let sw = &wg["sheep_wars"];
    let ctw = &wg["capture_the_wool"]["stats"];
    let layout = sw["layout"]["slot"]
        .as_object()
        .map(|slots| {
            let mut items: Vec<String> = Vec::new();
            for i in 0..9u32 {
                if let Some(v) = slots.get(&i.to_string()).and_then(|v| v.as_str()) {
                    items.push(v.to_string());
                }
            }
            items.join(", ")
        })
        .unwrap_or_else(|| "default".into());
    (
        "Wool Games",
        vec![
            (
                "Sheep Wars Wins",
                format_number(u64_val(&sw["stats"]["wins"])),
            ),
            (
                "Sheep Wars Kills",
                format_number(u64_val(&sw["stats"]["kills"])),
            ),
            ("Sheep Layout", layout),
            ("CTW Kills", format_number(u64_val(&ctw["kills"]))),
            (
                "CTW Wools Captured",
                format_number(u64_val(&ctw["wools_captured"])),
            ),
            ("CTW Draws", format_number(u64_val(&ctw["draw"]))),
        ],
    )
}

// ---- pit() ---- //

fn pit(p: &Value) -> (&'static str, Stats) {
    let profile = &p["stats"]["Pit"]["profile"];
    let cash = profile["cash"].as_f64().unwrap_or(0.0);
    (
        "The Pit",
        vec![
            (
                "Genesis Allegiance",
                str_val(&profile["genesis_allegiance"], "None").into(),
            ),
            (
                "Genesis Points",
                format_number(u64_val(&profile["genesis_points"])),
            ),
            ("Cash", format!("{:.0}", cash)),
        ],
    )
}

// ---- paintball() ---- //

fn paintball(p: &Value) -> (&'static str, Stats) {
    let slots = str_val(&p["stats"]["Paintball"]["favorite_slots"], "None").replace(',', ", ");
    (
        "Paintball",
        vec![
            ("Favorite Slots", slots),
            (
                "Kills",
                format_number(u64_val(&p["stats"]["Paintball"]["kills"])),
            ),
        ],
    )
}

// ---- crates() ---- //

fn crates(p: &Value) -> (&'static str, Stats) {
    let opened = p["monthlycrates"].as_object().map(|m| m.len()).unwrap_or(0);
    (
        "Monthly Crates",
        vec![("Crates Opened", format_number(opened as u64))],
    )
}

// ---- superstar() ---- //

fn superstar(p: &Value) -> (&'static str, Stats) {
    let is_superstar = str_val(&p["monthlyPackageRank"], "NONE") == "SUPERSTAR";
    let months = p["monthlycrates"].as_object().map(|m| m.len()).unwrap_or(0);
    (
        "Superstar",
        vec![
            (
                "Active Subscription",
                if is_superstar {
                    "SUPERSTAR 💎"
                } else {
                    "None"
                }
                .into(),
            ),
            ("Rank Color", str_val(&p["monthlyRankColor"], "none").into()),
            ("Months Subscribed", format_number(months as u64)),
        ],
    )
}

// ---- MODE_CATS table ---- //

// ── Gamemode → category map for autocomplete & validation ─────────────────────

const MODE_CATS: &[(&str, &[(&str, &str)])] = &[
    (
        "bedwars",
        &[
            ("Slumber Hotel Minion", "bwminion"),
            ("Gamble George", "gamblegeorge"),
            ("Private Games", "privategames"),
            ("Practice", "practice"),
            ("Favorite Maps", "favoritemaps"),
            ("Ultimate", "ultimate"),
            ("BW Challenges", "challenges"),
        ],
    ),
    ("skywars", &[("SkyWars Soul Well", "soulwell")]),
    (
        "general",
        &[
            ("Vanity", "vanity"),
            ("Collectibles", "collectibles"),
            ("Language", "language"),
            ("Tracked & First Achieve", "tracked"),
            ("Level Breakdown", "level"),
        ],
    ),
    (
        "housing",
        &[
            ("Housing Settings", "housingadvanced"),
            ("Cookies Donated", "cookies"),
        ],
    ),
    (
        "games",
        &[
            ("Coins Per Game", "coins"),
            ("Pet Info", "pet"),
            ("Seasonal Minigames", "seasonal"),
            ("Duels Settings", "duels"),
            ("Delivery Man", "deliveryman"),
            ("Warlords", "warlord"),
            ("Wool Games", "wool"),
            ("The Pit", "pit"),
            ("Paintball", "paintball"),
            ("Monthly Crates", "crates"),
            ("Superstar", "superstar"),
        ],
    ),
];

// ---- about() descriptions ---- //

fn about(cat: &str) -> &'static str {
    match cat {
        "bwminion" => "The Slumber hotel minion earns ender dust and tickets passively.",
        "gamblegeorge" => "NPC in the slumber hotel. How many times you've won against him.",
        "privategames" => "Bedwars private game settings like event time multiplier.",
        "practice" => "Bridging blocks placed and selected practice mode.",
        "favoritemaps" => "Bedwars maps you've marked as favourite.",
        "ultimate" => "Selected ultimate ability in Dream Bedwars.",
        "challenges" => "Selected BW challenge, completions, uncompleted count, and longest time.",
        "vanity" => "Sled type, glow effect in lobby, and vanity favorites count.",
        "collectibles" => "Total vanity packages (collectibles) owned.",
        "language" => "Your selected Hypixel client language.",
        "tracked" => "Achievements tracked on scoreboard plus first 3 ever unlocked.",
        "level" => "Your network level, raw XP, and XP as a multiple of level 250.",
        "housingadvanced" => "Advanced variable operations and selected housing channels.",
        "cookies" => "Cookie donations given to other players in housing.",
        "soulwell" => {
            "SkyWars soul well legendaries, rares, souls gathered, and times bought souls."
        }
        "coins" => "Total coins across every game mode, ranked highest first.",
        "pet" => "Pet consumables inventory, total quantity, and last journey timestamp.",
        "seasonal" => "Highest seasonal lobby XP across Halloween, Christmas, Easter, Summer.",
        "duels" => "Duels chat toggle, longest combo, and arena difficulty preferences.",
        "deliveryman" => "Daily reward tokens and last generation time.",
        "warlord" => "Warlords MVP count, wins, kills, and prestiged classes.",
        "wool" => "Sheep Wars stats, hotbar layout, and Capture The Wool stats.",
        "pit" => "The Pit Genesis allegiance, points, and total cash.",
        "paintball" => "Paintball kill count and favourite perk loadout.",
        "crates" => "Monthly crate reward history.",
        "superstar" => "[MVP++] subscription status, rank color, and months subscribed.",
        _ => "",
    }
}

// ---- register() ---- //

pub fn register() -> CreateCommand<'static> {
    CreateCommand::new("quent")
        .description("Unusual Hypixel stats")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "category", "Stat category")
                .required(true)
                .set_autocomplete(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "player",
                "Player name or UUID (leave empty to use linked account)",
            )
            .required(false),
        )
}

// ---- autocomplete() ---- //

pub async fn autocomplete(ctx: &Context, aut: &CommandInteraction) -> Result<()> {
    let mut response = CreateAutocompleteResponse::new();
    for &(_, cats) in MODE_CATS {
        for &(name, val) in cats {
            response = response.add_choice(AutocompleteChoice::new(name, val));
        }
    }
    aut.create_response(&ctx.http, CreateInteractionResponse::Autocomplete(response))
        .await?;
    Ok(())
}

// ---- run() ---- //

pub async fn run(ctx: &Context, cmd: &CommandInteraction, data: &Arc<BotData>) -> Result<()> {
    let options = &cmd.data.options;
    let category = options
        .get(0)
        .and_then(|o| o.value.as_str())
        .unwrap_or("")
        .to_string();

    cmd.defer(&ctx.http).await?;

    let input = options
        .get(1)
        .and_then(|o| o.value.as_str())
        .map(String::from);
    let author_id = cmd.user.id.to_string();
    let identity = match resolve_player_or_link(input, &author_id, data.as_ref()).await {
        Ok(id) => id,
        Err(e) => {
            return crate::commands::stats::send_error(ctx, cmd, "Player Not Found", &e.to_string())
                .await
        }
    };

    let player = match fetch_hypixel_player(&identity.uuid).await {
        Ok(p) => p,
        Err(_) => {
            return crate::commands::stats::send_error(
                ctx,
                cmd,
                "API Error",
                "Could not fetch Hypixel data.",
            )
            .await
        }
    };

    let in_mode = MODE_CATS
        .iter()
        .any(|(_, cats)| cats.iter().any(|(_, v)| *v == category));
    if !in_mode {
        return crate::commands::stats::send_error(
            ctx,
            cmd,
            "Invalid Category",
            &format!("`{category}` is not a valid category."),
        )
        .await;
    }

    let (subtitle, stats) = match category.as_str() {
        "bwminion" => bwminion(&player),
        "gamblegeorge" => gamblegeorge(&player),
        "privategames" => privategames(&player),
        "practice" => practice(&player),
        "favoritemaps" => favoritemaps(&player),
        "ultimate" => ultimate(&player),
        "challenges" => bwchallenges(&player),
        "vanity" => vanity(&player),
        "collectibles" => collectibles(&player),
        "language" => language(&player),
        "tracked" => tracked(&player),
        "level" => level(&player),
        "housingadvanced" => housingadvanced(&player),
        "cookies" => cookies(&player),
        "soulwell" => soulwell(&player),
        "coins" => coins(&player),
        "pet" => pet(&player),
        "seasonal" => seasonalminigames(&player),
        "duels" => duels(&player),
        "deliveryman" => deliveryman(&player),
        "warlord" => warlord(&player),
        "wool" => wool(&player),
        "pit" => pit(&player),
        "paintball" => paintball(&player),
        "crates" => crates(&player),
        "superstar" => superstar(&player),
        _ => {
            return crate::commands::stats::send_error(
                ctx,
                cmd,
                "Unknown Category",
                "Unknown category.",
            )
            .await
        }
    };

    let desc = stats
        .iter()
        .map(|(label, value)| {
            if label.is_empty() {
                String::new()
            } else if value.is_empty() {
                format!("**{label}**")
            } else {
                format!("**{label}:** {value}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let info = about(&category);
    let footer_text = if info.is_empty() {
        String::new()
    } else {
        format!("ℹ️  {info}")
    };

    let title = format!("{}'s {}", rank_name(&player, &identity.name), subtitle);
    let color = rank_embed_color(&player);
    let thumb = face_url(&identity.uuid);

    let embed = CreateEmbed::new()
        .title(title)
        .description(desc)
        .color(color)
        .thumbnail(thumb, None)
        .footer(CreateEmbedFooter::new(footer_text));

    cmd.edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}
