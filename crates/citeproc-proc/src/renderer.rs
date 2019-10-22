use crate::prelude::*;
use citeproc_io::output::LocalizedQuotes;
use citeproc_io::Name;
use citeproc_io::{Locator, NumericValue, Reference};
use csl::locale::Locale;
use csl::style::{NameLabel, NumericForm, Plural, Style, TextCase};
use csl::terms::{
    GenderedTermSelector, LocatorType, RoleTerm, RoleTermSelector, TermForm, TermFormExtended,
    TextTermSelector,
};
use csl::variables::{NameVariable, NumberVariable, StandardVariable};
use csl::Atom;

#[derive(Clone)]
pub enum GenericContext<'a, O: OutputFormat> {
    Ref(&'a RefContext<'a, O>),
    Cit(&'a CiteContext<'a, O>),
}

#[allow(dead_code)]
impl<O: OutputFormat> GenericContext<'_, O> {
    pub fn locale(&self) -> &Locale {
        match self {
            GenericContext::Cit(ctx) => ctx.locale,
            GenericContext::Ref(ctx) => ctx.locale,
        }
    }
    pub fn style(&self) -> &Style {
        match self {
            GenericContext::Cit(ctx) => ctx.style,
            GenericContext::Ref(ctx) => ctx.style,
        }
    }
    pub fn reference(&self) -> &Reference {
        match self {
            GenericContext::Cit(ctx) => ctx.reference,
            GenericContext::Ref(ctx) => ctx.reference,
        }
    }
    pub fn format(&self) -> &O {
        match self {
            GenericContext::Cit(ctx) => &ctx.format,
            GenericContext::Ref(ctx) => ctx.format,
        }
    }
    pub fn should_add_year_suffix_hook(&self) -> bool {
        match self {
            GenericContext::Cit(ctx) => ctx.style.citation.disambiguate_add_year_suffix,
            GenericContext::Ref(ctx) => ctx.year_suffix,
        }
    }
    pub fn locator_type(&self) -> Option<LocatorType> {
        match self {
            Cit(ctx) => ctx
                .cite
                .locators
                .as_ref()
                .and_then(|ls| ls.single())
                .map(Locator::type_of),
            Ref(ctx) => ctx.locator_type,
        }
    }
    pub fn get_name(&self, var: NameVariable) -> Option<&[Name]> {
        match self {
            Cit(ctx) => ctx.get_name(var),
            Ref(ctx) => ctx.reference.name.get(&var),
        }
        .map(|vec| vec.as_slice())
    }
    fn get_number(&self, var: NumberVariable) -> Option<NumericValue> {
        match self {
            Cit(ctx) => ctx.get_number(var),
            Ref(ctx) => ctx.get_number(var),
        }
    }
}

use crate::choose::CondChecker;
use citeproc_io::DateOrRange;
use csl::style::{CslType, Position};
use csl::variables::{AnyVariable, DateVariable};

impl<'a, O: OutputFormat> CondChecker for GenericContext<'a, O> {
    fn has_variable(&self, var: AnyVariable) -> bool {
        match self {
            Ref(ctx) => <RefContext<'a, O> as CondChecker>::has_variable(ctx, var),
            Cit(ctx) => <CiteContext<'a, O> as CondChecker>::has_variable(ctx, var),
        }
    }
    fn is_numeric(&self, var: AnyVariable) -> bool {
        match self {
            Ref(ctx) => <RefContext<'a, O> as CondChecker>::is_numeric(ctx, var),
            Cit(ctx) => <CiteContext<'a, O> as CondChecker>::is_numeric(ctx, var),
        }
    }
    fn is_disambiguate(&self) -> bool {
        match self {
            Ref(ctx) => <RefContext<'a, O> as CondChecker>::is_disambiguate(ctx),
            Cit(ctx) => <CiteContext<'a, O> as CondChecker>::is_disambiguate(ctx),
        }
    }
    fn csl_type(&self) -> CslType {
        match self {
            Ref(ctx) => <RefContext<'a, O> as CondChecker>::csl_type(ctx),
            Cit(ctx) => <CiteContext<'a, O> as CondChecker>::csl_type(ctx),
        }
    }
    fn locator_type(&self) -> Option<LocatorType> {
        match self {
            Ref(ctx) => <RefContext<'a, O> as CondChecker>::locator_type(ctx),
            Cit(ctx) => <CiteContext<'a, O> as CondChecker>::locator_type(ctx),
        }
    }
    fn get_date(&self, dvar: DateVariable) -> Option<&DateOrRange> {
        match self {
            Ref(ctx) => <RefContext<'a, O> as CondChecker>::get_date(ctx, dvar),
            Cit(ctx) => <CiteContext<'a, O> as CondChecker>::get_date(ctx, dvar),
        }
    }
    fn position(&self) -> Position {
        match self {
            Ref(ctx) => <RefContext<'a, O> as CondChecker>::position(ctx),
            Cit(ctx) => <CiteContext<'a, O> as CondChecker>::position(ctx),
        }
    }
    fn features(&self) -> &csl::version::Features {
        match self {
            Ref(ctx) => <RefContext<'a, O> as CondChecker>::features(ctx),
            Cit(ctx) => <CiteContext<'a, O> as CondChecker>::features(ctx),
        }
    }
}

use GenericContext::*;

pub struct Renderer<'a, O: OutputFormat, Custom: OutputFormat = O> {
    ctx: GenericContext<'a, O>,
    format: &'a Custom,
}

impl<'c, O: OutputFormat> Renderer<'c, O, O> {
    pub fn refr(c: &'c RefContext<'c, O>) -> Self {
        Renderer {
            ctx: GenericContext::Ref(c),
            format: c.format,
        }
    }

    pub fn cite(c: &'c CiteContext<'c, O>) -> Self {
        Renderer {
            ctx: GenericContext::Cit(c),
            format: &c.format,
        }
    }
}

impl<O: OutputFormat, O2: OutputFormat> Renderer<'_, O, O2> {
    pub fn sorting<'c>(ctx: GenericContext<'c, O>, format: &'c O2) -> Renderer<'c, O, O2> {
        Renderer { ctx, format }
    }

    #[inline]
    fn fmt(&self) -> &O2 {
        self.format
    }

    /// The spec is slightly impractical to implement:
    ///
    /// > Number variables rendered within the macro with cs:number and date variables are treated
    /// > the same as when they are called via variable.
    ///
    /// ... bu when it's a macro, you have to produce a string. So we just do an arbitrary amount
    /// of left-padding.
    pub fn number_sort_string(
        &self,
        var: NumberVariable,
        form: NumericForm,
        val: &NumericValue,
        af: &Affixes,
        text_case: TextCase,
    ) -> O2::Build {
        use citeproc_io::NumericToken;
        use crate::number::{roman_lower, roman_representable};
        let fmt = self.fmt();
        match (val, form) {
            (NumericValue::Tokens(_, ts), _) => {
                let mut s = String::new();
                for t in ts {
                    if !s.is_empty() {
                        s.push(',');
                    }
                    if let NumericToken::Num(n) = t {
                        s.push_str(&format!("{:08}", n));
                    }
                }
                let options = IngestOptions {
                    replace_hyphens: false,
                    text_case,
                };
                fmt.affixed_text(s, None, af)
            }
            // TODO: text-case
            _ => fmt.affixed_text(val.as_number(var.should_replace_hyphens()), None, af),
        }
    }

    pub fn number(
        &self,
        var: NumberVariable,
        form: NumericForm,
        val: &NumericValue,
        f: Option<Formatting>,
        af: &Affixes,
        text_case: TextCase,
    ) -> O2::Build {
        use crate::number::{roman_lower, roman_representable};
        let fmt = self.fmt();
        match (val, form) {
            (NumericValue::Tokens(_, ts), NumericForm::Roman) if roman_representable(&val) => {
                let options = IngestOptions {
                    replace_hyphens: var.should_replace_hyphens(),
                    text_case,
                };
                let string = roman_lower(&ts);
                let b = fmt.ingest(&string, options);
                let b = fmt.with_format(b, f);
                fmt.affixed(b, af)
            }
            // TODO: text-case
            _ => fmt.affixed_text(val.as_number(var.should_replace_hyphens()), f, af),
        }
    }

    fn quotes(quo: bool) -> Option<LocalizedQuotes> {
        let q = LocalizedQuotes::Single(Atom::from("'"), Atom::from("'"));
        let quotes = if quo { Some(q) } else { None };
        quotes
    }

    pub fn text_variable(
        &self,
        var: StandardVariable,
        value: &str,
        f: Option<Formatting>,
        af: &Affixes,
        quo: bool,
        text_case: TextCase,
        // sp, tc, disp
    ) -> O2::Build {
        let fmt = self.fmt();
        let quotes = Renderer::<O>::quotes(quo);
        let options = IngestOptions {
            replace_hyphens: match var {
                StandardVariable::Ordinary(v) => v.should_replace_hyphens(),
                StandardVariable::Number(v) => v.should_replace_hyphens(),
            },
            text_case,
        };
        let b = fmt.ingest(value, options);
        let txt = fmt.with_format(b, f);

        let txt = match var {
            StandardVariable::Ordinary(v) => {
                let maybe_link = v.hyperlink(value);
                fmt.hyperlinked(txt, maybe_link)
            }
            StandardVariable::Number(_) => txt,
        };
        fmt.affixed_quoted(txt, &af, quotes.as_ref())
    }

    pub fn text_value(
        &self,
        value: &str,
        f: Option<Formatting>,
        af: &Affixes,
        quo: bool,
        text_case: TextCase,
        // sp, tc, disp
    ) -> Option<O2::Build> {
        if value.len() == 0 {
            return None;
        }
        let fmt = self.fmt();
        let quotes = Renderer::<O>::quotes(quo);
        let b = fmt.ingest(
            value,
            IngestOptions {
                text_case,
                ..Default::default()
            },
        );
        let txt = fmt.with_format(b, f);
        Some(fmt.affixed_quoted(txt, af, quotes.as_ref()))
    }

    pub fn text_term(
        &self,
        term_selector: TextTermSelector,
        plural: bool,
        f: Option<Formatting>,
        af: &Affixes,
        quo: bool,
        // sp, tc, disp
    ) -> Option<O2::Build> {
        let fmt = self.fmt();
        let locale = self.ctx.locale();
        let quotes = Renderer::<O>::quotes(quo);
        locale
            .get_text_term(term_selector, plural)
            .map(|val| fmt.affixed_text_quoted(val.to_owned(), f, af, quotes.as_ref()))
    }

    pub fn name_label(&self, label: &NameLabel, var: NameVariable) -> Option<O2::Build> {
        let NameLabel {
            form,
            formatting,
            plural,
            affixes,
            after_name: _,
            // TODO: strip-periods
            strip_periods: _,
            // TODO: text-case
            text_case: _,
        } = label;
        let fmt = self.fmt();
        let selector = RoleTermSelector::from_name_variable(var, *form);
        let val = self.ctx.get_name(var);
        let len = val.map(|v| v.len()).unwrap_or(0);
        let plural = match (len, plural) {
            (0, Plural::Contextual) => return None,
            (1, Plural::Contextual) => false,
            (_, Plural::Contextual) => true,
            (_, Plural::Always) => true,
            (_, Plural::Never) => false,
        };
        selector.and_then(|sel| {
            self.ctx
                .locale()
                .get_text_term(TextTermSelector::Role(sel), plural)
                .map(|term_text| fmt.affixed_text(term_text.to_owned(), *formatting, affixes))
        })
    }

    pub fn numeric_label(
        &self,
        var: NumberVariable,
        form: TermForm,
        num_val: NumericValue,
        plural: Plural,
        f: Option<Formatting>,
        af: &Affixes,
        text_case: TextCase,
    ) -> Option<O2::Build> {
        let fmt = self.fmt();
        let selector =
            GenderedTermSelector::from_number_variable(&self.ctx.locator_type(), var, form);
        let plural = match (num_val, plural) {
            (ref val, Plural::Contextual) => val.is_multiple(),
            (_, Plural::Always) => true,
            (_, Plural::Never) => false,
        };
        selector.and_then(|sel| {
            let options = IngestOptions {
                text_case,
                ..Default::default()
            };
            self.ctx
                .locale()
                .get_text_term(TextTermSelector::Gendered(sel), plural)
                .map(|val| fmt.affixed(fmt.with_format(fmt.ingest(val, options), f), &af))
        })
    }
}

