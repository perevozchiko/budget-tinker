use super::*;

impl Budget {
    pub fn fresh_start(&self) -> Result<Self> {
        let mut next = self.clone();
        next.name = format!("{} (Fresh Start)", self.name);
        next.transactions.clear();
        next.register_meta.clear();
        next.allocations.clear();
        next.overspending_rules.clear();
        next.default_overspending_rule = OverspendingRule::AvailableToBudget;
        for account in &self.accounts {
            let balance = self.balance(&account.id, None, false);
            let category = if account.on_budget
                && (balance < 0
                    || (balance == 0
                        && matches!(
                            account.account_type.as_str(),
                            "credit" | "line_of_credit" | "mortgage" | "liability"
                        ))) {
                Some(next.create_initial_debt_category(
                    &account.id,
                    &account.name,
                    &current_month(),
                ))
            } else {
                None
            };
            if balance == 0 {
                continue;
            }
            let (base_amount, snapshot) =
                self.to_base(balance, &account.currency, &current_month())?;
            let mut posting = Transaction {
                operation_fx: None,
                check_number: None,
                flag: None,
                id: id(),
                account: account.id.clone(),
                date: today(),
                payee: "Начальный остаток".into(),
                memo: String::new(),
                amount: balance,
                base_amount,
                snapshot,
                income_month: None,
                category,
                cleared: true,
                reconciled: false,
                transfer: None,
                splits: vec![],
            };
            next.freeze_posting(&mut posting)?;
            next.transactions.push(posting);
        }
        Ok(next)
    }
    pub fn new(base: &str) -> Self {
        let groups = [
            (
                "Ежемесячные счета",
                vec!["Аренда / ипотека", "Электричество", "Интернет", "Телефон"],
            ),
            (
                "Повседневные расходы",
                vec![
                    "Продукты",
                    "Кафе и рестораны",
                    "Транспорт",
                    "Здоровье",
                    "Одежда",
                    "Для дома",
                ],
            ),
            (
                "На чёрный день",
                vec!["Резервный фонд", "Ремонт", "Страхование"],
            ),
            ("Цели накоплений", vec!["Отпуск", "Крупные покупки"]),
            ("Обязательства", vec!["Платежи по кредитам"]),
        ];
        Self {
            version: 1,
            name: "Мой бюджет".into(),
            base: base.into(),
            accounts: vec![],
            categories: groups
                .into_iter()
                .flat_map(|(g, ns)| {
                    ns.into_iter().map(move |n| Category {
                        id: id(),
                        group: g.into(),
                        name: n.into(),
                        hidden: false,
                    })
                })
                .collect(),
            category_groups: vec![],
            transactions: vec![],
            allocations: BTreeMap::new(),
            default_overspending_rule: OverspendingRule::AvailableToBudget,
            overspending_rules: BTreeMap::new(),
            snapshots: vec![],
            monthly_fx: MonthlyFxState::default(),
            scheduled: vec![],
            payee_rules: BTreeMap::new(),
            notes: BTreeMap::new(),
            register_meta: BTreeMap::new(),
        }
    }
}
