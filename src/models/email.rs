use crate::errors::*;
use crate::fmt::Write;
use crate::fmt::colors::*;
use diesel;
use diesel::prelude::*;
use crate::models::*;
use std::sync::Arc;
use crate::engine::ctx::State;


#[derive(Identifiable, Queryable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name="emails"]
pub struct Email {
    pub id: i32,
    pub value: String,
    pub unscoped: bool,
    pub valid: Option<bool>,
}

impl Model for Email {
    type ID = str;

    fn to_string(&self) -> String {
        self.value.to_owned()
    }

    fn list(db: &Database) -> Result<Vec<Self>> {
        use crate::schema::emails::dsl::*;

        let results = emails.load::<Self>(db.db())?;

        Ok(results)
    }

    fn filter(db: &Database, filter: &Filter) -> Result<Vec<Self>> {
        use crate::schema::emails::dsl::*;

        let query = emails.filter(filter.sql());
        let results = query.load::<Self>(db.db())?;

        Ok(results)
    }

    fn delete(db: &Database, filter: &Filter) -> Result<usize> {
        use crate::schema::emails::dsl::*;

        diesel::delete(emails.filter(filter.sql()))
            .execute(db.db())
            .map_err(Error::from)
    }

    fn delete_id(db: &Database, my_id: i32) -> Result<usize> {
        use crate::schema::emails::dsl::*;

        diesel::delete(emails.filter(id.eq(my_id)))
            .execute(db.db())
            .map_err(Error::from)
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn value(&self) -> &Self::ID {
        &self.value
    }

    fn by_id(db: &Database, my_id: i32) -> Result<Self> {
        use crate::schema::emails::dsl::*;

        let domain = emails.filter(id.eq(my_id))
            .first::<Self>(db.db())?;

        Ok(domain)
    }

    fn get(db: &Database, query: &Self::ID) -> Result<Self> {
        use crate::schema::emails::dsl::*;

        let email = emails.filter(value.eq(query))
            .first::<Self>(db.db())?;

        Ok(email)
    }

    fn get_opt(db: &Database, query: &Self::ID) -> Result<Option<Self>> {
        use crate::schema::emails::dsl::*;

        let email = emails.filter(value.eq(query))
            .first::<Self>(db.db())
            .optional()?;

        Ok(email)
    }
}

impl Scopable for Email {
    fn scoped(&self) -> bool {
        !self.unscoped
    }

    fn scope(db: &Database, filter: &Filter) -> Result<usize> {
        use crate::schema::emails::dsl::*;

        diesel::update(emails.filter(filter.sql()))
            .set(unscoped.eq(false))
            .execute(db.db())
            .map_err(Error::from)
    }

    fn noscope(db: &Database, filter: &Filter) -> Result<usize> {
        use crate::schema::emails::dsl::*;

        diesel::update(emails.filter(filter.sql()))
            .set(unscoped.eq(true))
            .execute(db.db())
            .map_err(Error::from)
    }
}

impl Email {
    fn breaches(&self, db: &Database) -> Result<Vec<(Breach, Option<String>)>> {
        use std::result;

        let breach_id_pws = BreachEmail::belonging_to(self)
            .select((breach_emails::breach_id, breach_emails::password))
            .load::<(i32, Option<String>)>(db.db())?;

        breach_id_pws.into_iter()
            .map(|(breach_id, password)| {
                breaches::table
                    .filter(breaches::id.eq(breach_id))
                    .first::<Breach>(db.db())
                    .map(|breach| (breach, password))
            })
            .collect::<result::Result<Vec<_>, _>>()
            .map_err(Error::from)
    }
}

pub struct PrintableEmail {
    value: String,
}

impl fmt::Display for PrintableEmail {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        write!(w, "{:?}", self.value)
    }
}

impl Printable<PrintableEmail> for Email {
    fn printable(&self, _db: &Database) -> Result<PrintableEmail> {
        Ok(PrintableEmail {
            value: self.value.to_string(),
        })
    }
}

pub struct BreachWithPassword {
    breach: PrintableBreach,
    password: Option<String>,
}

impl fmt::Display for BreachWithPassword {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        write!(w, "{}", self.breach)?;
        if let Some(password) = &self.password {
            write!(w, " ({:?})", password)?;
        }
        Ok(())
    }
}

pub struct DetailedEmail {
    id: i32,
    value: String,
    breaches: Vec<BreachWithPassword>,
    unscoped: bool,
    valid: Option<bool>,
}

impl DisplayableDetailed for DetailedEmail {
    #[inline]
    fn scoped(&self) -> bool {
        !self.unscoped
    }

    #[inline]
    fn print(&self, w: &mut fmt::DetailFormatter) -> fmt::Result {
        w.id(self.id)?;
        w.debug::<Green, _>(&self.value)?;

        if let Some(valid) = self.valid {
            write!(w, " [")?;
            if valid {
                w.display::<Green, _>("valid")?;
            } else {
                w.display::<Red, _>("invalid")?;
            }
            write!(w, "]")?;
        }

        Ok(())
    }

    #[inline]
    fn children(&self, w: &mut fmt::DetailFormatter) -> fmt::Result {
        for breach in &self.breaches {
            w.child(breach)?;
        }
        Ok(())
    }
}

display_detailed!(DetailedEmail);

impl Detailed for Email {
    type T = DetailedEmail;

    fn detailed(&self, db: &Database) -> Result<Self::T> {
        let breaches = self.breaches(db)?.into_iter()
            .map(|(sd, password)| Ok(BreachWithPassword {
                breach: sd.printable(db)?,
                password,
            }))
            .collect::<Result<_>>()?;

        Ok(DetailedEmail {
            id: self.id,
            value: self.value.to_string(),
            breaches,
            unscoped: self.unscoped,
            valid: self.valid,
        })
    }
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[table_name="emails"]
pub struct NewEmail {
    pub value: String,
    pub valid: Option<bool>,
}

impl InsertableStruct<Email> for NewEmail {
    fn value(&self) -> &str {
        &self.value
    }

    fn insert(&self, db: &Database) -> Result<()> {
        diesel::insert_into(emails::table)
            .values(self)
            .execute(db.db())?;
        Ok(())
    }
}

impl Upsertable<Email> for NewEmail {
    type Update = EmailUpdate;

    fn upsert(self, existing: &Email) -> Self::Update {
        Self::Update {
            id: existing.id,
            valid: Self::upsert_opt(self.valid, &existing.valid),
        }
    }
}

impl Printable<PrintableEmail> for NewEmail {
    fn printable(&self, _db: &Database) -> Result<PrintableEmail> {
        Ok(PrintableEmail {
            value: self.value.to_string(),
        })
    }
}

pub type InsertEmail = NewEmail;

impl LuaInsertToNew for InsertEmail {
    type Target = NewEmail;

    fn try_into_new(self, _state: &Arc<State>) -> Result<NewEmail> {
        Ok(self)
    }
}

#[derive(Identifiable, AsChangeset, Serialize, Deserialize, Debug)]
#[table_name="emails"]
pub struct EmailUpdate {
    pub id: i32,
    pub valid: Option<bool>,
}

impl Upsert for EmailUpdate {
    fn is_dirty(&self) -> bool {
        self.valid.is_some()
    }

    fn generic(self) -> Update {
        Update::Email(self)
    }

    fn apply(&self, db: &Database) -> Result<i32> {
        db.update_email(self)
    }
}

impl Updateable<Email> for EmailUpdate {
    fn changeset(&mut self, existing: &Email) {
        Self::clear_if_equal(&mut self.valid, &existing.valid);
    }

    fn fmt(&self, updates: &mut Vec<String>) {
        Self::push_value(updates, "valid", &self.valid);
    }
}
