//------------------------------------------------------------------------------
// emStocksItemPanel.rs
//
// Port of C++ emStocksItemPanel.h / emStocksItemPanel.cpp
//------------------------------------------------------------------------------
// DIVERGED: Data model with widget data fields — actual widget creation and
// layout deferred until panel framework integration. Widget state is stored
// in ItemWidgets struct instead of C++ widget pointers.

use super::emStocksRec::{Interest, ParseDate, PaymentPriceToString, StockRec};

/// Number of web page slots, matching C++ NUM_WEB_PAGES.
const NUM_WEB_PAGES: usize = 4;

/// Port of C++ emStocksItemPanel::CategoryType.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryType {
    Country,
    Sector,
    Collection,
}

/// Port of C++ emStocksItemPanel::CategoryPanel.
/// DIVERGED: Data model only — actual widget creation deferred until panel framework integration.
pub struct CategoryPanel {
    pub category_type: CategoryType,
    pub preserved_category: String,
    pub update_controls_needed: bool,
    pub have_list_box_content: bool,
}

impl CategoryPanel {
    pub fn new(category_type: CategoryType) -> Self {
        CategoryPanel {
            category_type,
            preserved_category: String::new(),
            update_controls_needed: false,
            have_list_box_content: false,
        }
    }
}

/// DIVERGED: Widget data fields stored as plain values instead of C++ widget pointers.
/// Represents the data that C++ widgets would display.
pub(crate) struct ItemWidgets {
    // NameLabel
    pub(crate) name_label_caption: String,
    pub(crate) name_label_color: (u8, u8, u8, u8),

    // Text fields
    pub(crate) name: String,
    pub(crate) symbol: String,
    pub(crate) wkn: String,
    pub(crate) isin: String,

    // OwningShares checkbox
    pub(crate) owning_shares_checked: bool,

    // OwnShares
    pub(crate) own_shares_text: String,
    pub(crate) own_shares_enabled: bool,

    // TradePrice
    pub(crate) trade_price_caption: String,
    pub(crate) trade_price_description: String,
    pub(crate) trade_price_text: String,

    // TradeDate
    pub(crate) trade_date_caption: String,
    pub(crate) trade_date_description: String,
    pub(crate) trade_date_text: String,

    // UpdateTradeDate button
    pub(crate) update_trade_date_caption: String,
    pub(crate) update_trade_date_description: String,

    // FetchSharePrice button
    pub(crate) fetch_share_price_enabled: bool,

    // Price / PriceDate
    pub(crate) price_text: String,
    pub(crate) price_date_text: String,

    // ExpectedDividend
    pub(crate) expected_dividend_text: String,

    // DesiredPrice
    pub(crate) desired_price_caption: String,
    pub(crate) desired_price_description: String,
    pub(crate) desired_price_text: String,

    // InquiryDate
    pub(crate) inquiry_date_text: String,

    // Interest
    pub(crate) interest_index: Interest,

    // WebPages
    pub(crate) web_pages: [String; NUM_WEB_PAGES],
    pub(crate) show_web_page_enabled: [bool; NUM_WEB_PAGES],
    pub(crate) show_all_web_pages_enabled: bool,

    // Comment
    pub(crate) comment_text: String,

    // Computed values
    pub(crate) trade_value: String,
    pub(crate) current_value: String,
    pub(crate) difference_value: String,
}

impl ItemWidgets {
    fn new() -> Self {
        Self {
            name_label_caption: String::new(),
            name_label_color: (240, 240, 240, 255),
            name: String::new(),
            symbol: String::new(),
            wkn: String::new(),
            isin: String::new(),
            owning_shares_checked: false,
            own_shares_text: String::new(),
            own_shares_enabled: false,
            trade_price_caption: String::new(),
            trade_price_description: String::new(),
            trade_price_text: String::new(),
            trade_date_caption: String::new(),
            trade_date_description: String::new(),
            trade_date_text: String::new(),
            update_trade_date_caption: String::new(),
            update_trade_date_description: String::new(),
            fetch_share_price_enabled: false,
            price_text: String::new(),
            price_date_text: String::new(),
            expected_dividend_text: String::new(),
            desired_price_caption: String::new(),
            desired_price_description: String::new(),
            desired_price_text: String::new(),
            inquiry_date_text: String::new(),
            interest_index: Interest::Medium,
            web_pages: Default::default(),
            show_web_page_enabled: [false; NUM_WEB_PAGES],
            show_all_web_pages_enabled: false,
            comment_text: String::new(),
            trade_value: String::new(),
            current_value: String::new(),
            difference_value: String::new(),
        }
    }
}

impl Default for ItemWidgets {
    fn default() -> Self {
        Self::new()
    }
}

/// Port of C++ emStocksItemPanel.
/// DIVERGED: Data model with widget data fields — widget creation and layout
/// deferred until panel framework integration.
pub struct emStocksItemPanel {
    stock_rec_index: Option<usize>,
    pub(crate) update_controls_needed: bool,

    pub country: CategoryPanel,
    pub sector: CategoryPanel,
    pub collection: CategoryPanel,

    /// DIVERGED: Widget data stored in struct instead of C++ widget pointers.
    pub(crate) widgets: Option<ItemWidgets>,

    // Previous values for OwningShares toggle (C++ PrevOwnShares etc.)
    pub prev_own_shares: String,
    pub prev_purchase_price: String,
    pub prev_purchase_date: String,
    pub prev_sale_price: String,
    pub prev_sale_date: String,
}

impl emStocksItemPanel {
    pub fn new() -> Self {
        emStocksItemPanel {
            stock_rec_index: None,
            update_controls_needed: true,
            country: CategoryPanel::new(CategoryType::Country),
            sector: CategoryPanel::new(CategoryType::Sector),
            collection: CategoryPanel::new(CategoryType::Collection),
            widgets: None,
            prev_own_shares: String::new(),
            prev_purchase_price: String::new(),
            prev_purchase_date: String::new(),
            prev_sale_price: String::new(),
            prev_sale_date: String::new(),
        }
    }

    pub fn GetStockRecIndex(&self) -> Option<usize> {
        self.stock_rec_index
    }

    pub fn SetStockRecIndex(&mut self, index: Option<usize>) {
        if self.stock_rec_index != index {
            self.stock_rec_index = index;
            self.update_controls_needed = true;
        }
    }

    /// Port of C++ UpdateControls (logic only, no widget updates).
    /// Checks if stock data has changed and flags need to update.
    pub fn NeedsUpdate(&self) -> bool {
        self.update_controls_needed
    }

    pub fn MarkUpdated(&mut self) {
        self.update_controls_needed = false;
    }

    /// Port of C++ AutoExpand — creates widget data fields.
    /// DIVERGED: Creates ItemWidgets struct instead of C++ widget tree.
    pub fn AutoExpand(&mut self) {
        if self.widgets.is_none() {
            self.widgets = Some(ItemWidgets::new());
            self.update_controls_needed = true;
        }
    }

    /// Port of C++ AutoShrink — destroys widget data fields.
    pub fn AutoShrink(&mut self) {
        self.widgets = None;
    }

    /// Port of C++ emStocksItemPanel::Cycle OwningShares toggle logic.
    ///
    /// When toggling from not-owning to owning:
    ///   - Restore OwnShares from PrevOwnShares (if currently empty)
    ///   - Save current TradePrice/TradeDate as PrevSalePrice/PrevSaleDate
    ///   - Restore TradePrice/TradeDate from PrevPurchasePrice/PrevPurchaseDate
    ///
    /// When toggling from owning to not-owning:
    ///   - Save OwnShares to PrevOwnShares, clear OwnShares (if not empty)
    ///   - Save current TradePrice/TradeDate as PrevPurchasePrice/PrevPurchaseDate
    ///   - Restore TradePrice/TradeDate from PrevSalePrice/PrevSaleDate
    pub fn ToggleOwningShares(&mut self, stock: &mut StockRec) {
        stock.owning_shares = !stock.owning_shares;
        if stock.owning_shares {
            // Toggled to owning
            if stock.own_shares.is_empty() {
                stock.own_shares = self.prev_own_shares.clone();
                self.prev_sale_price = stock.trade_price.clone();
                self.prev_sale_date = stock.trade_date.clone();
                stock.trade_price = self.prev_purchase_price.clone();
                stock.trade_date = self.prev_purchase_date.clone();
            }
        } else {
            // Toggled to not-owning
            if !stock.own_shares.is_empty() {
                self.prev_own_shares = stock.own_shares.clone();
                stock.own_shares.clear();
                self.prev_purchase_price = stock.trade_price.clone();
                self.prev_purchase_date = stock.trade_date.clone();
                stock.trade_price = self.prev_sale_price.clone();
                stock.trade_date = self.prev_sale_date.clone();
            }
        }
        self.update_controls_needed = true;
    }

    /// Port of C++ emStocksItemPanel::UpdateControls.
    /// Syncs stock record data to widget data fields.
    /// DIVERGED: Takes stock and selected_date as parameters instead of
    /// accessing via C++ widget/model references.
    pub fn UpdateControls(&mut self, stock: &StockRec, selected_date: &str) {
        self.update_controls_needed = false;

        let w = match self.widgets.as_mut() {
            Some(w) => w,
            None => return,
        };

        // NameLabel
        if stock.name.is_empty() {
            w.name_label_caption = "<unnamed>".to_string();
            let alpha = 64;
            if stock.owning_shares {
                w.name_label_color = (240, 255, 160, alpha);
            } else {
                w.name_label_color = (240, 240, 240, alpha);
            }
        } else {
            w.name_label_caption = stock.name.clone();
            let alpha = 255;
            if stock.owning_shares {
                w.name_label_color = (240, 255, 160, alpha);
            } else {
                w.name_label_color = (240, 240, 240, alpha);
            }
        }

        // Text fields
        w.name = stock.name.clone();
        w.symbol = stock.symbol.clone();
        w.wkn = stock.wkn.clone();
        w.isin = stock.isin.clone();

        // OwningShares
        w.owning_shares_checked = stock.owning_shares;

        // OwnShares
        w.own_shares_enabled = stock.owning_shares;
        w.own_shares_text = stock.own_shares.clone();

        // TradePrice
        if stock.owning_shares {
            w.trade_price_caption = "Purchase Price".to_string();
            w.trade_price_description =
                "Here you should enter the share price at which you bought shares of this stock."
                    .to_string();
        } else {
            w.trade_price_caption = "Sale Price".to_string();
            w.trade_price_description =
                "Here you may enter the share price at which you sold shares of this stock."
                    .to_string();
        }
        w.trade_price_text = stock.trade_price.clone();

        // TradeDate
        if stock.owning_shares {
            w.trade_date_caption = "Purchase Date".to_string();
            w.trade_date_description =
                "Here you may enter the date on which you bought the shares.\n\
                 The date must have the form YYYY-MM-DD."
                    .to_string();
        } else {
            w.trade_date_caption = "Sale Date".to_string();
            w.trade_date_description =
                "Here you may enter the date on which you sold shares of this stock.\n\
                 The date must have the form YYYY-MM-DD."
                    .to_string();
        }
        w.trade_date_text = stock.trade_date.clone();

        // UpdateTradeDate button
        if stock.owning_shares {
            w.update_trade_date_caption = "Update Purchase Date".to_string();
            w.update_trade_date_description =
                "Set the purchase date to the current date. Note: In the emStocks\n\
                 Preferences is a check box for automatically updating dates, so that\n\
                 the purchase date is updated whenever the purchase price is modified."
                    .to_string();
        } else {
            w.update_trade_date_caption = "Update Sale Date".to_string();
            w.update_trade_date_description =
                "Set the sale date to the current date. Note: In the emStocks\n\
                 Preferences is a check box for automatically updating dates, so that\n\
                 the sale date is updated whenever the sale price is modified."
                    .to_string();
        }

        // FetchSharePrice
        w.fetch_share_price_enabled = !stock.symbol.is_empty();

        // Price / PriceDate
        w.price_text = stock.GetPriceOfDate(selected_date);
        if w.price_text.is_empty() {
            w.price_date_text.clear();
        } else {
            w.price_date_text = selected_date.to_string();
        }

        // ExpectedDividend
        w.expected_dividend_text = stock.expected_dividend.clone();

        // DesiredPrice
        if stock.owning_shares {
            w.desired_price_caption = "Desired Sale Price".to_string();
            w.desired_price_description =
                "Here you should enter the share price at which you want to sell your\n\
                 shares of this stock."
                    .to_string();
        } else {
            w.desired_price_caption = "Desired Purchase Price".to_string();
            w.desired_price_description =
                "Here you should enter the share price at which you want to purchase\n\
                 shares of this stock."
                    .to_string();
        }
        w.desired_price_text = stock.desired_price.clone();

        // InquiryDate
        w.inquiry_date_text = stock.inquiry_date.clone();

        // Interest
        w.interest_index = stock.interest;

        // WebPages
        for i in 0..NUM_WEB_PAGES {
            if i < stock.web_pages.len() {
                w.web_pages[i] = stock.web_pages[i].clone();
            } else {
                w.web_pages[i].clear();
            }
            w.show_web_page_enabled[i] = !w.web_pages[i].is_empty();
        }
        w.show_all_web_pages_enabled = !stock.web_pages.is_empty();

        // Comment
        w.comment_text = stock.comment.clone();

        // Computed values
        w.trade_value = match stock.GetTradeValue() {
            Some(d) => PaymentPriceToString(d),
            None => String::new(),
        };

        w.current_value = match stock.GetValueOfDate(selected_date) {
            Some(d) => PaymentPriceToString(d),
            None => String::new(),
        };

        w.difference_value = match stock.GetDifferenceValueOfDate(selected_date) {
            Some(d) => PaymentPriceToString(d),
            None => String::new(),
        };
    }

    /// Port of C++ ValidateNumber. Returns true if the string is a valid
    /// decimal number (digits and at most one '.'), or empty.
    pub fn ValidateNumber(s: &str) -> bool {
        let mut dot_seen = false;
        for c in s.chars() {
            if c.is_ascii_digit() {
                continue;
            }
            if c == '.' {
                if dot_seen {
                    return false;
                }
                dot_seen = true;
                continue;
            }
            return false;
        }
        true
    }

    /// Port of C++ ValidateDate. Returns true if the string is a valid
    /// date of the form YYYY-MM-DD (parseable), or empty.
    pub fn ValidateDate(s: &str) -> bool {
        if s.is_empty() {
            return true;
        }
        ParseDate(s).is_some()
    }
}

impl Default for emStocksItemPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_panel_new() {
        let panel = emStocksItemPanel::new();
        assert!(panel.GetStockRecIndex().is_none());
        assert!(panel.update_controls_needed);
    }

    #[test]
    fn validate_number_valid() {
        assert!(emStocksItemPanel::ValidateNumber("123.45"));
        assert!(emStocksItemPanel::ValidateNumber("0"));
        assert!(emStocksItemPanel::ValidateNumber(""));
    }

    #[test]
    fn validate_number_invalid() {
        assert!(!emStocksItemPanel::ValidateNumber("abc"));
        assert!(!emStocksItemPanel::ValidateNumber("12.34.56"));
    }

    #[test]
    fn validate_date_valid() {
        assert!(emStocksItemPanel::ValidateDate("2024-03-15"));
        assert!(emStocksItemPanel::ValidateDate(""));
    }

    #[test]
    fn validate_date_invalid() {
        assert!(!emStocksItemPanel::ValidateDate("not-a-date"));
    }

    #[test]
    fn category_panel_types() {
        let cp = CategoryPanel::new(CategoryType::Country);
        assert_eq!(cp.category_type, CategoryType::Country);
    }

    // ─── AutoExpand / AutoShrink ─────────────────────────────────────────────

    #[test]
    fn auto_expand_creates_widgets() {
        let mut panel = emStocksItemPanel::new();
        assert!(panel.widgets.is_none());
        panel.AutoExpand();
        assert!(panel.widgets.is_some());
        assert!(panel.update_controls_needed);
    }

    #[test]
    fn auto_shrink_destroys_widgets() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        panel.AutoShrink();
        assert!(panel.widgets.is_none());
    }

    #[test]
    fn auto_expand_idempotent() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        panel.update_controls_needed = false;
        panel.AutoExpand();
        // Should not re-create or re-flag
        assert!(!panel.update_controls_needed);
    }

    // ─── ToggleOwningShares ──────────────────────────────────────────────────

    fn make_owning_stock() -> StockRec {
        let mut stock = StockRec::default();
        stock.owning_shares = true;
        stock.own_shares = "100".to_string();
        stock.trade_price = "50.00".to_string();
        stock.trade_date = "2024-01-15".to_string();
        stock
    }

    #[test]
    fn toggle_owning_to_not_owning() {
        let mut panel = emStocksItemPanel::new();
        let mut stock = make_owning_stock();

        panel.ToggleOwningShares(&mut stock);

        // Should be not-owning now
        assert!(!stock.owning_shares);
        // OwnShares saved and cleared
        assert_eq!(panel.prev_own_shares, "100");
        assert!(stock.own_shares.is_empty());
        // Trade fields saved as purchase, restored from (empty) sale
        assert_eq!(panel.prev_purchase_price, "50.00");
        assert_eq!(panel.prev_purchase_date, "2024-01-15");
        assert!(stock.trade_price.is_empty());
        assert!(stock.trade_date.is_empty());
        assert!(panel.update_controls_needed);
    }

    #[test]
    fn toggle_not_owning_to_owning() {
        let mut panel = emStocksItemPanel::new();
        // Pre-populate previous values (simulating earlier toggle)
        panel.prev_own_shares = "100".to_string();
        panel.prev_purchase_price = "50.00".to_string();
        panel.prev_purchase_date = "2024-01-15".to_string();

        let mut stock = StockRec::default();
        stock.owning_shares = false;
        stock.trade_price = "45.00".to_string();
        stock.trade_date = "2024-06-01".to_string();

        panel.ToggleOwningShares(&mut stock);

        assert!(stock.owning_shares);
        // OwnShares restored
        assert_eq!(stock.own_shares, "100");
        // Current trade saved as sale
        assert_eq!(panel.prev_sale_price, "45.00");
        assert_eq!(panel.prev_sale_date, "2024-06-01");
        // Trade restored from purchase
        assert_eq!(stock.trade_price, "50.00");
        assert_eq!(stock.trade_date, "2024-01-15");
    }

    #[test]
    fn toggle_round_trip_preserves_data() {
        let mut panel = emStocksItemPanel::new();
        let mut stock = make_owning_stock();

        // Toggle off
        panel.ToggleOwningShares(&mut stock);
        // Toggle back on
        panel.ToggleOwningShares(&mut stock);

        assert!(stock.owning_shares);
        assert_eq!(stock.own_shares, "100");
        assert_eq!(stock.trade_price, "50.00");
        assert_eq!(stock.trade_date, "2024-01-15");
    }

    #[test]
    fn toggle_to_owning_with_nonempty_own_shares_is_noop_on_fields() {
        // C++ guard: if OwnShares is NOT empty when toggling to owning, skip restore
        let mut panel = emStocksItemPanel::new();
        let mut stock = StockRec::default();
        stock.owning_shares = false;
        stock.own_shares = "50".to_string();
        stock.trade_price = "10.00".to_string();
        stock.trade_date = "2024-03-01".to_string();

        panel.ToggleOwningShares(&mut stock);

        assert!(stock.owning_shares);
        // own_shares was not empty, so no restore happened
        assert_eq!(stock.own_shares, "50");
        assert_eq!(stock.trade_price, "10.00");
        assert_eq!(stock.trade_date, "2024-03-01");
    }

    #[test]
    fn toggle_to_not_owning_with_empty_own_shares_is_noop_on_fields() {
        // C++ guard: if OwnShares IS empty when toggling to not-owning, skip save
        let mut panel = emStocksItemPanel::new();
        let mut stock = StockRec::default();
        stock.owning_shares = true;
        stock.own_shares.clear();
        stock.trade_price = "10.00".to_string();

        panel.ToggleOwningShares(&mut stock);

        assert!(!stock.owning_shares);
        // No save happened because own_shares was already empty
        assert!(panel.prev_own_shares.is_empty());
        assert!(panel.prev_purchase_price.is_empty());
    }

    // ─── UpdateControls ──────────────────────────────────────────────────────

    #[test]
    fn update_controls_without_widgets_is_noop() {
        let mut panel = emStocksItemPanel::new();
        let stock = StockRec::default();
        panel.UpdateControls(&stock, "2024-03-15");
        assert!(!panel.update_controls_needed);
        assert!(panel.widgets.is_none());
    }

    #[test]
    fn update_controls_name_label_owning() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let mut stock = StockRec::default();
        stock.name = "ACME Corp".to_string();
        stock.owning_shares = true;

        panel.UpdateControls(&stock, "");

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.name_label_caption, "ACME Corp");
        assert_eq!(w.name_label_color, (240, 255, 160, 255)); // golden color
    }

    #[test]
    fn update_controls_name_label_not_owning() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let stock = StockRec::default(); // owning_shares = false by default

        panel.UpdateControls(&stock, "");

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.name_label_caption, "<unnamed>");
        assert_eq!(w.name_label_color, (240, 240, 240, 64)); // grey, dimmed
    }

    #[test]
    fn update_controls_trade_captions_owning() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let mut stock = StockRec::default();
        stock.owning_shares = true;

        panel.UpdateControls(&stock, "");

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.trade_price_caption, "Purchase Price");
        assert_eq!(w.trade_date_caption, "Purchase Date");
        assert_eq!(w.update_trade_date_caption, "Update Purchase Date");
        assert_eq!(w.desired_price_caption, "Desired Sale Price");
    }

    #[test]
    fn update_controls_trade_captions_not_owning() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let stock = StockRec::default();

        panel.UpdateControls(&stock, "");

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.trade_price_caption, "Sale Price");
        assert_eq!(w.trade_date_caption, "Sale Date");
        assert_eq!(w.update_trade_date_caption, "Update Sale Date");
        assert_eq!(w.desired_price_caption, "Desired Purchase Price");
    }

    #[test]
    fn update_controls_computed_values_owning() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let mut stock = StockRec::default();
        stock.owning_shares = true;
        stock.own_shares = "10".to_string();
        stock.trade_price = "150.00".to_string();
        // prices are pipe-separated, last entry = last_price_date
        stock.last_price_date = "2024-03-15".to_string();
        stock.prices = "100.50".to_string();

        panel.UpdateControls(&stock, "2024-03-15");

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.trade_value, "1500.00");
        assert_eq!(w.current_value, "1005.00");
        assert_eq!(w.difference_value, "-495.00");
    }

    #[test]
    fn update_controls_computed_values_not_owning() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let stock = StockRec::default();

        panel.UpdateControls(&stock, "2024-03-15");

        let w = panel.widgets.as_ref().unwrap();
        assert!(w.trade_value.is_empty());
        assert!(w.current_value.is_empty());
        assert!(w.difference_value.is_empty());
    }

    #[test]
    fn update_controls_text_fields() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let mut stock = StockRec::default();
        stock.name = "Test Stock".to_string();
        stock.symbol = "TST".to_string();
        stock.wkn = "123456".to_string();
        stock.isin = "US1234567890".to_string();
        stock.own_shares = "50".to_string();
        stock.trade_price = "25.00".to_string();
        stock.trade_date = "2024-01-01".to_string();
        stock.expected_dividend = "2.50".to_string();
        stock.desired_price = "30.00".to_string();
        stock.inquiry_date = "2024-02-01".to_string();
        stock.interest = Interest::High;
        stock.comment = "Good stock".to_string();

        panel.UpdateControls(&stock, "");

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.name, "Test Stock");
        assert_eq!(w.symbol, "TST");
        assert_eq!(w.wkn, "123456");
        assert_eq!(w.isin, "US1234567890");
        assert_eq!(w.own_shares_text, "50");
        assert_eq!(w.trade_price_text, "25.00");
        assert_eq!(w.trade_date_text, "2024-01-01");
        assert_eq!(w.expected_dividend_text, "2.50");
        assert_eq!(w.desired_price_text, "30.00");
        assert_eq!(w.inquiry_date_text, "2024-02-01");
        assert_eq!(w.interest_index, Interest::High);
        assert_eq!(w.comment_text, "Good stock");
    }

    #[test]
    fn update_controls_web_pages() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let mut stock = StockRec::default();
        stock.web_pages = vec![
            "http://example.com".to_string(),
            "http://test.com".to_string(),
        ];

        panel.UpdateControls(&stock, "");

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.web_pages[0], "http://example.com");
        assert_eq!(w.web_pages[1], "http://test.com");
        assert!(w.web_pages[2].is_empty());
        assert!(w.web_pages[3].is_empty());
        assert!(w.show_web_page_enabled[0]);
        assert!(w.show_web_page_enabled[1]);
        assert!(!w.show_web_page_enabled[2]);
        assert!(!w.show_web_page_enabled[3]);
        assert!(w.show_all_web_pages_enabled);
    }

    #[test]
    fn update_controls_fetch_enabled_with_symbol() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let mut stock = StockRec::default();
        stock.symbol = "TST".to_string();

        panel.UpdateControls(&stock, "");

        let w = panel.widgets.as_ref().unwrap();
        assert!(w.fetch_share_price_enabled);
    }

    #[test]
    fn update_controls_fetch_disabled_without_symbol() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let stock = StockRec::default();

        panel.UpdateControls(&stock, "");

        let w = panel.widgets.as_ref().unwrap();
        assert!(!w.fetch_share_price_enabled);
    }

    #[test]
    fn update_controls_price_and_price_date() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let mut stock = StockRec::default();
        stock.last_price_date = "2024-03-15".to_string();
        stock.prices = "100.50".to_string();

        panel.UpdateControls(&stock, "2024-03-15");

        let w = panel.widgets.as_ref().unwrap();
        assert_eq!(w.price_text, "100.50");
        assert_eq!(w.price_date_text, "2024-03-15");
    }

    #[test]
    fn update_controls_empty_price_clears_date() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        let stock = StockRec::default();

        panel.UpdateControls(&stock, "2024-03-15");

        let w = panel.widgets.as_ref().unwrap();
        assert!(w.price_text.is_empty());
        assert!(w.price_date_text.is_empty());
    }

    #[test]
    fn update_controls_clears_flag() {
        let mut panel = emStocksItemPanel::new();
        panel.AutoExpand();
        assert!(panel.update_controls_needed);

        let stock = StockRec::default();
        panel.UpdateControls(&stock, "");
        assert!(!panel.update_controls_needed);
    }
}
