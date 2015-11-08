pub enum Type {
    Offer,
    Request
}

impl Type {
    pub fn map<T>(&self, offer: T, request: T) -> T {
        match *self {
            Type::Offer => offer,
            Type::Request => request
        }
    }

    pub fn german_article(&self) -> &'static str {
        self.map("das", "die")
    }

    pub fn german_article_capital(&self) -> &'static str {
        self.map("Das", "Die")
    }

    pub fn german_noun(&self) -> &'static str {
        self.map("Angebot", "Anfrage")
    }

    pub fn german_plural(&self) -> &'static str {
        self.map("Angebote", "Anfragen")
    }

    pub fn table(&self) -> &'static str {
        self.map("offers", "requests")
    }

    pub fn url_part(&self) -> &'static str {
        self.map("biete", "suche")
    }
}
