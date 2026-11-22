//! topbar: settings (split out of main.rs; shares its items through `super::*`).
use super::*;

/// The gear's menu: a short main page that leads to pages for the theme, the colours, the templates and the bar itself
/// (the look is saved for the other WinRust programs too). Picking on a page keeps it open, so choices can be tried in a row.
pub(crate) unsafe fn style_menu() {
    let items = vec![
        fly::MItem::head(t!("Внешний вид", "Look")),
        fly::MItem::new(t!("Тема", "Theme")).hint(style_name(style())).swatch(i18n::style::pal().accent),
        fly::MItem::new(t!("Цвета и прозрачность", "Colors and transparency")).hint("›"),
        fly::MItem::new(t!("Шаблоны", "Templates")).hint("›"),
        fly::MItem::head(t!("Панель", "Bar")).sep(),
        fly::MItem::new(t!("Положение, размер, поведение", "Position, size, behavior")).hint("›"),
        fly::MItem::new(t!("Разложить окна плиткой", "Tile the windows")),
        fly::MItem::new(t!("Сбросить всё по умолчанию…", "Reset everything to defaults…")).danger().sep(),
    ];
    gear_page("\u{E713}", t!("Настройки панели", "Bar settings"), t!("Вид панели и программ WinRust", "Look of the bar and the WinRust programs"), items, |i| match i {
        1 => theme_page(),
        2 => colors_page(),
        3 => templates_page(),
        5 => bar_page(),
        6 => tile_windows(),
        7 => reset_page(),
        _ => {}
    });
}

/// One page of the gear's menu, opened at the gear.
pub(crate) unsafe fn gear_page(glyph: &'static str, title: &str, sub: &str, items: Vec<fly::MItem>, pick: impl FnOnce(usize) + 'static) {
    let bar = BAR.load(Relaxed) as HWND;
    fly::menu(tip_cx(&bar_layout(bar), GEAR), Some((null_mut(), glyph, title.to_string(), sub.to_string())), items, pick);
}

pub(crate) fn back() -> fly::MItem {
    fly::MItem::new(t!("‹   Назад", "‹   Back"))
}

pub(crate) fn style_name(s: i18n::style::Style) -> &'static str {
    use i18n::style::Style::*;
    match s {
        Normal => t!("Обычный", "Normal"),
        Terminal => t!("Терминал", "Terminal"),
        Dock => t!("Плавающий док", "Floating dock"),
        Tiling => t!("Тайлинг", "Tiling"),
        Road => t!("Белая дорога", "White road"),
        Ruby => t!("Рубин", "Ruby"),
    }
}

/// Repaints the bar in the new look; `moved` = its place or size may have changed too.
pub(crate) unsafe fn relook(moved: bool) {
    let bar = BAR.load(Relaxed) as HWND;
    place(bar);
    if moved {
        bar_anim(bar);
    }
    apply_opacity(bar);
    InvalidateRect(bar, null(), 0);
}

pub(crate) unsafe fn theme_page() {
    let mut items = vec![back()];
    for (k, &s) in i18n::style::ALL.iter().enumerate() {
        let it = fly::MItem::new(style_name(s)).swatch(i18n::style::PALS[k].accent).on(s == style());
        items.push(if k == 0 { it.sep() } else { it });
    }
    gear_page("\u{E790}", t!("Тема", "Theme"), t!("Готовый вид панели и окон", "A ready-made look for the bar and windows"), items, |i| {
        let Some(&s) = i.checked_sub(1).and_then(|k| i18n::style::ALL.get(k)) else { return style_menu() };
        i18n::style::set_style(s);
        relook(false);
        theme_page();
    });
}

pub(crate) unsafe fn bar_page() {
    use i18n::style::{self as st, Side};
    let sides = [Side::Bottom, Side::Left, Side::Right];
    let items = vec![
        back(),
        fly::MItem::head(t!("Положение", "Position")).sep(),
        fly::MItem::new(t!("Снизу", "Bottom")).on(st::side() == Side::Bottom),
        fly::MItem::new(t!("Слева", "Left")).on(st::side() == Side::Left),
        fly::MItem::new(t!("Справа", "Right")).on(st::side() == Side::Right),
        fly::MItem::head(t!("Размер", "Size")),
        fly::MItem::new(t!("Маленькая", "Small")).on(st::bar_size() == 0),
        fly::MItem::new(t!("Обычная", "Normal")).on(st::bar_size() == 1),
        fly::MItem::new(t!("Большая", "Large")).on(st::bar_size() == 2),
        fly::MItem::head(t!("Поведение", "Behavior")),
        fly::MItem::new(t!("Скрывать автоматически", "Hide automatically")).on(st::autohide()),
        fly::MItem::new(t!("Кнопки слева, как в Windows 10", "Buttons on the left, like Windows 10")).on(st::align_left()),
        fly::MItem::new(t!("Alt+Tab: только этот монитор", "Alt+Tab: this monitor only")).on(st::alttab_per_monitor()),
    ];
    gear_page("\u{E8A1}", t!("Панель", "Bar"), t!("Где она, какого размера и как себя ведёт", "Where it sits, its size and behavior"), items, move |i| {
        match i {
            2..=4 => st::set_side(sides[i - 2]),
            6..=8 => st::set_bar_size((i - 6) as u8),
            10 => st::set_autohide(!st::autohide()), // the reserved strip comes or goes with it
            11 => st::set_align_left(!st::align_left()),
            12 => st::set_alttab_per_monitor(!st::alttab_per_monitor()),
            _ => return style_menu(),
        }
        relook(i != 12);
        bar_page();
    });
}

/// Bar transparency from the colours page (the window is layered; 100 % = opaque).
pub(crate) unsafe fn apply_opacity(hwnd: HWND) {
    SetLayeredWindowAttributes(hwnd, 0, (255 * i18n::style::bar_opacity() as u32 / 100) as u8, LWA_ALPHA);
}

/// The user's own look: every colour of the bar and the WinRust windows, and the bar's transparency. Each colour opens
/// the Windows colour dialog; the last row goes back to the chosen theme's colours.
pub(crate) unsafe fn colors_page() {
    use windows_sys::Win32::UI::Controls::Dialogs::{CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, ChooseColorW};
    const PCT: [u8; 5] = [100, 85, 70, 55, 40];
    let colors = [
        t!("Фон", "Background"),
        t!("Линии и рамки", "Lines and borders"),
        t!("Наведение", "Hover"),
        t!("Нажатие и выбранное", "Pressed and selected"),
        t!("Акцент", "Accent"),
        t!("Текст", "Text"),
        t!("Второстепенный текст", "Secondary text"),
    ];
    let cur = i18n::style::custom_colors().unwrap_or([u32::MAX; 7]);
    let p = i18n::style::pal();
    let now = [p.bg, p.line, p.hover, p.active, p.accent, p.text, p.dim]; // what is shown now: the dialog starts there
    let mut items = vec![back(), fly::MItem::head(t!("Цвета (точка справа: изменён вами)", "Colors (dot on the right: changed by you)")).sep()];
    items.extend(colors.iter().enumerate().map(|(i, n)| fly::MItem::new(*n).swatch(now[i]).on(cur[i] != u32::MAX)));
    items.push(fly::MItem::head(t!("Непрозрачность панели", "Bar opacity")));
    items.extend(PCT.iter().map(|&v| fly::MItem::new(if v == 100 { t!("100 %, без прозрачности", "100 %, solid").to_string() } else { format!("{v} %") }).on(i18n::style::bar_opacity() == v)));
    items.push(fly::MItem::new(t!("Вернуть цвета темы", "Back to the theme's colors")).danger().sep());
    let (c0, o0) = (2, 3 + colors.len()); // first colour row, first opacity row
    gear_page("\u{E790}", t!("Цвета и прозрачность", "Colors and transparency"), t!("Поверх выбранной темы", "On top of the chosen theme"), items, move |i| {
        let bar = BAR.load(Relaxed) as HWND;
        if i == 0 {
            return style_menu();
        } else if (c0..c0 + colors.len()).contains(&i) {
            static CUST: std::sync::Mutex<[u32; 16]> = std::sync::Mutex::new([0xFFFFFF; 16]);
            let mut cust = CUST.lock().unwrap();
            let mut cc: CHOOSECOLORW = zeroed();
            cc.lStructSize = size_of::<CHOOSECOLORW>() as u32;
            cc.hwndOwner = bar;
            cc.rgbResult = now[i - c0];
            cc.lpCustColors = cust.as_mut_ptr();
            cc.Flags = CC_RGBINIT | CC_FULLOPEN;
            if ChooseColorW(&mut cc) != 0 {
                i18n::style::set_custom_color(i - c0, Some(cc.rgbResult));
            }
        } else if let Some(&v) = i.checked_sub(o0).and_then(|k| PCT.get(k)) {
            i18n::style::set_bar_opacity(v);
        } else {
            i18n::style::reset_custom();
        }
        apply_opacity(bar);
        InvalidateRect(bar, null(), 0);
        colors_page();
    });
}

/// Saved looks (theme + colours + opacity): apply one, save what is on screen, delete.
pub(crate) unsafe fn templates_page() {
    use i18n::style::{ALL, PALS, Theme};
    let list = i18n::style::themes();
    let now = Theme::current("");
    let mut items = vec![back(), fly::MItem::head(t!("Мои шаблоны", "My templates")).sep()];
    if list.is_empty() {
        items.push(fly::MItem::head(t!("Пока пусто: настройте вид и сохраните", "None yet: set up a look and save it")));
    }
    let first = items.len(); // index of the first template row
    for th in &list {
        let accent = if th.colors[4] != u32::MAX { th.colors[4] } else { PALS[ALL.iter().position(|&s| s == th.style).unwrap_or(0)].accent };
        let same = (th.style, th.colors, th.opacity) == (now.style, now.colors, now.opacity);
        items.push(fly::MItem::new(th.name.clone()).hint(style_name(th.style)).swatch(accent).on(same));
    }
    let save = items.len();
    items.push(fly::MItem::new(t!("Сохранить текущий вид как шаблон", "Save the current look as a template")).sep());
    if !list.is_empty() {
        items.push(fly::MItem::new(t!("Удалить шаблон…", "Delete a template…")).danger());
    }
    gear_page("\u{E8F1}", t!("Шаблоны", "Templates"), t!("Свои наборы на любое настроение", "Your own looks for any mood"), items, move |i| {
        let mut list = list;
        if i == 0 {
            return style_menu();
        } else if (first..save).contains(&i) {
            list[i - first].apply();
            relook(false);
        } else if i == save {
            let name = |n: usize| tf!("Шаблон {}", "Template {}", n);
            let n = (1..).find(|&n| !list.iter().any(|t| t.name == name(n))).unwrap_or(1);
            list.push(Theme::current(&name(n)));
            i18n::style::save_themes(&list);
        } else {
            return delete_page();
        }
        templates_page();
    });
}

pub(crate) unsafe fn delete_page() {
    let list = i18n::style::themes();
    let mut items = vec![back()];
    for (k, th) in list.iter().enumerate() {
        let it = fly::MItem::new(th.name.clone()).hint(style_name(th.style)).danger();
        items.push(if k == 0 { it.sep() } else { it });
    }
    gear_page("\u{E74D}", t!("Удалить шаблон", "Delete a template"), t!("Нажмите на шаблон, чтобы удалить", "Click a template to delete it"), items, move |i| {
        let mut list = list;
        if i == 0 || i > list.len() {
            return templates_page();
        }
        list.remove(i - 1);
        i18n::style::save_themes(&list);
        if list.is_empty() { templates_page() } else { delete_page() }
    });
}

pub(crate) unsafe fn reset_page() {
    let items = vec![fly::MItem::new(t!("Да, сбросить", "Yes, reset")).danger(), fly::MItem::new(t!("Отмена", "Cancel"))];
    gear_page("\u{E777}", t!("Сбросить всё?", "Reset everything?"), t!("Тема, цвета, панель. Шаблоны останутся", "Theme, colors, bar. Templates stay"), items, |i| {
        if i == 0 {
            i18n::style::reset_all();
            relook(true);
        }
        style_menu();
    });
}
