// Port of C++ emStocksControlPanel.h / emStocksControlPanel.cpp

use crate::emStocksConfig::{ChartPeriod, Sorting, emStocksConfig};
use crate::emStocksListBox::emStocksListBox;
use crate::emStocksRec::{Interest, PaymentPriceToString, StockRec, emStocksRec};

// ─── FileFieldPanel ──────────────────────────────────────────────────────────

/// Port of C++ emStocksControlPanel::FileFieldType.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileFieldType {
    Script,      // FT_SCRIPT
    Interpreter, // FT_INTERPRETER
    Browser,     // FT_BROWSER
}

/// Port of C++ emStocksControlPanel::FileFieldPanel.
/// DIVERGED: Data model only — actual widget layout and emFileSelectionBox deferred.
/// Label/description are static metadata provided at GUI construction time (not stored here).
pub(crate) struct FileFieldPanel {
    pub(crate) field_type: FileFieldType,
    pub(crate) text_value: String,
    pub(crate) update_controls_needed: bool,
}

impl FileFieldPanel {
    pub(crate) fn new(field_type: FileFieldType) -> Self {
        Self {
            field_type,
            text_value: String::new(),
            update_controls_needed: true,
        }
    }

    /// Port of C++ FileFieldPanel::UpdateControls.
    pub(crate) fn UpdateControls(&mut self, config: &emStocksConfig) {
        self.update_controls_needed = false;
        let value = match self.field_type {
            FileFieldType::Script => &config.api_script,
            FileFieldType::Interpreter => &config.api_script_interpreter,
            FileFieldType::Browser => &config.web_browser,
        };
        self.text_value = value.clone();
    }
}

// ─── CategoryType ────────────────────────────────────────────────────────────

/// Port of C++ emStocksControlPanel::CategoryType.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CategoryType {
    Country,    // CT_COUNTRY
    Sector,     // CT_SECTOR
    Collection, // CT_COLLECTION
}

// ─── ControlCategoryPanel ────────────────────────────────────────────────────

/// Port of C++ emStocksControlPanel::CategoryPanel.
/// DIVERGED: Stub — actual widget creation deferred.
/// This is a different type from emStocksItemPanel::CategoryPanel.
pub struct ControlCategoryPanel {
    pub caption: String,
    pub sorted_items: Vec<String>,
    pub(crate) category_type: CategoryType,
}

impl ControlCategoryPanel {
    pub(crate) fn new(caption: &str, category_type: CategoryType) -> Self {
        Self {
            caption: caption.to_string(),
            sorted_items: Vec::new(),
            category_type,
        }
    }

    /// Returns the extractor function for this panel's category type.
    pub(crate) fn extractor(&self) -> fn(&StockRec) -> &str {
        match self.category_type {
            CategoryType::Country => |s| &s.country,
            CategoryType::Sector => |s| &s.sector,
            CategoryType::Collection => |s| &s.collection,
        }
    }

    /// Port of C++ CategoryPanel::UpdateItems.
    /// Rebuilds the sorted item list from all stocks.
    pub fn UpdateItems(&mut self, stocks: &[StockRec], extract: fn(&StockRec) -> &str) {
        let mut items: Vec<String> = stocks
            .iter()
            .map(|s| extract(s).to_string())
            .filter(|s| !s.is_empty())
            .collect();
        items.sort();
        items.dedup();
        self.sorted_items = items;
    }
}

// ─── ControlWidgets ──────────────────────────────────────────────────────────

/// Port of C++ emStocksControlPanel widget fields.
/// DIVERGED: Data model only — actual GUI widget types (emButton, emTextField,
/// emCheckBox, emScalarField, emRadioButton) are not yet implemented.
/// Fields are Option<T> mirroring C++ NULL-pointer pattern in AutoExpand/AutoShrink.
pub(crate) struct ControlWidgets {
    // Config fields (Preferences group)
    pub(crate) api_script: FileFieldPanel,
    pub(crate) api_script_interpreter: FileFieldPanel,
    pub(crate) api_key: String,
    pub(crate) web_browser: FileFieldPanel,
    pub(crate) auto_update_dates: bool,
    pub(crate) triggering_opens_web_page: bool,
    pub(crate) chart_period: ChartPeriod,
    /// Display text for the current chart period (set via ChartPeriodTextOfValue).
    pub(crate) chart_period_text: &'static str,

    // Filter fields
    pub(crate) min_visible_interest: Interest,
    pub(crate) visible_countries: ControlCategoryPanel,
    pub(crate) visible_sectors: ControlCategoryPanel,
    pub(crate) visible_collections: ControlCategoryPanel,

    // Sorting
    pub(crate) sorting: Sorting,
    pub(crate) owned_shares_first: bool,

    // Prices group — FetchSharePrices, DeleteSharePrices always enabled in C++
    pub(crate) go_back_in_history_enabled: bool,
    pub(crate) go_forward_in_history_enabled: bool,
    pub(crate) selected_date: String,
    pub(crate) total_purchase_value: String,
    pub(crate) total_current_value: String,
    pub(crate) total_difference_value: String,

    // Commands group — NewStock, PasteStocks always enabled in C++
    pub(crate) cut_stocks_enabled: bool,
    pub(crate) copy_stocks_enabled: bool,
    pub(crate) delete_stocks_enabled: bool,
    pub(crate) select_all_enabled: bool,
    pub(crate) clear_selection_enabled: bool,
    pub(crate) set_high_interest_enabled: bool,
    pub(crate) set_medium_interest_enabled: bool,
    pub(crate) set_low_interest_enabled: bool,
    pub(crate) show_first_web_pages_enabled: bool,
    pub(crate) show_all_web_pages_enabled: bool,

    // Search group — FindSelected always enabled in C++
    pub(crate) search_text: String,
    pub(crate) find_next_enabled: bool,
    pub(crate) find_previous_enabled: bool,
}

impl ControlWidgets {
    fn new() -> Self {
        Self {
            api_script: FileFieldPanel::new(FileFieldType::Script),
            api_script_interpreter: FileFieldPanel::new(FileFieldType::Interpreter),
            api_key: String::new(),
            web_browser: FileFieldPanel::new(FileFieldType::Browser),
            auto_update_dates: false,
            triggering_opens_web_page: false,
            chart_period: ChartPeriod::default(),
            chart_period_text: ChartPeriodTextOfValue(ChartPeriod::default()),

            min_visible_interest: Interest::default(),
            visible_countries: ControlCategoryPanel::new("Visible Countries", CategoryType::Country),
            visible_sectors: ControlCategoryPanel::new("Visible Sectors", CategoryType::Sector),
            visible_collections: ControlCategoryPanel::new(
                "Visible Collections",
                CategoryType::Collection,
            ),

            sorting: Sorting::default(),
            owned_shares_first: false,

            go_back_in_history_enabled: false,
            go_forward_in_history_enabled: false,
            selected_date: String::new(),
            total_purchase_value: String::new(),
            total_current_value: String::new(),
            total_difference_value: String::new(),

            cut_stocks_enabled: false,
            copy_stocks_enabled: false,
            delete_stocks_enabled: false,
            select_all_enabled: false,
            clear_selection_enabled: false,
            set_high_interest_enabled: false,
            set_medium_interest_enabled: false,
            set_low_interest_enabled: false,
            show_first_web_pages_enabled: false,
            show_all_web_pages_enabled: false,

            search_text: String::new(),
            find_next_enabled: false,
            find_previous_enabled: false,
        }
    }
}

// ─── ChartPeriodTextOfValue ──────────────────────────────────────────────────

/// Port of C++ emStocksControlPanel::ChartPeriodTextOfValue.
/// Returns the display text for a chart period value.
pub(crate) fn ChartPeriodTextOfValue(period: ChartPeriod) -> &'static str {
    match period {
        ChartPeriod::Week1 => "1\nweek",
        ChartPeriod::Weeks2 => "2\nweeks",
        ChartPeriod::Month1 => "1\nmonth",
        ChartPeriod::Months3 => "3\nmonths",
        ChartPeriod::Months6 => "6\nmonths",
        ChartPeriod::Year1 => "1\nyear",
        ChartPeriod::Years3 => "3\nyears",
        ChartPeriod::Years5 => "5\nyears",
        ChartPeriod::Years10 => "10\nyears",
        ChartPeriod::Years20 => "20\nyears",
    }
}

// ─── ValidateDate ────────────────────────────────────────────────────────────

/// Port of C++ emStocksControlPanel::ValidateDate.
/// Filters a string to contain only digits and at most 2 dashes, max 32 chars.
pub(crate) fn ValidateDate(input: &str) -> String {
    let mut result = String::new();
    let mut dash_count = 0;
    for ch in input.chars() {
        if result.len() >= 32 {
            break;
        }
        if ch.is_ascii_digit() {
            result.push(ch);
        } else if ch == '-' && dash_count < 2 {
            dash_count += 1;
            result.push(ch);
        }
    }
    result
}

// ─── emStocksControlPanel ────────────────────────────────────────────────────

/// Port of C++ emStocksControlPanel.
/// DIVERGED: Data model only — widget layout and signal handling deferred.
/// The `widgets` field mirrors C++ AutoExpand/AutoShrink lifecycle:
/// `None` when shrunk (C++ NULL pointers), `Some` when expanded.
pub struct emStocksControlPanel {
    pub(crate) update_controls_needed: bool,
    pub(crate) widgets: Option<ControlWidgets>,
}

impl emStocksControlPanel {
    pub fn new() -> Self {
        Self {
            update_controls_needed: true,
            widgets: None,
        }
    }

    pub fn NeedsUpdate(&self) -> bool {
        self.update_controls_needed
    }

    pub fn MarkUpdated(&mut self) {
        self.update_controls_needed = false;
    }

    /// Port of C++ AutoExpand.
    /// Creates all widget fields. In C++ these are `new emButton(...)` etc.
    pub fn AutoExpand(&mut self) {
        self.widgets = Some(ControlWidgets::new());
        self.update_controls_needed = true;
    }

    /// Port of C++ AutoShrink.
    /// Destroys all widget fields. In C++ these are set to NULL.
    pub fn AutoShrink(&mut self) {
        self.widgets = None;
    }

    /// Port of C++ IsAutoExpanded.
    pub fn IsAutoExpanded(&self) -> bool {
        self.widgets.is_some()
    }

    /// Port of C++ UpdateControls.
    /// DIVERGED: C++ reads from owned Config/FileModel/ListBox references.
    /// Rust takes explicit parameters since ownership model differs.
    pub fn UpdateControls(
        &mut self,
        config: &emStocksConfig,
        rec: &emStocksRec,
        list_box: &emStocksListBox,
    ) {
        self.update_controls_needed = false;

        let widgets = match self.widgets.as_mut() {
            Some(w) => w,
            None => return,
        };

        // Sync config values to widget state
        widgets.api_script.UpdateControls(config);
        widgets.api_script_interpreter.UpdateControls(config);
        widgets.api_key = config.api_key.clone();
        widgets.web_browser.UpdateControls(config);

        widgets.auto_update_dates = config.auto_update_dates;
        widgets.triggering_opens_web_page = config.triggering_opens_web_page;
        widgets.chart_period = config.chart_period;
        widgets.chart_period_text = ChartPeriodTextOfValue(config.chart_period);

        widgets.min_visible_interest = config.min_visible_interest;

        // Update category panels with current stock data
        let countries_ext = widgets.visible_countries.extractor();
        widgets
            .visible_countries
            .UpdateItems(&rec.stocks, countries_ext);
        let sectors_ext = widgets.visible_sectors.extractor();
        widgets.visible_sectors.UpdateItems(&rec.stocks, sectors_ext);
        let collections_ext = widgets.visible_collections.extractor();
        widgets
            .visible_collections
            .UpdateItems(&rec.stocks, collections_ext);

        widgets.sorting = config.sorting;
        widgets.owned_shares_first = config.owned_shares_first;

        // History navigation enabled state
        widgets.go_back_in_history_enabled =
            !rec.GetPricesDateBefore(list_box.GetSelectedDate()).is_empty();
        widgets.go_forward_in_history_enabled =
            !rec.GetPricesDateAfter(list_box.GetSelectedDate()).is_empty();

        widgets.selected_date = ValidateDate(list_box.GetSelectedDate());

        // Calculate totals from owned visible stocks
        let mut total_purchase = 0.0_f64;
        let mut total_current = 0.0_f64;
        let mut total_purchase_valid = true;
        let mut total_current_valid = true;

        for &stock_idx in &list_box.visible_items {
            if let Some(stock_rec) = rec.stocks.get(stock_idx) {
                if !stock_rec.owning_shares {
                    continue;
                }
                match stock_rec.GetTradeValue() {
                    Some(d) => total_purchase += d,
                    None => total_purchase_valid = false,
                }
                match stock_rec.GetValueOfDate(list_box.GetSelectedDate()) {
                    Some(d) => total_current += d,
                    None => total_current_valid = false,
                }
            }
        }

        widgets.total_purchase_value = if total_purchase_valid {
            PaymentPriceToString(total_purchase)
        } else {
            String::new()
        };

        widgets.total_current_value = if total_current_valid {
            PaymentPriceToString(total_current)
        } else {
            String::new()
        };

        widgets.total_difference_value = if total_purchase_valid && total_current_valid {
            PaymentPriceToString(total_current - total_purchase)
        } else {
            String::new()
        };

        // Enable/disable buttons based on selection
        let selection_count = list_box.GetSelectionCount();
        let has_selection = selection_count > 0;

        widgets.cut_stocks_enabled = has_selection;
        widgets.copy_stocks_enabled = has_selection;
        widgets.delete_stocks_enabled = has_selection;
        widgets.select_all_enabled = selection_count < list_box.visible_items.len();
        widgets.clear_selection_enabled = has_selection;
        widgets.set_high_interest_enabled = has_selection;
        widgets.set_medium_interest_enabled = has_selection;
        widgets.set_low_interest_enabled = has_selection;
        widgets.show_first_web_pages_enabled = has_selection;
        widgets.show_all_web_pages_enabled = has_selection;

        // Search
        widgets.search_text = config.search_text.clone();
        let has_search_text = !config.search_text.is_empty();
        widgets.find_next_enabled = has_search_text;
        widgets.find_previous_enabled = has_search_text;
    }
}

impl Default for emStocksControlPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emStocksRec::StockRec;

    #[test]
    fn control_panel_new() {
        let panel = emStocksControlPanel::new();
        assert!(panel.update_controls_needed);
        assert!(!panel.IsAutoExpanded());
    }

    #[test]
    fn file_field_panel_new() {
        let panel = FileFieldPanel::new(FileFieldType::Script);
        assert_eq!(panel.field_type, FileFieldType::Script);
        assert!(panel.update_controls_needed);
        assert!(panel.text_value.is_empty());
    }

    #[test]
    fn category_panel_update_items() {
        let mut cp = ControlCategoryPanel::new("Countries", CategoryType::Country);
        let mut stocks = vec![StockRec::default(), StockRec::default(), StockRec::default()];
        stocks[0].country = "US".to_string();
        stocks[1].country = "DE".to_string();
        stocks[2].country = "US".to_string(); // duplicate

        cp.UpdateItems(&stocks, |s| &s.country);
        assert_eq!(cp.sorted_items, vec!["DE", "US"]); // sorted, deduplicated
    }

    #[test]
    fn auto_expand_creates_widgets() {
        let mut panel = emStocksControlPanel::new();
        assert!(!panel.IsAutoExpanded());

        panel.AutoExpand();
        assert!(panel.IsAutoExpanded());
        assert!(panel.update_controls_needed);

        let widgets = panel.widgets.as_ref().unwrap();
        assert_eq!(widgets.api_script.field_type, FileFieldType::Script);
        assert_eq!(
            widgets.api_script_interpreter.field_type,
            FileFieldType::Interpreter
        );
        assert_eq!(widgets.web_browser.field_type, FileFieldType::Browser);
        assert_eq!(widgets.chart_period, ChartPeriod::default());
        assert_eq!(widgets.sorting, Sorting::default());
        assert!(!widgets.auto_update_dates);
        assert!(!widgets.triggering_opens_web_page);
        assert!(!widgets.owned_shares_first);
    }

    #[test]
    fn auto_shrink_destroys_widgets() {
        let mut panel = emStocksControlPanel::new();
        panel.AutoExpand();
        assert!(panel.IsAutoExpanded());

        panel.AutoShrink();
        assert!(!panel.IsAutoExpanded());
        assert!(panel.widgets.is_none());
    }

    #[test]
    fn auto_expand_shrink_cycle() {
        let mut panel = emStocksControlPanel::new();

        // First expand
        panel.AutoExpand();
        assert!(panel.IsAutoExpanded());

        // Shrink
        panel.AutoShrink();
        assert!(!panel.IsAutoExpanded());

        // Re-expand
        panel.AutoExpand();
        assert!(panel.IsAutoExpanded());
        assert!(panel.update_controls_needed);
    }

    #[test]
    fn chart_period_text_of_value_all_variants() {
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Week1), "1\nweek");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Weeks2), "2\nweeks");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Month1), "1\nmonth");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Months3), "3\nmonths");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Months6), "6\nmonths");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Year1), "1\nyear");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Years3), "3\nyears");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Years5), "5\nyears");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Years10), "10\nyears");
        assert_eq!(ChartPeriodTextOfValue(ChartPeriod::Years20), "20\nyears");
    }

    #[test]
    fn validate_date_filters_correctly() {
        assert_eq!(ValidateDate("2024-06-15"), "2024-06-15");
        assert_eq!(ValidateDate("abc"), "");
        assert_eq!(ValidateDate("2024--06-15"), "2024--0615"); // only 2 dashes
        assert_eq!(ValidateDate("12-34-56-78"), "12-34-5678"); // third dash dropped
    }

    #[test]
    fn validate_date_length_limit() {
        let long = "1".repeat(50);
        assert_eq!(ValidateDate(&long).len(), 32);
    }

    #[test]
    fn update_controls_syncs_config() {
        let mut panel = emStocksControlPanel::new();
        panel.AutoExpand();

        let config = emStocksConfig {
            api_key: "test-key".to_string(),
            auto_update_dates: true,
            triggering_opens_web_page: true,
            chart_period: ChartPeriod::Months3,
            min_visible_interest: Interest::High,
            sorting: Sorting::ByTradeDate,
            owned_shares_first: true,
            search_text: "find me".to_string(),
            ..Default::default()
        };
        let rec = emStocksRec::default();
        let list_box = emStocksListBox::new();

        panel.UpdateControls(&config, &rec, &list_box);

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.api_key, "test-key");
        assert!(w.auto_update_dates);
        assert!(w.triggering_opens_web_page);
        assert_eq!(w.chart_period, ChartPeriod::Months3);
        assert_eq!(w.min_visible_interest, Interest::High);
        assert_eq!(w.sorting, Sorting::ByTradeDate);
        assert!(w.owned_shares_first);
        assert_eq!(w.search_text, "find me");
        assert!(w.find_next_enabled);
        assert!(w.find_previous_enabled);
        assert!(!panel.update_controls_needed);
    }

    #[test]
    fn update_controls_empty_search_disables_find() {
        let mut panel = emStocksControlPanel::new();
        panel.AutoExpand();

        let config = emStocksConfig::default(); // search_text is empty
        let rec = emStocksRec::default();
        let list_box = emStocksListBox::new();

        panel.UpdateControls(&config, &rec, &list_box);

        let w = panel.widgets.as_ref().unwrap();
        assert!(!w.find_next_enabled);
        assert!(!w.find_previous_enabled);
    }

    #[test]
    fn update_controls_selection_enables_buttons() {
        let mut panel = emStocksControlPanel::new();
        panel.AutoExpand();

        let config = emStocksConfig::default();
        let rec = emStocksRec::default();
        let list_box = emStocksListBox::new();

        // No selection
        panel.UpdateControls(&config, &rec, &list_box);
        let w = panel.widgets.as_ref().unwrap();
        assert!(!w.cut_stocks_enabled);
        assert!(!w.copy_stocks_enabled);
        assert!(!w.delete_stocks_enabled);
        assert!(!w.clear_selection_enabled);
        assert!(!w.set_high_interest_enabled);
        assert!(!w.show_first_web_pages_enabled);
    }

    #[test]
    fn update_controls_with_selection() {
        let mut panel = emStocksControlPanel::new();
        panel.AutoExpand();

        let config = emStocksConfig::default();
        let mut rec = emStocksRec::default();
        rec.stocks.push(StockRec::default());
        rec.stocks.push(StockRec::default());

        let mut list_box = emStocksListBox::new();
        list_box.visible_items = vec![0, 1];
        list_box.Select(0);

        panel.UpdateControls(&config, &rec, &list_box);

        let w = panel.widgets.as_ref().unwrap();
        assert!(w.cut_stocks_enabled);
        assert!(w.copy_stocks_enabled);
        assert!(w.delete_stocks_enabled);
        assert!(w.clear_selection_enabled);
        assert!(w.set_high_interest_enabled);
        assert!(w.set_medium_interest_enabled);
        assert!(w.set_low_interest_enabled);
        assert!(w.show_first_web_pages_enabled);
        assert!(w.show_all_web_pages_enabled);
        // Not all selected, so select_all should be enabled
        assert!(w.select_all_enabled);
    }

    #[test]
    fn update_controls_all_selected_disables_select_all() {
        let mut panel = emStocksControlPanel::new();
        panel.AutoExpand();

        let config = emStocksConfig::default();
        let mut rec = emStocksRec::default();
        rec.stocks.push(StockRec::default());

        let mut list_box = emStocksListBox::new();
        list_box.visible_items = vec![0];
        list_box.Select(0);

        panel.UpdateControls(&config, &rec, &list_box);

        let w = panel.widgets.as_ref().unwrap();
        assert!(!w.select_all_enabled); // all already selected
    }

    #[test]
    fn update_controls_total_values_with_owned_stocks() {
        let mut panel = emStocksControlPanel::new();
        panel.AutoExpand();

        let config = emStocksConfig::default();
        let mut rec = emStocksRec::default();

        // Stock with owned shares: 10 shares at $5 trade price
        let mut stock = StockRec::default();
        stock.owning_shares = true;
        stock.own_shares = "10".to_string();
        stock.trade_price = "5.00".to_string();
        // Need a price for the selected date to compute current value
        stock.last_price_date = "2024-06-15".to_string();
        stock.prices = "A".to_string(); // price byte 'A' = 65-32 = 33 -> 3.30
        rec.stocks.push(stock);

        let mut list_box = emStocksListBox::new();
        list_box.visible_items = vec![0];
        list_box.SetSelectedDate("2024-06-15");

        panel.UpdateControls(&config, &rec, &list_box);

        let w = panel.widgets.as_ref().unwrap();
        // trade_value = 10 * 5.00 = 50.00
        assert_eq!(w.total_purchase_value, "50.00");
    }

    #[test]
    fn update_controls_no_owned_stocks_zeros() {
        let mut panel = emStocksControlPanel::new();
        panel.AutoExpand();

        let config = emStocksConfig::default();
        let mut rec = emStocksRec::default();

        // Stock without owned shares
        let stock = StockRec::default();
        rec.stocks.push(stock);

        let mut list_box = emStocksListBox::new();
        list_box.visible_items = vec![0];

        panel.UpdateControls(&config, &rec, &list_box);

        let w = panel.widgets.as_ref().unwrap();
        // No owned stocks, so totals are valid but 0
        assert_eq!(w.total_purchase_value, "0.00");
        assert_eq!(w.total_current_value, "0.00");
        assert_eq!(w.total_difference_value, "0.00");
    }

    #[test]
    fn update_controls_not_expanded_is_noop() {
        let mut panel = emStocksControlPanel::new();
        // Don't call AutoExpand

        let config = emStocksConfig::default();
        let rec = emStocksRec::default();
        let list_box = emStocksListBox::new();

        panel.UpdateControls(&config, &rec, &list_box);
        // Should not panic, just returns early
        assert!(!panel.update_controls_needed);
        assert!(panel.widgets.is_none());
    }

    #[test]
    fn file_field_panel_update_controls() {
        let config = emStocksConfig {
            api_script: "/path/to/script.pl".to_string(),
            api_script_interpreter: "python3".to_string(),
            web_browser: "chromium".to_string(),
            ..Default::default()
        };

        let mut script = FileFieldPanel::new(FileFieldType::Script);
        script.UpdateControls(&config);
        assert_eq!(script.text_value, "/path/to/script.pl");
        assert!(!script.update_controls_needed);

        let mut interp = FileFieldPanel::new(FileFieldType::Interpreter);
        interp.UpdateControls(&config);
        assert_eq!(interp.text_value, "python3");

        let mut browser = FileFieldPanel::new(FileFieldType::Browser);
        browser.UpdateControls(&config);
        assert_eq!(browser.text_value, "chromium");
    }

    #[test]
    fn category_panel_types() {
        let cp = ControlCategoryPanel::new("Countries", CategoryType::Country);
        assert_eq!(cp.category_type, CategoryType::Country);
        assert_eq!(cp.caption, "Countries");
        assert!(cp.sorted_items.is_empty());
    }

    #[test]
    fn category_panel_empty_strings_filtered() {
        let mut cp = ControlCategoryPanel::new("Sectors", CategoryType::Sector);
        let mut stocks = vec![StockRec::default(), StockRec::default()];
        stocks[0].sector = "Tech".to_string();
        stocks[1].sector = String::new(); // empty — should be filtered

        cp.UpdateItems(&stocks, |s| &s.sector);
        assert_eq!(cp.sorted_items, vec!["Tech"]);
    }
}
