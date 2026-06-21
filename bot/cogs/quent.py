"""
cogs/quent.py
-------------
/quent command
unusual stuff
"""

import io
import os
import math
import datetime
import aiohttp
import discord
from discord import app_commands
from discord.ext import commands
from PIL import Image, ImageDraw, ImageFont

from core import fetch_uuid, fetch_player, FONT_PATH, FONT_SYMBOLS_PATH
from cogs.hypixel import get_rank_display, draw_segments, segments_width

# ---------------------------------------------------------------------------
# PATHS
# ---------------------------------------------------------------------------

_BOT_ROOT           = os.path.dirname(os.path.dirname(__file__))
MINECRAFT_FONT_PATH = os.path.join(_BOT_ROOT, "Minecraft.ttf")
MINION_IMG_PATH     = os.path.join(_BOT_ROOT, "minion.png")

# ---------------------------------------------------------------------------
# FONTS
# ---------------------------------------------------------------------------

def load_font(size=10):
    try:
        return ImageFont.truetype(FONT_PATH, size)
    except Exception:
        return ImageFont.load_default()

# ---------------------------------------------------------------------------
# PALETTE
# ---------------------------------------------------------------------------

C_BG       = (12,  12,  20)
C_PANEL    = (22,  24,  38)
C_BORDER   = (45,  50,  75)
C_GREEN    = (80,  230, 120)
C_CYAN     = (0,   200, 255)
C_GOLD     = (255, 200,  50)
C_RED      = (255,  80,  80)
C_WHITE    = (230, 235, 245)
C_GRAY     = (120, 130, 150)
C_PURPLE   = (255,  85, 255)
C_YELLOW   = (255, 255,  85)
C_SHADOW   = (63,   61,  22)

# ---------------------------------------------------------------------------
# ABOUT DESCRIPTIONS
# ---------------------------------------------------------------------------

ABOUT = {
    "bwminion":        "Informations about the minion behind the door from the sky, it generates enderdust.",
    "gamblegeorge":    "NPC from the slumber hotel, currently doesn't work.",
    "sled":            "Gadget with different forms.",
    "collectibles":    "Number of collectibles owned by the player.",
    "practiceblocks":  "Number of blocks placed in the Bedwars practice.",
    "cookies":         "Number of cookies donated in housing.",
    "tracked":         "Number of achievements tracked by the player.",
    "souls":           "Number of souls bought (legacy stat).",
    "favoritemaps":    "Bedwars maps set as favorite.",
    "ultimate":        'Selected ultimate in Bedwars dream mode "Ultimate".',
    "housingadvanced": "Use of complex features within the housing gamemode.",
    "level":           "Advanced vision of the player's level.",
    "backdropwins":    "Number of wins per backdrop (background) in BuildBattle.",
}

# ---------------------------------------------------------------------------
# DRAW PRIMITIVES
# ---------------------------------------------------------------------------

def panel(draw, x, y, w, h, color=C_PANEL, border=C_BORDER, radius=6):
    draw.rounded_rectangle([x, y, x+w, y+h], radius=radius,
                           fill=color, outline=border, width=1)

def px(draw, x, y, text, font, color=C_WHITE, anchor="la"):
    draw.text((x, y), text, font=font, fill=color, anchor=anchor)

# ---------------------------------------------------------------------------
# IMAGE BUILDER
# ---------------------------------------------------------------------------

W = 860

def build_image(title_segments, title_base_color: tuple,
                ign: str, cells: list, category: str = "",
                grid: bool = False) -> io.BytesIO:

    PAD   = 16
    GAP   = 6

    f12   = load_font(12)
    f16   = load_font(16)
    f22   = load_font(22)
    f28   = load_font(28)   # IGN big

    # Header height: 4px bar + 8 padding + 28 IGN + 6 gap + 16 category + 10 padding
    header_h = 4 + 8 + 28 + 6 + 16 + 10
    sep_y    = header_h
    GY       = sep_y + 10

    if grid:
        COLS = 3
        BH   = 90
        rows = math.ceil(len(cells) / COLS)
        CW   = (W - PAD * 2 - GAP * (COLS - 1)) // COLS
        H    = GY + rows * (BH + GAP) + PAD
    else:
        ROW_H = 52
        H     = GY + len(cells) * (ROW_H + GAP) + PAD

    img = Image.new("RGB", (W, H), C_BG)
    d   = ImageDraw.Draw(img)
    RX  = PAD
    RW  = W - PAD * 2

    # Accent bar
    d.rectangle([0, 0, W, 4], fill=title_base_color)

    # IGN line (rank segments + IGN)
    ign_y = 12
    seg_w = segments_width(d, title_segments, f28) if title_segments else 0
    draw_segments(d, RX, ign_y, title_segments, f28)
    px(d, RX + seg_w + (10 if title_segments else 0), ign_y, ign, f28, title_base_color)

    # Category subtitle
    cat_y = ign_y + 28 + 6
    px(d, RX, cat_y, category.upper(), f16, C_GRAY)

    # Separator
    d.line([(RX, sep_y), (W - PAD, sep_y)], fill=C_BORDER, width=1)

    if grid:
        for i, (lbl, val, col) in enumerate(cells):
            row_i, col_i = divmod(i, COLS)
            bx = RX + col_i * (CW + GAP)
            by = GY + row_i * (BH + GAP)
            panel(d, bx, by, CW, BH)
            px(d, bx + 12, by + 10, lbl, f12, C_GRAY)
            px(d, bx + 12, by + 36, val, f22, col)
    else:
        for i, (lbl, val, col) in enumerate(cells):
            by = GY + i * (ROW_H + GAP)
            panel(d, RX, by, RW, ROW_H)
            px(d, RX + 16, by + (ROW_H - 22) // 2, lbl, f16, C_GRAY)
            val_w = int(d.textlength(val, f22))
            px(d, RX + RW - 16 - val_w, by + (ROW_H - 22) // 2, val, f22, col)

    buf = io.BytesIO()
    img.save(buf, "PNG")
    buf.seek(0)
    return buf

# ---------------------------------------------------------------------------
# ABOUT VIEW
# ---------------------------------------------------------------------------

class AboutView(discord.ui.View):
    def __init__(self, category: str):
        super().__init__(timeout=120)
        self.category = category

    @discord.ui.button(label="About", style=discord.ButtonStyle.secondary, emoji="ℹ️")
    async def about(self, interaction: discord.Interaction, button: discord.ui.Button):
        desc = ABOUT.get(self.category, "No description available.")
        await interaction.response.send_message(
            f"**{self.category.upper()}**\n{desc}",
            ephemeral=True
        )

# ---------------------------------------------------------------------------
# HANDLERS
# ---------------------------------------------------------------------------

async def handle_bwminion(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    stats = player.get("stats", {}).get("Bedwars", {}).get("slumber", {}).get("minion", {})
    if not stats:
        await interaction.followup.send("No Bed Wars minion stats found for this player.")
        return

    dust    = stats.get("ender_dust_collected", 0)
    tickets = stats.get("tickets_collected", 0)

    image   = Image.open(MINION_IMG_PATH).convert("RGBA")
    overlay = Image.new("RGBA", image.size, (255, 255, 255, 0))
    draw    = ImageDraw.Draw(overlay)

    try:
        font = ImageFont.truetype(MINECRAFT_FONT_PATH, 26)
    except Exception:
        font = load_font(26)

    draw.text((350, 411), f"{dust:,}",    font=font, fill=C_SHADOW)
    draw.text((288, 441), f"{tickets:,}", font=font, fill=C_SHADOW)
    draw.text((347, 408), f"{dust:,}",    font=font, fill=C_YELLOW)
    draw.text((285, 438), f"{tickets:,}", font=font, fill=C_YELLOW)

    final = Image.alpha_composite(image, overlay)
    buf   = io.BytesIO()
    final.save(buf, "PNG")
    buf.seek(0)
    await interaction.followup.send(file=discord.File(buf, "bwminion.png"), view=AboutView(category))


async def handle_gamblegeorge(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    wins = (
        player.get("stats", {})
              .get("Bedwars", {})
              .get("slumber", {})
              .get("quest", {})
              .get("gambler_george", {})
              .get("gamble_games_won", 0)
    )
    cells = [(category.upper(), f"{wins:,}", C_GOLD)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "gamblegeorge.png"), view=AboutView(category))


async def handle_sled(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    sled = player.get("vanityMeta", {}).get("gadgetSledType")
    val  = sled.replace("_", " ").title() if sled else "None"
    cells = [(category.upper(), val, C_CYAN if sled else C_GRAY)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "sled.png"), view=AboutView(category))


async def handle_collectibles(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    count = len(player.get("vanityMeta", {}).get("packages", []))
    cells = [(category.upper(), f"{count}", C_PURPLE)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "collectibles.png"), view=AboutView(category))


async def handle_practiceblocks(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    blocks = (
        player.get("stats", {})
              .get("Bedwars", {})
              .get("practice", {})
              .get("bridging", {})
              .get("blocks_placed", 0)
    )
    cells = [(category.upper(), f"{blocks:,}", C_GREEN)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "practiceblocks.png"), view=AboutView(category))


async def handle_cookies(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    housing_meta = player.get("housingMeta", {})
    total = sum(
        len(v)
        for k, v in housing_meta.items()
        if k.startswith("given_cookies_") and isinstance(v, list)
    )
    cells = [(category.upper(), f"{total:,}", C_GOLD)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "cookies.png"), view=AboutView(category))


async def handle_tracked(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    count = len(player.get("achievementTracking", []))
    cells = [(category.upper(), f"{count}", C_CYAN)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "tracked.png"), view=AboutView(category))


async def handle_souls(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    paid = player.get("stats", {}).get("SkyWars", {}).get("paid_souls", 0)
    cells = [(category.upper(), f"{paid:,}", C_PURPLE)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "souls.png"), view=AboutView(category))


async def handle_favoritemaps(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    packages = player.get("stats", {}).get("Bedwars", {}).get("packages", [])
    favs = sorted(
        p.replace("favoritemap_", "").replace("_", " ").title()
        for p in packages if p.startswith("favoritemap_")
    )
    if not favs:
        cells = [("Favorite Maps", "None", C_GRAY)]
    else:
        cells = [(f"Map {i+1}", m, C_CYAN) for i, m in enumerate(favs)]
        cells.insert(0, ("Total", f"{len(favs)} maps", C_GOLD))
    buf = build_image(rank_segs, rank_base, name, cells, category, grid=True)
    await interaction.followup.send(file=discord.File(buf, "favoritemaps.png"), view=AboutView(category))


async def handle_ultimate(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    selected = player.get("stats", {}).get("Bedwars", {}).get("selected_ultimate") or "None"
    cells = [(category.upper(), selected, C_RED if selected != "None" else C_GRAY)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "ultimate.png"), view=AboutView(category))


async def handle_housingadvanced(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    state = (
        player.get("housingMeta", {})
              .get("playerSettings", {})
              .get("ADVANCED_VARIABLE_OPERATIONS")
    )
    enabled = state == "BooleanState-true"
    cells = [(category.upper(), "Enabled" if enabled else "Disabled",
              C_GREEN if enabled else C_RED)]
    buf = build_image(rank_segs, rank_base, name, cells, category)
    await interaction.followup.send(file=discord.File(buf, "housingadvanced.png"), view=AboutView(category))


async def handle_level(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    exp          = player.get("networkExp", 0)
    level        = int(((2 * exp + 30625) ** 0.5) / 50 - 2.5)
    multiple_250 = round(exp / 79680000, 2)
    cells = [
        ("Network Level", str(level),         C_GOLD),
        ("x Level 250",   f"{multiple_250}x", C_CYAN),
        ("Network XP",    f"{exp:,}",         C_WHITE),
    ]
    buf = build_image(rank_segs, rank_base, name, cells, category, grid=True)
    await interaction.followup.send(file=discord.File(buf, "level.png"), view=AboutView(category))



async def handle_backdropwins(session, uuid, name, rank_segs, rank_base, interaction, category=""):
    player = await fetch_player(session, uuid)
    if not player:
        await interaction.followup.send(f"No Hypixel data found for `{name}`.")
        return
    backdrop_wins = (
        player.get("stats", {})
              .get("BuildBattle", {})
              .get("backdrop_wins", {})
    )
    if not backdrop_wins:
        await interaction.followup.send(f"No backdrop wins found for `{name}`.")
        return
    cells = [
        (bg.replace("_", " ").title(), f"{wins:,}", C_CYAN)
        for bg, wins in sorted(backdrop_wins.items(), key=lambda x: -x[1])
    ]
    buf = build_image(rank_segs, rank_base, name, cells, category, grid=True)
    await interaction.followup.send(file=discord.File(buf, "backdropwins.png"), view=AboutView(category))

# ---------------------------------------------------------------------------
# DISPATCH TABLE
# ---------------------------------------------------------------------------

CATEGORIES = {
    "bwminion":        handle_bwminion,
    "gamblegeorge":    handle_gamblegeorge,
    "sled":            handle_sled,
    "collectibles":    handle_collectibles,
    "practiceblocks":  handle_practiceblocks,
    "cookies":         handle_cookies,
    "tracked":         handle_tracked,
    "souls":           handle_souls,
    "favoritemaps":    handle_favoritemaps,
    "ultimate":        handle_ultimate,
    "housingadvanced": handle_housingadvanced,
    "level":           handle_level,
    "backdropwins":    handle_backdropwins,
}

CATEGORY_CHOICES = [
    app_commands.Choice(name=k, value=k) for k in CATEGORIES
]

# ---------------------------------------------------------------------------
# COG
# ---------------------------------------------------------------------------

class QuentCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @app_commands.command(name="quent", description="Commandes diverses Hypixel")
    @app_commands.describe(
        category="La catégorie de stat à afficher",
        username="Pseudo Minecraft du joueur",
    )
    @app_commands.choices(category=CATEGORY_CHOICES)
    async def quent(
        self,
        interaction: discord.Interaction,
        category: app_commands.Choice[str],
        username: str,
    ):
        await interaction.response.defer()
        async with aiohttp.ClientSession() as session:
            uuid, name = await fetch_uuid(session, username)
            if not uuid:
                await interaction.followup.send(f"Player not found: `{username}`.")
                return
            player_preview = await fetch_player(session, uuid)
            rank_segs, rank_base = (
                get_rank_display(player_preview) if player_preview
                else ([], (150, 150, 150))
            )
            handler = CATEGORIES.get(category.value)
            if handler is None:
                await interaction.followup.send(f"Unknown category: `{category.value}`.")
                return
            try:
                await handler(session, uuid, name, rank_segs, rank_base, interaction, category.value)
            except Exception as e:
                print(f"[quent/{category.value}] {e}")
                await interaction.followup.send("An error occurred while retrieving the data.")


async def setup(bot):
    await bot.add_cog(QuentCog(bot))
